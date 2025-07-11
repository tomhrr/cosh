extern crate cosh;
extern crate ctrlc;
extern crate dirs;
extern crate getopts;
extern crate nix;
extern crate regex;
extern crate rustyline;
extern crate tempfile;

use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use nix::fcntl::{Flock, FlockArg};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::io::{BufRead, BufReader, Cursor};
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

use cpu_time::ProcessTime;
use getopts::Options;
use regex::Regex;
use rustyline::config::OutputStreamType;
use rustyline::error::ReadlineError;
use rustyline::{
    At, Cmd, CompletionType, Config, EditMode, Editor, KeyPress, Movement, Word,
};


use cosh::chunk::{Chunk, new_string_value};
use cosh::compiler::Compiler;
use cosh::vm::VM;
use cosh::rl::{RLHelper, ShellCompleter};

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] [file] [args]", program);
    print!("{}", opts.usage(&brief));
}

fn import_cosh_conf(vm: &mut VM, global_functions: Rc<RefCell<HashMap<String, Rc<RefCell<Chunk>>>>>) {
    let home_opt   = dirs::home_dir();
    let config_opt = dirs::config_dir();
    match (home_opt, config_opt) {
        (Some(home), Some(config)) => {
            let old_path = format!("{}/.coshrc", home.into_os_string().into_string().unwrap());
            let new_path = format!("{}/cosh.conf", config.into_os_string().into_string().unwrap());
            let has_old_path = Path::new(&old_path).exists();
            let has_new_path = Path::new(&new_path).exists();
            if has_old_path && !has_new_path {
                eprintln!("error: please move '{}' to new expected location '{}'",
                          old_path, new_path);
                std::process::exit(1);
            }
            if has_new_path {
                let file_res = fs::File::open(new_path);
                if let Ok(file) = file_res {
                    let mut bufread: Box<dyn BufRead> = Box::new(BufReader::new(file));
                    let chunk_opt = vm.interpret(&mut bufread, "cosh.conf");
                    if let Some(chunk) = chunk_opt {
                        for (k, v) in chunk.borrow().functions.iter() {
                            if !k.starts_with("anon") {
                                global_functions.borrow_mut().insert(k.clone(), v.clone());
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut program = args[0].clone();
    let mut program_done = false;
    let md_opt = fs::metadata(&program);
    if let Ok(md) = md_opt {
        if md.is_file() {
            program_done = true;
        }
    }
    if !program_done {
        program = format!("/proc/{}/exe", std::process::id());
    }

    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");
    opts.optflag("", "bytecode", "input file is bytecode (for compilation)");
    opts.optflag("c", "compile", "compile to bytecode");
    opts.optflag("", "disassemble", "disassemble from bytecode");
    opts.optflag("", "no-rt", "run without loading runtime");
    opts.optflag("", "no-cosh-conf", "run without loading cosh.conf");
    opts.optflag("d", "debug", "show debug information");
    opts.optopt("e", "", "run single expression", "EXPR");
    opts.optopt("o", "", "set output file name for compilation", "NAME");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            eprintln!("unable to parse option: {}", f);
            std::process::exit(1);
        }
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }
    if matches.opt_present("disassemble") && matches.opt_present("bytecode") {
        println!("--bytecode and --disassemble options are mutually exclusive.");
        print_usage(&program, opts);
        return;
    }

    let libdir_opt: Option<&'static str> = option_env!("libdir");
    let libdir =
        match libdir_opt {
            Some(s) => s,
            None    => "/usr/local/lib"
        };
    let rt_chc = format!("{}/cosh/rt.chc", libdir);

    let debug = matches.opt_present("debug");
    let expr_opt = matches.opt_str("e");

    if !matches.free.is_empty() {
        /* A path has been provided, so the user is attempting to run
         * non-interactively, for compilation/disassembly or similar.
         * */
        let path = &matches.free[0];
        if matches.opt_present("disassemble") {
            let mut compiler = Compiler::new();
            let chunk_opt = compiler.deserialise(path);
            if chunk_opt.is_none() {
                eprintln!("unable to deserialise file");
                std::process::exit(1);
            }
            let chunk = chunk_opt.unwrap();
            chunk.disassemble(path);
        } else if matches.opt_present("bytecode") {
            let mut compiler = Compiler::new();
            let chunk_opt = compiler.deserialise(path);
            if chunk_opt.is_none() {
                eprintln!("unable to deserialise file");
                std::process::exit(1);
            }
            let chunk = Rc::new(RefCell::new(chunk_opt.unwrap()));
            let mut vm = VM::new(true, debug, Rc::new(RefCell::new(HashMap::new())),
                                 Rc::new(RefCell::new(HashMap::new())), libdir);
            let mut functions = Vec::new();
            if !matches.opt_present("no-rt") {
                let mut rtchunk_opt = compiler.deserialise(&rt_chc);
                if rtchunk_opt.is_none() {
                    rtchunk_opt = compiler.deserialise("./rt.chc");
                    if rtchunk_opt.is_none() {
                        eprintln!("unable to deserialise runtime library");
                        std::process::exit(1);
                    }
                }
                functions.push(Rc::new(RefCell::new(rtchunk_opt.unwrap())));
            }
            if !functions.is_empty() {
                vm.call_stack_chunks.push((functions[0].clone(), 0));
            }
            vm.run(chunk);
        } else {
            let file_res = fs::File::open(path);
            match file_res {
                Ok(_) => {}
                Err(e) => {
                    let err_str = format!("unable to open file: {}", e);
                    eprintln!("{}", err_str);
                    std::process::exit(1);
                }
            }
            let file = file_res.unwrap();
            let mut bufread: Box<dyn BufRead> = Box::new(BufReader::new(file));
            if matches.opt_present("c") {
                let mut compiler = Compiler::new();
                let re_pre = Regex::new(r#".*/"#).unwrap();
                let path1 = re_pre.replace_all(path, "");
                let re_post = Regex::new(r#"\..*"#).unwrap();
                let name = re_post.replace_all(&path1, "");

                let res = compiler.compile(&mut bufread, &name);
                if let Some(chunk) = res {
                    let output_path_opt = matches.opt_str("o");
                    if output_path_opt.is_none() {
                        eprintln!("output path is required for compilation");
                        std::process::exit(1);
                    }
                    let output_path = output_path_opt.unwrap();
                    let mut res = true;
                    let mut err_str = "".to_owned();
                    {
                        let file_res = fs::File::create(output_path.clone());
                        match file_res {
                            Ok(mut file) => {
                                compiler.serialise(&chunk, &mut file);
                            }
                            Err(e) => {
                                res = false;
                                err_str = e.to_string();
                            }
                        }
                    }
                    if !res {
                        eprintln!("unable to write to path {}: {}", output_path, err_str);
                    }
                }
            } else {
                let mut vm = VM::new(true, debug, Rc::new(RefCell::new(HashMap::new())),
                                     Rc::new(RefCell::new(HashMap::new())), libdir);
                let global_functions = Rc::new(RefCell::new(HashMap::new()));

                if !matches.opt_present("no-rt") {
                    vm.stack.push(new_string_value("rt".to_string()));
                    let res = vm.opcode_import();
                    if res == 0 {
                        std::process::exit(1);
                    }
                }
                if !matches.opt_present("no-cosh-conf") {
                    import_cosh_conf(&mut vm, global_functions.clone());
                }
                for arg in &matches.free[1..] {
                    vm.stack.push(new_string_value(arg.to_string()));
                }

                vm.interpret(&mut bufread, "(main)");
            }
        }
    } else if !expr_opt.is_none() {
        let expr = expr_opt.unwrap();
        
        let global_functions = Rc::new(RefCell::new(HashMap::new()));
        let global_vars = Rc::new(RefCell::new(HashMap::new()));
        let mut vm = VM::new(true, debug, global_functions.clone(),
                             global_vars.clone(), libdir);

        if !matches.opt_present("no-rt") {
            vm.stack.push(new_string_value("rt".to_string()));
            let res = vm.opcode_import();
            if res == 0 {
                std::process::exit(1);
            }
        }
        let running_clone = vm.running.clone();
        ctrlc::set_handler(move || {
            running_clone.store(false, Ordering::SeqCst);
        })
        .unwrap();
        if !matches.opt_present("no-cosh-conf") {
            import_cosh_conf(&mut vm, global_functions.clone());
        }
        let mut bufread: Box<dyn BufRead> = Box::new(Cursor::new(expr.into_bytes()));
        vm.interpret(&mut bufread, "(main)");
    } else {
        /* A path has not been provided, so start the shell. */
        let global_functions = Rc::new(RefCell::new(HashMap::new()));
        let global_vars = Rc::new(RefCell::new(HashMap::new()));
        let mut vm = VM::new(true, debug, global_functions.clone(),
                             global_vars.clone(), libdir);

        if !matches.opt_present("no-rt") {
            vm.stack.push(new_string_value("rt".to_string()));
            let res = vm.opcode_import();
            if res == 0 {
                std::process::exit(1);
            }
        }

        let running_clone = vm.running.clone();
        ctrlc::set_handler(move || {
            running_clone.store(false, Ordering::SeqCst);
        })
        .unwrap();

        if !matches.opt_present("no-cosh-conf") {
            import_cosh_conf(&mut vm, global_functions.clone());
        }

        let config = Config::builder()
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .output_stream(OutputStreamType::Stdout)
            .build();

        let helper = RLHelper {
            completer: ShellCompleter::new(global_functions.clone(), global_vars),
        };

        let mut rl = Editor::with_config(config);
        rl.bind_sequence(
            KeyPress::ControlLeft,
            Cmd::Move(Movement::BackwardWord(1, Word::Vi)),
        );
        rl.bind_sequence(
            KeyPress::ControlRight,
            Cmd::Move(Movement::ForwardWord(1, At::AfterEnd, Word::Vi)),
        );
        rl.set_helper(Some(helper));

        let homedir_res = std::env::var("HOME");
        let history_path_opt =
            match homedir_res {
                Ok(homedir) => {
                    Some(format!("{}/.cosh_history", homedir))
                }
                _ => {
                    None
                }
            };

        /* There isn't a "no limit" setting in rustyline, so just set
         * it to an arbitrary large number. */
        rl.history_mut().set_max_len(1000000);
        if let Some(history_path) = history_path_opt.clone() {
            if rl.load_history(&history_path).is_err() {}
        }
        let history_start_len = rl.history().len();

        let rl_rr = Rc::new(RefCell::new(rl));
        vm.readline = Some(rl_rr.clone());

        // Keep track of the last known working directory for the prompt
        let mut last_known_cwd = env::current_dir()
            .map(|p| p.as_path().to_str().unwrap_or("/").to_string())
            .unwrap_or_else(|_| "/".to_string());

        loop {
            /* The ctrl-c handler that sets running to false is
             * supposed to be caught by the loop in run_inner in the
             * VM and set to true again, but that doesn't always
             * happen, so set it to true here just in case. */
            vm.running.clone().store(true, Ordering::SeqCst);
            let cwd_res = env::current_dir();
            let cwd_str = match cwd_res {
                Ok(cwd) => {
                    // Update the last known directory when successful
                    let cwd_str = cwd.as_path().to_str().unwrap_or("/").to_string();
                    last_known_cwd = cwd_str.clone();
                    cwd_str
                }
                Err(_) => {
                    // If current directory is not available (e.g., removed),
                    // continue using the last known directory for the prompt
                    last_known_cwd.clone()
                }
            };
            let prompt = format!("{}$ ", cwd_str);

            let readline_res = rl_rr.borrow_mut().readline(&prompt);
            match readline_res {
                Ok(mut line) => {
                    let original_line = line.clone();
                    if line.is_empty() {
                        continue;
                    }
                    let starts_with_space = line.starts_with(' ');
                    if starts_with_space {
                        line = "$".to_owned() + &line;
                    }
                    if line.ends_with("; sudo") || line.ends_with("; sudo;") {
                        let sudo_re = Regex::new(r" sudo;?$").unwrap();
                        line = sudo_re.replace(&line, "").to_string();
                        line = format!("$ sudo {} -e '{}'", program, line);
                    }
                    let timing =
                        if line.ends_with("; time") || line.ends_with("; time;") {
                            let time_re = Regex::new(r" time;?$").unwrap();
                            line = time_re.replace(&line, "").to_string();
                            Some((ProcessTime::try_now().unwrap(),
                                  SystemTime::now()))
                        } else {
                            None
                        };

                    line = line.trim().to_string();
                    
                    let mut bufread: Box<dyn BufRead> = Box::new(Cursor::new(line.into_bytes()));
                    rl_rr.borrow_mut().add_history_entry(original_line.as_str());
                    let chunk_opt = vm.interpret_with_mode(&mut bufread, "(main)", true);
                    match chunk_opt {
                        Some(chunk) => {
                            for (k, v) in chunk.borrow().functions.iter() {
                                if !k.starts_with("anon") {
                                    global_functions.borrow_mut().insert(k.clone(), v.clone());
                                }
                            }
                        }
                        None => {}
                    }
                    if starts_with_space && !vm.stack.is_empty() {
                        vm.stack.pop().unwrap();
                    }

                    match timing {
                        Some((ct, rt_start)) => {
                            let ct_dur = ct.try_elapsed().unwrap();
                            let rt_end = SystemTime::now();

                            let rt_start_dur = rt_start.duration_since(UNIX_EPOCH).unwrap();
                            let rt_end_dur   = rt_end.duration_since(UNIX_EPOCH).unwrap();
                            let rt_dur       = rt_end_dur - rt_start_dur;

                            let rt_min = rt_dur.as_secs() / 60;
                            let rt_sec = rt_dur.as_secs() % 60;
                            let rt_ms  = rt_dur.subsec_millis();

                            let ct_min = ct_dur.as_secs() / 60;
                            let ct_sec = ct_dur.as_secs() % 60;
                            let ct_ms  = ct_dur.subsec_millis();

                            eprintln!("");
                            eprintln!("real    {}m{}.{:03}s", rt_min, rt_sec, rt_ms);
                            eprintln!("cpu     {}m{}.{:03}s", ct_min, ct_sec, ct_ms);
                        }
                        _ => {}
                    }
                }
                Err(ReadlineError::Interrupted) => {}
                Err(ReadlineError::Eof) => break,
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }

        if let Some(history_path) = history_path_opt.clone() {
            let history_end_len = rl_rr.borrow().history().len();
            let mut history_file = OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(&history_path)
                .unwrap();
            Flock::lock(history_file.try_clone().unwrap(),
                        FlockArg::LockExclusive).unwrap();

            for i in history_start_len..history_end_len {
                writeln!(history_file, "{}",
                         rl_rr.borrow().history().get(i).unwrap()).unwrap();
            }
        }
    }
}

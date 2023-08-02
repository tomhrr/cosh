extern crate cosh;
extern crate ctrlc;
extern crate dirs_next;
extern crate getopts;
extern crate nix;
extern crate regex;
extern crate rustyline;
extern crate tempfile;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::os::fd::FromRawFd;
use std::env;
use nix::fcntl::{flock, FlockArg};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::io::{Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use libc::setvbuf;
use std::ptr::null_mut;
use libc_stdhandle::stderr;
use libc::FILE;

use dirs_next::home_dir;
use getopts::Options;
use regex::Regex;
use rustyline::config::OutputStreamType;
use rustyline::error::ReadlineError;
use rustyline::{
    At, Cmd, CompletionType, Config, EditMode, Editor, KeyPress, Movement, Word,
};
use tempfile::tempfile;

use cosh::chunk::Chunk;
use cosh::compiler::Compiler;
use cosh::vm::VM;
use cosh::rl::{RLHelper, ShellCompleter};

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] file", program);
    print!("{}", opts.usage(&brief));
}

fn import_coshrc(vm: &mut VM, global_functions: Rc<RefCell<HashMap<String, Rc<RefCell<Chunk>>>>>) {
    if let Some(home) = home_dir() {
        let coshrc_path = format!("{}/.coshrc", home.into_os_string().into_string().unwrap());
        if Path::new(&coshrc_path).exists() {
            let file_res = fs::File::open(coshrc_path);
            if let Ok(file) = file_res {
                let mut bufread: Box<dyn BufRead> = Box::new(BufReader::new(file));
                let chunk_opt = vm.interpret(global_functions.clone(), &mut bufread, ".coshrc");
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
}

fn main() {
    /*
    pub unsafe extern "C" fn setvbuf(
	stream: *mut FILE,
	buffer: *mut c_char,
	mode: c_int,
	size: size_t
    ) -> c_int
    */
    let n: *mut i8 = std::ptr::null_mut();
    unsafe { setvbuf(stderr(), n, libc::_IOLBF, libc::BUFSIZ as usize); };

    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");
    opts.optflag("", "bytecode", "input file is bytecode (for compilation)");
    opts.optflag("c", "compile", "compile to bytecode");
    opts.optflag("", "disassemble", "disassemble from bytecode");
    opts.optflag("", "no-rt", "run without loading runtime");
    opts.optflag("", "no-coshrc", "run without loading .coshrc");
    opts.optflag("d", "debug", "show debug information");
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
            let mut vm = VM::new(true, debug, Rc::new(RefCell::new(HashMap::new())));
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
                let mut vm = VM::new(true, debug, Rc::new(RefCell::new(HashMap::new())));
                let mut compiler = Compiler::new();
                let global_functions = Rc::new(RefCell::new(HashMap::new()));

                if !matches.opt_present("no-rt") {
                    let mut rtchunk_opt = compiler.deserialise(&rt_chc);
                    if rtchunk_opt.is_none() {
                        rtchunk_opt = compiler.deserialise("./rt.chc");
                        if rtchunk_opt.is_none() {
                            eprintln!("unable to deserialise runtime library");
                            std::process::exit(1);
                        }
                    }
                    let rtchunk = rtchunk_opt.unwrap();
                    for (k, v) in rtchunk.functions.iter() {
                        global_functions.borrow_mut().insert(k.clone(), v.clone());
                    }
                }
                if !matches.opt_present("no-coshrc") {
                    import_coshrc(&mut vm, global_functions.clone());
                }

                vm.interpret(global_functions, &mut bufread, "(main)");
            }
        }
    } else {
        /* A path has not been provided, so start the shell. */
        let mut compiler = Compiler::new();
        let global_functions = Rc::new(RefCell::new(HashMap::new()));

        if !matches.opt_present("no-rt") {
            let mut rtchunk_opt = compiler.deserialise(&rt_chc);
            if rtchunk_opt.is_none() {
                rtchunk_opt = compiler.deserialise("./rt.chc");
                if rtchunk_opt.is_none() {
                    eprintln!("unable to deserialise runtime library");
                    std::process::exit(1);
                }
            }
            let rtchunk = rtchunk_opt.unwrap();
            for (k, v) in rtchunk.functions.iter() {
                global_functions.borrow_mut().insert(k.clone(), v.clone());
            }
        }

        let global_vars = Rc::new(RefCell::new(HashMap::new()));
        let mut vm = VM::new(true, debug, global_vars.clone());

        let running_clone = vm.running.clone();
        ctrlc::set_handler(move || {
            running_clone.store(false, Ordering::SeqCst);
        })
        .unwrap();

        if !matches.opt_present("no-coshrc") {
            import_coshrc(&mut vm, global_functions.clone());
        }

        let config = Config::builder()
            .history_ignore_space(true)
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

        loop {
            /* The ctrl-c handler that sets running to false is
             * supposed to be caught by the loop in run_inner in the
             * VM and set to true again, but that doesn't always
             * happen, so set it to true here just in case. */
            vm.running.clone().store(true, Ordering::SeqCst);
            let cwd_res = env::current_dir();
            match cwd_res {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("unable to get current working directory: {}", e);
                    std::process::exit(1);
                }
            }
            let cwd = cwd_res.unwrap();
            let cwd_str = cwd.as_path().to_str().unwrap();
            let prompt = format!("{}$ ", cwd_str);

            let readline_res = rl_rr.borrow_mut().readline(&prompt);
            match readline_res {
                Ok(mut line) => {
                    if line.is_empty() {
                        continue;
                    }
                    if line.starts_with(' ') {
                        line = "$".to_owned() + &line;
                    }
                    line = line.trim().to_string();
                    let file_res = tempfile();
                    match file_res {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("unable to create temporary REPL file: {}", e);
                            std::process::exit(1);
                        }
                    }
                    let mut file = file_res.unwrap();
                    let res = file.write_all(line.as_bytes());
                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("unable to write content to temporary REPL file: {}", e);
                            std::process::exit(1);
                        }
                    }
                    file.seek(SeekFrom::Start(0)).unwrap();

                    let mut bufread: Box<dyn BufRead> = Box::new(BufReader::new(file));
                    rl_rr.borrow_mut().add_history_entry(line.as_str());
                    let chunk_opt = vm.interpret(global_functions.clone(), &mut bufread, "(main)");
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
                .append(true)
                .open(&history_path)
                .unwrap();
            let history_fd = history_file.as_raw_fd();
            flock(history_fd, FlockArg::LockExclusive).unwrap();

            for i in history_start_len..history_end_len {
                writeln!(history_file, "{}",
                         rl_rr.borrow().history().get(i).unwrap()).unwrap();
            }

            drop(history_file);
        }
    }
}

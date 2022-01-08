extern crate cosh;
extern crate ctrlc;
extern crate getopts;
extern crate regex;
extern crate rustyline;
extern crate rustyline_derive;
extern crate tempfile;

use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::io::{Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use getopts::Options;
use regex::Regex;
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::config::OutputStreamType;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{CompletionType, Config, Context, EditMode, Editor};
use rustyline_derive::Helper;
use tempfile::tempfile;

use cosh::compiler::Compiler;
use cosh::vm::VM;

#[derive(Helper)]
struct RLHelper {
    completer: FilenameCompleter,
}

impl Completer for RLHelper {
    type Candidate = Pair;

    fn complete(
        &self, line: &str, pos: usize, ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for RLHelper {
}

impl Highlighter for RLHelper {
}

impl Validator for RLHelper {
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] file", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");
    opts.optflag("", "bytecode", "input file is bytecode (for compilation)");
    opts.optflag("c", "compile", "compile to bytecode");
    opts.optflag("", "disassemble", "disassemble from bytecode");
    opts.optflag("", "no-rt", "run without loading runtime");
    opts.optflag("d", "debug", "show debug information");
    opts.optopt("o", "", "set output file name for compilation", "NAME");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            eprintln!("unable to parse option: {}", f.to_string());
            std::process::exit(1);
        }
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }
    if matches.opt_present("disassemble") && matches.opt_present("bytecode") {
        println!(
            "--bytecode and --disassemble options are mutually exclusive."
        );
        print_usage(&program, opts);
        return;
    }

    let debug = matches.opt_present("debug");

    if !matches.free.is_empty() {
        let path = &matches.free[0];
        if matches.opt_present("disassemble") {
            let mut compiler = Compiler::new(debug);
            let chunk_opt = compiler.deserialise(path);
            if chunk_opt.is_none() {
                eprintln!("unable to deserialise file");
                std::process::exit(1);
            }
            let chunk = chunk_opt.unwrap();
            chunk.disassemble(path);
        } else if matches.opt_present("bytecode") {
            let mut compiler = Compiler::new(debug);
            let chunk_opt = compiler.deserialise(path);
            if chunk_opt.is_none() {
                eprintln!("unable to deserialise file");
                std::process::exit(1);
            }
            let chunk = chunk_opt.unwrap();
            let mut vm = VM::new(true, debug);
            let mut scopes = Vec::new();
            scopes.push(RefCell::new(HashMap::new()));
            let mut functions = Vec::new();
            if !matches.opt_present("no-rt") {
                let rtchunk_opt =
                    compiler.deserialise("/usr/local/lib/cosh/rt.chc");
                if rtchunk_opt.is_none() {
                    let rtchunk_opt =
                        compiler.deserialise("./rt.chc");
                    if rtchunk_opt.is_none() {
                        eprintln!("unable to deserialise runtime library");
                        std::process::exit(1);
                    }
                }
                functions.push(rtchunk_opt.unwrap());
            }
            let mut call_stack_chunks = Vec::new();
            if functions.len() > 0 {
                call_stack_chunks.push(&functions[0]);
            }
            let mut chunk_values = HashMap::new();
            let mut prev_local_vars_stacks = vec![];
            let mut global_functions = RefCell::new(HashMap::new());
            let running = Arc::new(AtomicBool::new(true));
            vm.run(
                &mut scopes,
                &mut global_functions,
                &mut call_stack_chunks,
                &chunk,
                &mut chunk_values,
                0,
                None,
                None,
                &mut prev_local_vars_stacks,
                (0, 0),
                running.clone(),
            );
        } else {
            let file_res = fs::File::open(path);
            match file_res {
                Ok(_) => {}
                Err(e) => {
                    let err_str =
                        format!("unable to open file: {}", e.to_string());
                    eprintln!("{}", err_str);
                    std::process::exit(1);
                }
            }
            let file = file_res.unwrap();
            let mut bufread: Box<dyn BufRead> = Box::new(BufReader::new(file));
            if matches.opt_present("c") {
                let mut compiler = Compiler::new(debug);
                let re_pre = Regex::new(r#".*/"#).unwrap();
                let path1 = re_pre.replace_all(path, "");
                let re_post = Regex::new(r#"\..*"#).unwrap();
                let name = re_post.replace_all(&path1, "");

                let res = compiler.compile(&mut bufread, &name);
                match res {
                    Some(chunk) => {
                        let output_path_opt = matches.opt_str("o");
                        if output_path_opt.is_none() {
                            eprintln!(
                                "output path is required for compilation"
                            );
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
                                    err_str = e.to_string().clone();
                                }
                            }
                        }
                        if res == false {
                            eprintln!(
                                "unable to write to path {}: {}",
                                output_path, err_str
                            );
                        }
                    }
                    _ => {}
                }
            } else {
                let mut vm = VM::new(true, debug);
                let mut compiler = Compiler::new(debug);
                let mut global_functions = HashMap::new();

                if !matches.opt_present("no-rt") {
                    let rtchunk_opt =
                        compiler.deserialise("/usr/local/lib/cosh/rt.chc");
                    if rtchunk_opt.is_none() {
                        let rtchunk_opt =
                            compiler.deserialise("./rt.chc");
                        if rtchunk_opt.is_none() {
                            eprintln!("unable to deserialise runtime library");
                            std::process::exit(1);
                        }
                    }
                    let rtchunk = rtchunk_opt.unwrap();
                    let functions = rtchunk.functions.borrow_mut();
                    for (k, v) in functions.iter() {
                        global_functions.insert(k.clone(), v.clone());
                    }
                }

                let variables = HashMap::new();
                let running = Arc::new(AtomicBool::new(true));
                vm.interpret(
                    global_functions.clone(),
                    variables.clone(),
                    &mut bufread,
                    running.clone(),
                );
            }
        }
    } else {
        let mut compiler = Compiler::new(debug);
        let mut global_functions = HashMap::new();
        let mut variables = HashMap::new();

        if !matches.opt_present("no-rt") {
            let rtchunk_opt =
                compiler.deserialise("/usr/local/lib/cosh/rt.chc");
            if rtchunk_opt.is_none() {
                let rtchunk_opt =
                    compiler.deserialise("./rt.chc");
                if rtchunk_opt.is_none() {
                    eprintln!("unable to deserialise runtime library");
                    std::process::exit(1);
                }
            }
            let rtchunk = rtchunk_opt.unwrap();
            let functions = rtchunk.functions.borrow_mut();
            for (k, v) in functions.iter() {
                global_functions.insert(k.clone(), v.clone());
            }
        }

        let mut vm = VM::new(true, debug);

        let config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .output_stream(OutputStreamType::Stdout)
            .build();

        let helper = RLHelper {
            completer: FilenameCompleter::new(),
        };

        let mut rl = Editor::with_config(config);
        rl.set_helper(Some(helper));
        if rl.load_history(".cosh_history").is_err() {}

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        ctrlc::set_handler(move || {
            running_clone.store(false, Ordering::SeqCst);
        })
        .unwrap();

        loop {
            let cwd_res = env::current_dir();
            match cwd_res {
                Ok(_) => {}
                Err(e) => {
                    eprintln!(
                        "unable to get current working directory: {}",
                        e.to_string()
                    );
                    std::process::exit(1);
                }
            }
            let cwd = cwd_res.unwrap();
            let cwd_str = cwd.as_path().to_str().unwrap();
            let prompt = format!("{}$ ", cwd_str);

            let readline_res = rl.readline(&prompt);
            match readline_res {
                Ok(mut line) => {
                    if line.len() == 0 {
                        continue;
                    }
                    if line.chars().nth(0).unwrap() == ' ' {
                        line = "$".to_owned() + &line;
                    }
                    let file_res = tempfile();
                    match file_res {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!(
                                "unable to create temporary REPL file: {}",
                                e.to_string()
                            );
                            std::process::exit(1);
                        }
                    }
                    let mut file = file_res.unwrap();
                    let res = file.write_all(line.as_bytes());
                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!(
                                "unable to write content to temporary REPL file: {}",
                                e.to_string()
                            );
                            std::process::exit(1);
                        }
                    }
                    file.seek(SeekFrom::Start(0)).unwrap();

                    let mut bufread: Box<dyn BufRead> =
                        Box::new(BufReader::new(file));
                    rl.add_history_entry(line.as_str());
                    let (chunk_opt, updated_variables, mut updated_functions) = vm.interpret(
                        global_functions,
                        variables.clone(),
                        &mut bufread,
                        running.clone(),
                    );
                    if updated_functions.len() > 0 {
                        global_functions = updated_functions.remove(0).into_inner();
                    } else {
                        global_functions = HashMap::new();
                    }
                    for (k, v) in updated_variables.iter() {
                        variables.insert(k.clone(), v.clone());
                    }
                    match chunk_opt {
                        Some(chunk) => {
                            let chunk_functions = chunk.functions.borrow();
                            for (k, v) in chunk_functions.iter() {
                                if !k.starts_with("anon") {
                                    global_functions.insert(k.clone(), v.clone());
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
        let res = rl.save_history(".cosh_history");
        match res {
            Err(e) => {
                eprintln!("unable to save REPL history: {}", e.to_string());
            }
            _ => {}
        }
    }
}

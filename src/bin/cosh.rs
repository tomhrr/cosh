extern crate cosh;
extern crate ctrlc;
extern crate dirs_next;
extern crate getopts;
extern crate memchr;
extern crate regex;
extern crate rustyline;
extern crate rustyline_derive;
extern crate searchpath;
extern crate tempfile;

use std::borrow::Cow::{self, Borrowed};
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::env::current_dir;
use std::fs;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::io::{Seek, SeekFrom};
use std::path::{self, Path};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use dirs_next::home_dir;
use getopts::Options;
use memchr::memchr;
use regex::Regex;
use rustyline::completion::{escape, unescape, Candidate, Completer, Pair, Quote};
use rustyline::config::OutputStreamType;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{CompletionType, Config, Context, EditMode, Editor, Result};
use rustyline_derive::Helper;
use searchpath::search_path;
use tempfile::tempfile;

use cosh::compiler::Compiler;
use cosh::vm::VM;

// Most of the code through to 'impl Completer for ShellCompleter' is
// taken from kkawakam/rustyline#574 as at 3a41ee9, with some small
// changes.  Licence text from that repository:
//
// The MIT License (MIT)
//
// Copyright (c) 2015 Katsu Kawakami & Rustyline authors
//
// Permission is hereby granted, free of charge, to any person
// obtaining a copy of this software and associated documentation
// files (the "Software"), to deal in the Software without
// restriction, including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense, and/or sell copies
// of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS
// BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
// ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

const ESCAPE_CHAR: Option<char> = Some('\\');
const DOUBLE_QUOTES_ESCAPE_CHAR: Option<char> = Some('\\');
const DEFAULT_BREAK_CHARS: [u8; 18] = [
    b' ', b'\t', b'\n', b'"', b'\\', b'\'', b'`', b'@', b'$', b'>', b'<', b'=', b';', b'|', b'&',
    b'{', b'(', b'\0',
];
const DOUBLE_QUOTES_SPECIAL_CHARS: [u8; 4] = [b'"', b'$', b'\\', b'`'];

#[derive(PartialEq)]
enum ScanMode {
    DoubleQuote,
    Escape,
    EscapeInDoubleQuote,
    Normal,
    SingleQuote,
}

fn normalize(s: &str) -> Cow<str> {
    Cow::Borrowed(s)
}

/// try to find an unclosed single/double quote in `s`.
/// Return `None` if no unclosed quote is found.
/// Return the unclosed quote position and if it is a double quote.
fn find_unclosed_quote(s: &str) -> Option<(usize, Quote)> {
    let char_indices = s.char_indices();
    let mut mode = ScanMode::Normal;
    let mut quote_index = 0;
    for (index, char) in char_indices {
        match mode {
            ScanMode::DoubleQuote => {
                if char == '"' {
                    mode = ScanMode::Normal;
                } else if char == '\\' {
                    mode = ScanMode::EscapeInDoubleQuote;
                }
            }
            ScanMode::Escape => {
                mode = ScanMode::Normal;
            }
            ScanMode::EscapeInDoubleQuote => {
                mode = ScanMode::DoubleQuote;
            }
            ScanMode::Normal => {
                if char == '"' {
                    mode = ScanMode::DoubleQuote;
                    quote_index = index;
                } else if char == '\\' {
                    mode = ScanMode::Escape;
                } else if char == '\'' {
                    mode = ScanMode::SingleQuote;
                    quote_index = index;
                }
            }
            ScanMode::SingleQuote => {
                if char == '\'' {
                    mode = ScanMode::Normal;
                } // no escape in single quotes
            }
        };
    }
    if ScanMode::DoubleQuote == mode || ScanMode::EscapeInDoubleQuote == mode {
        return Some((quote_index, Quote::Double));
    } else if ScanMode::SingleQuote == mode {
        return Some((quote_index, Quote::Single));
    }
    None
}

/// Given a `line` and a cursor `pos`ition,
/// try to find backward the start of a word.
/// Return (0, `line[..pos]`) if no break char has been found.
/// Return the word and its start position (idx, `line[idx..pos]`) otherwise.
pub fn extract_word<'l>(
    line: &'l str,
    pos: usize,
    esc_char: Option<char>,
    break_chars: &[u8],
) -> (usize, &'l str) {
    let line = &line[..pos];
    if line.is_empty() {
        return (0, line);
    }
    let mut start = None;
    for (i, c) in line.char_indices().rev() {
        if let (Some(esc_char), true) = (esc_char, start.is_some()) {
            if esc_char == c {
                // escaped break char
                start = None;
                continue;
            } else {
                break;
            }
        }
        if c.is_ascii() && memchr(c as u8, break_chars).is_some() {
            start = Some(i + c.len_utf8());
            if esc_char.is_none() {
                break;
            } // else maybe escaped...
        }
    }

    match start {
        Some(start) => (start, &line[start..]),
        None => (0, line),
    }
}

fn filename_complete(
    path: &str,
    esc_char: Option<char>,
    break_chars: &[u8],
    quote: Quote,
) -> Vec<Pair> {
    let sep = path::MAIN_SEPARATOR;
    let (dir_name, file_name) = match path.rfind(sep) {
        Some(idx) => path.split_at(idx + sep.len_utf8()),
        None => ("", path),
    };

    let dir_path = Path::new(dir_name);
    let dir = if dir_path.starts_with("~") {
        if let Some(home) = home_dir() {
            match dir_path.strip_prefix("~") {
                Ok(rel_path) => home.join(rel_path),
                _ => home,
            }
        } else {
            dir_path.to_path_buf()
        }
    } else if dir_path.is_relative() {
        if let Ok(cwd) = current_dir() {
            cwd.join(dir_path)
        } else {
            dir_path.to_path_buf()
        }
    } else {
        dir_path.to_path_buf()
    };

    let mut entries: Vec<Pair> = Vec::new();

    // if dir doesn't exist, then don't offer any completions
    if !dir.exists() {
        return entries;
    }

    // if any of the below IO operations have errors, just ignore them
    if let Ok(read_dir) = dir.read_dir() {
        let file_name = normalize(file_name);
        for entry in read_dir.flatten() {
            if let Some(s) = entry.file_name().to_str() {
                let ns = normalize(s);
                if ns.starts_with(file_name.as_ref()) {
                    if let Ok(metadata) = fs::metadata(entry.path()) {
                        let mut path = String::from(dir_name) + s;
                        if metadata.is_dir() {
                            path.push(sep);
                        }
                        entries.push(Pair {
                            display: String::from(s),
                            replacement: escape(path, esc_char, break_chars, quote),
                        });
                    } // else ignore PermissionDenied
                }
            }
        }
    }
    entries
}

fn bin_complete(path: &str, esc_char: Option<char>, break_chars: &[u8], quote: Quote) -> Vec<Pair> {
    let mut entries: Vec<Pair> = Vec::new();
    for file in search_path(path, std::env::var_os("PATH").as_deref(), None) {
        entries.push(Pair {
            display: file.clone(),
            replacement: escape(file, esc_char, break_chars, quote),
        });
    }

    entries
}

pub struct ShellCompleter {
    break_chars: &'static [u8],
    double_quotes_special_chars: &'static [u8],
}

fn should_complete_executable(path: &str, line: &str, start: usize) -> bool {
    // If the string prior to path comprises whitespace, then
    // executable completion should be used (unless the path is
    // qualified).
    let before = &line[0..start];
    if before.len() > 0 && before.chars().all(char::is_whitespace) {
        if !path.contains(char::is_whitespace) {
            return !(path.starts_with("./") || path.starts_with('/'));
        }
    }

    // If the string prior to path includes a $ or { character,
    // followed by (optional) whitespace, and then the path, then
    // executable completion should be used (unless the path is
    // qualified).
    let mut index_opt = before.rfind("$");
    if index_opt.is_none() {
        index_opt = before.rfind("{");
    }
    if index_opt.is_none() {
        return false;
    }
    let index = index_opt.unwrap();
    let before2_chars = &mut before[index + 1..start].chars();
    let mut hit_char = false;
    loop {
        let c_opt = before2_chars.next();
        if c_opt.is_none() {
            break;
        }
        let c = c_opt.unwrap();
        if c.is_whitespace() {
            if hit_char {
                return false;
            }
        } else {
            hit_char = true;
        }
    }
    return !(path.starts_with("./") || path.starts_with('/'));
}

impl ShellCompleter {
    /// Constructor
    pub fn new() -> Self {
        Self {
            break_chars: &DEFAULT_BREAK_CHARS,
            double_quotes_special_chars: &DOUBLE_QUOTES_SPECIAL_CHARS,
        }
    }

    /// Takes the currently edited `line` with the cursor `pos`ition and
    /// returns the start position and the completion candidates for the
    /// partial path to be completed.
    pub fn complete_path(&self, line: &str, pos: usize) -> Result<(usize, Vec<Pair>)> {
        let (start, path, _, esc_char, break_chars, quote) =
            if let Some((idx, quote)) = find_unclosed_quote(&line[..pos]) {
                let start = idx + 1;
                if quote == Quote::Double {
                    (
                        start,
                        unescape(&line[start..pos], DOUBLE_QUOTES_ESCAPE_CHAR),
                        Borrowed(&line[..pos]),
                        DOUBLE_QUOTES_ESCAPE_CHAR,
                        &self.double_quotes_special_chars,
                        quote,
                    )
                } else {
                    (
                        start,
                        Borrowed(&line[start..pos]),
                        Borrowed(&line[..pos]),
                        None,
                        &self.break_chars,
                        quote,
                    )
                }
            } else {
                let (start, path) = extract_word(line, pos, ESCAPE_CHAR, self.break_chars);
                (
                    start,
                    unescape(path, ESCAPE_CHAR),
                    Borrowed(path),
                    ESCAPE_CHAR,
                    &self.break_chars,
                    Quote::None,
                )
            };

        let mut matches = if should_complete_executable(&path, line, start) {
            bin_complete(&path, esc_char, break_chars, quote)
        } else {
            filename_complete(&path, esc_char, break_chars, quote)
        };

        #[allow(clippy::unnecessary_sort_by)]
        matches.sort_by(|a, b| a.display().cmp(b.display()));
        Ok((start, matches))
    }
}

impl Default for ShellCompleter {
    fn default() -> Self {
        Self::new()
    }
}

impl Completer for ShellCompleter {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Result<(usize, Vec<Pair>)> {
        self.complete_path(line, pos)
    }
}

#[derive(Helper)]
struct RLHelper {
    completer: ShellCompleter,
}

impl Completer for RLHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Result<(usize, Vec<Pair>)> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for RLHelper {}

impl Highlighter for RLHelper {}

impl Validator for RLHelper {}

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
        println!("--bytecode and --disassemble options are mutually exclusive.");
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
                let mut rtchunk_opt = compiler.deserialise("/usr/local/lib/cosh/rt.chc");
                if rtchunk_opt.is_none() {
                    rtchunk_opt = compiler.deserialise("./rt.chc");
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
            let chunk_functions = Vec::new();
            let mut prev_local_vars_stacks = vec![];
            let mut global_functions = RefCell::new(HashMap::new());
            let running = Arc::new(AtomicBool::new(true));
            vm.run(
                &mut scopes,
                &mut global_functions,
                &mut call_stack_chunks,
                &chunk,
                Rc::new(RefCell::new(chunk_functions)),
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
                    let err_str = format!("unable to open file: {}", e.to_string());
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
                                    err_str = e.to_string().clone();
                                }
                            }
                        }
                        if res == false {
                            eprintln!("unable to write to path {}: {}", output_path, err_str);
                        }
                    }
                    _ => {}
                }
            } else {
                let mut vm = VM::new(true, debug);
                let mut compiler = Compiler::new(debug);
                let mut global_functions = HashMap::new();

                if !matches.opt_present("no-rt") {
                    let mut rtchunk_opt = compiler.deserialise("/usr/local/lib/cosh/rt.chc");
                    if rtchunk_opt.is_none() {
                        rtchunk_opt = compiler.deserialise("./rt.chc");
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
                    "(main)",
                );
            }
        }
    } else {
        let mut compiler = Compiler::new(debug);
        let mut global_functions = HashMap::new();
        let mut variables = HashMap::new();

        if !matches.opt_present("no-rt") {
            let mut rtchunk_opt = compiler.deserialise("/usr/local/lib/cosh/rt.chc");
            if rtchunk_opt.is_none() {
                rtchunk_opt = compiler.deserialise("./rt.chc");
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

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        ctrlc::set_handler(move || {
            running_clone.store(false, Ordering::SeqCst);
        })
        .unwrap();

        if let Some(home) = home_dir() {
            let coshrc_path = format!("{}/.coshrc", home.into_os_string().into_string().unwrap());
            if Path::new(&coshrc_path).exists() {
                let file_res = fs::File::open(coshrc_path);
                match file_res {
                    Ok(file) => {
                        let mut bufread: Box<dyn BufRead> = Box::new(BufReader::new(file));
                        let (chunk_opt, updated_variables, mut updated_functions) = vm.interpret(
                            global_functions,
                            variables.clone(),
                            &mut bufread,
                            running.clone(),
                            ".coshrc",
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
                    Err(_) => {}
                }
            }
        }

        let config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .output_stream(OutputStreamType::Stdout)
            .build();

        let helper = RLHelper {
            completer: ShellCompleter::new(),
        };

        let mut rl = Editor::with_config(config);
        rl.set_helper(Some(helper));
        if rl.load_history(".cosh_history").is_err() {}

        loop {
            let cwd_res = env::current_dir();
            match cwd_res {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("unable to get current working directory: {}", e.to_string());
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
                            eprintln!("unable to create temporary REPL file: {}", e.to_string());
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

                    let mut bufread: Box<dyn BufRead> = Box::new(BufReader::new(file));
                    rl.add_history_entry(line.as_str());
                    let (chunk_opt, updated_variables, mut updated_functions) = vm.interpret(
                        global_functions,
                        variables.clone(),
                        &mut bufread,
                        running.clone(),
                        "(main)",
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

use std::cell::RefCell;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::env;
use std::fs::File;
use std::io::Write;
use std::os::fd::FromRawFd;
use std::rc::Rc;
use std::str;

use lazy_static::lazy_static;
use nix::unistd::{fork, ForkResult};
use nonblock::NonBlockingReader;
use regex::Regex;
use std::process::{Command, Stdio};

use crate::chunk::{CommandGenerator, Value};
use crate::vm::*;

lazy_static! {
    static ref START_DOUBLE_QUOTE:    Regex = Regex ::new(r#"^\s*""#).unwrap();
    static ref END_DOUBLE_QUOTE:      Regex = Regex ::new(r#""\s*$"#).unwrap();
    static ref START_SINGLE_QUOTE:    Regex = Regex ::new(r#"^\s*'"#).unwrap();
    static ref END_SINGLE_QUOTE:      Regex = Regex ::new(r#"'\s*$"#).unwrap();
    static ref END_SLASH_EXTRA:       Regex = Regex ::new(r#".*\\$"#).unwrap();
    static ref END_SLASH:             Regex = Regex ::new(r#"\\$"#).unwrap();
    static ref CAPTURE_NUM:           Regex = Regex ::new("\\{(\\d+)\\}").unwrap();
    static ref CAPTURE_WITHOUT_NUM:   Regex = Regex ::new("\\{\\}").unwrap();
    static ref HOME_DIR_TILDE:        Regex = Regex ::new("\\s~").unwrap();
    static ref LEADING_WS:            Regex = Regex ::new("^\\s*").unwrap();
    static ref ENV_VAR:               Regex = Regex ::new("^(.*)=(.*)$").unwrap();
    static ref STDOUT_REDIRECT:       Regex = Regex ::new("^1?>(.*)$").unwrap();
    static ref STDERR_REDIRECT:       Regex = Regex ::new("^2>(.*)$").unwrap();
    static ref STDOUT_APPEND_REDIRECT: Regex = Regex ::new("^1?>>(.*)$").unwrap();
    static ref STDERR_APPEND_REDIRECT: Regex = Regex ::new("^2>>(.*)$").unwrap();
}

/// Splits a string on whitespace, taking into account quoted values
/// (for both single-quotes and double-quotes).
fn split_command(s: &str) -> Option<VecDeque<String>> {
    let elements = s.split_whitespace();
    let mut final_elements = Vec::new();
    let mut buffer = Vec::new();
    let mut delimiter = '"';
    let mut add_to_next_opt: Option<String> = None;
    for e in elements {
        let mut e_str = e.to_string();
        match add_to_next_opt {
            Some(add_to_next) => {
                e_str = add_to_next + &e_str;
                add_to_next_opt = None;
            }
            _ => {
                if e_str == ">" || e_str == "2>" || e_str == "1>" || e_str == ">>" || e_str == "2>>" || e_str == "1>>" {
                    add_to_next_opt = Some(e_str);
                    continue;
                }
            }
        }
        if !buffer.is_empty() {
            if !e_str.is_empty() {
                if e_str.chars().last().unwrap() == delimiter {
                    buffer.push(e_str);
                    let new_str = buffer.join(" ");
                    if delimiter == '"' {
                        let new_str2 = START_DOUBLE_QUOTE.replace(&new_str, "");
                        let new_str3 = END_DOUBLE_QUOTE.replace(&new_str2, "");
                        final_elements.push(new_str3.to_string());
                    } else {
                        let new_str2 = START_SINGLE_QUOTE.replace(&new_str, "");
                        let new_str3 = END_SINGLE_QUOTE.replace(&new_str2, "");
                        final_elements.push(new_str3.to_string());
                    }
                    buffer.clear();
                } else {
                    buffer.push(e_str);
                }
            }
        } else if START_DOUBLE_QUOTE.is_match(&e_str) && !END_DOUBLE_QUOTE.is_match(&e_str) {
            buffer.push(e_str);
            delimiter = '"';
        } else if START_SINGLE_QUOTE.is_match(&e_str) && !END_SINGLE_QUOTE.is_match(&e_str) {
            buffer.push(e_str);
            delimiter = '\'';
        } else {
            /* If this element is a single string, then replace any
             * quotation marks that are present. */
            let new_str2 = START_DOUBLE_QUOTE.replace(&e_str, "");
            let new_str3 = END_DOUBLE_QUOTE.replace(&new_str2, "");
            let new_str4 = START_SINGLE_QUOTE.replace(&new_str3, "");
            let new_str5 = END_SINGLE_QUOTE.replace(&new_str4, "");
            final_elements.push(new_str5.to_string());
        }
    }
    if !buffer.is_empty() {
        return None;
    }

    let mut lst = VecDeque::new();
    for e in final_elements.iter() {
        if lst.is_empty() {
            lst.push_back(e.to_string());
        } else {
            let back = lst.back().unwrap();
            if END_SLASH_EXTRA.is_match(back) {
                let back = lst.pop_back().unwrap();
                let back2 = END_SLASH.replace_all(&back, "");
                let back3 = format!("{} {}", back2, e);
                lst.push_back(back3);
            } else {
                lst.push_back(e.to_string());
            }
        }
    }
    Some(lst)
}

fn restore_env(env: HashMap<String, String>, del_env: HashSet<String>) {
    for (key, value) in env {
        env::set_var(key, value);
    }
    for key in del_env {
        env::remove_var(key);
    }
}

impl VM {
    /// Takes a command string, substitutes for the {num} and {}
    /// stack element placeholders as well as the ~ home directory
    /// placeholder, and returns the resulting string.
    fn prepare_command(&mut self, s: &str) -> Option<String> {
        let st = StringTriple::new(s.to_string(), None);
        self.stack.push(Value::String(Rc::new(RefCell::new(st))));
        let res = self.core_fmtq();
        if res == 0 {
            return None;
        }

        let str_rr = self.stack.pop().unwrap();
        match str_rr {
            Value::String(st) => {
                let input_s = &(st.borrow().string);
                let final_s: String;
                let homedir_res = std::env::var("HOME");
                match homedir_res {
                    Ok(homedir) => {
                        let s = " ".to_owned() + &homedir;
                        final_s = HOME_DIR_TILDE.replace_all(input_s, &*s).to_string();
                    }
                    _ => {
                        final_s = input_s.to_string();
                    }
                }

                Some(final_s)
            }
            _ => {
                eprintln!("expected string!");
                std::process::abort();
            }
        }
    }

    fn prepare_and_split_command(
        &mut self,
        cmd: &str,
        accept_redirects: bool
    ) -> Option<(String,
                 Vec<String>,
                 HashMap<String, String>,
                 HashSet<String>,
                 Option<(String, bool)>,
                 Option<(String, bool)>)> {
        let prepared_cmd_opt = self.prepare_command(cmd);
        if prepared_cmd_opt.is_none() {
            return None;
        }
        let prepared_cmd = prepared_cmd_opt.unwrap();
        let elements_opt = split_command(&prepared_cmd);
        if elements_opt.is_none() {
            self.print_error("syntax error in command");
            return None;
        }
        let mut elements = elements_opt.unwrap();
        if elements.is_empty() {
            self.print_error("unable to execute empty command");
            return None;
        }

        let mut prev_env = HashMap::new();
        let mut del_env = HashSet::new();
        while !elements.is_empty() {
            let element = elements.get(0).unwrap();
            let captures = ENV_VAR.captures_iter(element);
            let mut has = false;
            for capture in captures {
                has = true;
                let key_str = capture.get(1).unwrap().as_str();
                let value_str = capture.get(2).unwrap().as_str();
                let current_str = env::var(key_str);
                match current_str {
                    Ok(s) => { prev_env.insert(key_str.to_string(), s); }
                    _     => { del_env.insert(key_str.to_string()); }
                }
                env::set_var(key_str, value_str);
            }
            if has {
                elements.pop_front();
            } else {
                break;
            }
        }

        let mut stdout_redirect = None;
        let mut stderr_redirect = None;
        if accept_redirects {
            while !elements.is_empty() {
                let len = elements.len();
                let element = elements.get(len - 1).unwrap();
                
                // Check for stdout append redirect (>> or 1>>)
                let mut captures = STDOUT_APPEND_REDIRECT.captures_iter(element);
                match captures.next() {
                    Some(capture) => {
                        let output = capture.get(1).unwrap().as_str();
                        stdout_redirect = Some((output.to_string(), true));
                        elements.pop_back();
                        continue;
                    }
                    _ => {}
                }
                
                // Check for stderr append redirect (2>>)
                captures = STDERR_APPEND_REDIRECT.captures_iter(element);
                match captures.next() {
                    Some(capture) => {
                        let output = capture.get(1).unwrap().as_str();
                        stderr_redirect = Some((output.to_string(), true));
                        elements.pop_back();
                        continue;
                    }
                    _ => {}
                }
                
                // Check for stdout regular redirect (> or 1>)
                captures = STDOUT_REDIRECT.captures_iter(element);
                match captures.next() {
                    Some(capture) => {
                        let output = capture.get(1).unwrap().as_str();
                        stdout_redirect = Some((output.to_string(), false));
                        elements.pop_back();
                        continue;
                    }
                    _ => {}
                }
                
                // Check for stderr regular redirect (2>)
                captures = STDERR_REDIRECT.captures_iter(element);
                match captures.next() {
                    Some(capture) => {
                        let output = capture.get(1).unwrap().as_str();
                        stderr_redirect = Some((output.to_string(), false));
                        elements.pop_back();
                        continue;
                    }
                    _ => {}
                }
                break;
            }
        }

        let mut element_iter = elements.iter();
        let executable_opt = element_iter.next();
        if executable_opt.is_none() {
            self.print_error("unable to execute empty command");
            return None;
        }
        let executable = executable_opt.unwrap();
        let executable_final = LEADING_WS.replace_all(executable, "").to_string();
        let args = element_iter.map(|v| v.to_string()).collect::<Vec<_>>();
        Some((executable_final, args, prev_env, del_env,
              stdout_redirect, stderr_redirect))
    }

    /// Takes a command string and a set of parameters as its
    /// arguments.  Substitutes for placeholders, executes the
    /// command, and places a generator over the standard output/error
    /// (depends on parameters) of the command onto the stack.
    pub fn core_command(&mut self, cmd: &str, params: HashSet<char>) -> i32 {
        let prepared_cmd_opt = self.prepare_and_split_command(cmd, false);
        if prepared_cmd_opt.is_none() {
            return 0;
        }
        let (executable, args, env, del_env, _, _) =
            prepared_cmd_opt.unwrap();

        let process_res = Command::new(executable)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        restore_env(env, del_env);
        match process_res {
            Ok(mut process) => {
                self.child_processes.insert(process.id(), cmd.to_string());
                let upstream_stdout = process.stdout.take().unwrap();
                let upstream_stderr = process.stderr.take().unwrap();
                let noblock_stdout = NonBlockingReader::from_fd(upstream_stdout).unwrap();
                let noblock_stderr = NonBlockingReader::from_fd(upstream_stderr).unwrap();
                let mut get_stdout = params.contains(&'o');
                let get_stderr = params.contains(&'e');
                let get_combined = params.contains(&'c');
                if !get_stdout && !get_stderr && !get_combined {
                    get_stdout = true;
                }
                let get_bytes = params.contains(&'b');
                let cmd_generator =
                    Value::CommandGenerator(Rc::new(RefCell::new(CommandGenerator::new(
                        Some(nix::unistd::Pid::from_raw(process.id() as i32)),
                        None,
                        None,
                        noblock_stdout,
                        noblock_stderr,
                        get_stdout,
                        get_stderr,
                        get_combined,
                        get_bytes,
                    ))));
                self.stack.push(cmd_generator);
            }
            Err(e) => {
                let err_str = format!("unable to run command: {}", e);
                self.print_error(&err_str);
                return 0;
            }
        }
        1
    }

    /// As per `core_command`, except that the output isn't captured
    /// and nothing is placed onto the stack.
    pub fn core_command_uncaptured(&mut self, cmd: &str) -> i32 {
        let separator = Regex::new(r"\s+&&\s+").unwrap();
        let cmds: Vec<_> = separator.split(cmd).into_iter().collect();
        let mut last_status = 0;
        for cmd in cmds {
            let prepared_cmd_opt =
                self.prepare_and_split_command(cmd, true);
            if prepared_cmd_opt.is_none() {
                return 0;
            }
            let (executable, args, env, del_env,
                 stdout_redirect_opt, stderr_redirect_opt) =
                prepared_cmd_opt.unwrap();

            let mut process_cmd = Command::new(executable);
            let mut process_im = process_cmd.args(args);

            let mut stdout_file_opt = None;
            let mut stdout_to_stderr = false;
            if let Some((stdout_redirect, is_append)) = stdout_redirect_opt {
                if stdout_redirect == "&2" {
                    stdout_to_stderr = true;
                } else {
                    let stdout_file_res = if is_append {
                        std::fs::File::options()
                            .create(true)
                            .append(true)
                            .open(&stdout_redirect)
                    } else {
                        File::create(&stdout_redirect)
                    };
                    match stdout_file_res {
                        Ok(stdout_file_arg) => {
                            stdout_file_opt = Some(stdout_file_arg.try_clone().unwrap());
                            process_im = process_im.stdout(stdout_file_arg);
                        }
                        Err(e) => {
                            let err_str = format!("unable to open stdout redirect file: {}", e);
                            self.print_error(&err_str);
                            return 0;
                        }
                    }
                }
            }

            let mut stderr_file_opt = None;
            let mut stderr_to_stdout = false;
            if let Some((stderr_redirect, is_append)) = stderr_redirect_opt {
                if stderr_redirect == "&1" {
                    stderr_to_stdout = true;
                } else {
                    let stderr_file_res = if is_append {
                        std::fs::File::options()
                            .create(true)
                            .append(true)
                            .open(&stderr_redirect)
                    } else {
                        File::create(&stderr_redirect)
                    };
                    match stderr_file_res {
                        Ok(stderr_file_arg) => {
                            stderr_file_opt = Some(stderr_file_arg.try_clone().unwrap());
                            process_im = process_im.stderr(stderr_file_arg);
                        }
                        Err(e) => {
                            let err_str = format!("unable to open stderr redirect file: {}", e);
                            self.print_error(&err_str);
                            return 0;
                        }
                    }
                }
            }

            if stdout_to_stderr {
                match stderr_file_opt {
                    Some(stderr_file) => {
                        process_im =
                            process_im.stdout(stderr_file.try_clone().unwrap());
                    }
                    _ => {
                        unsafe {
                            process_im =
                                process_im.stdout(Stdio::from_raw_fd(2));
                        }
                    }
                }
            }
            if stderr_to_stdout {
                match stdout_file_opt {
                    Some(stdout_file) => {
                        process_im =
                            process_im.stderr(stdout_file.try_clone().unwrap());
                    }
                    _ => {
                        unsafe {
                            process_im =
                                process_im.stderr(Stdio::from_raw_fd(1));
                        }
                    }
                }
            }

            let process_res = process_im.spawn();
            restore_env(env, del_env);
            match process_res {
                Ok(mut process) => {
                    let res = process.wait();
                    match res {
                        Ok(es) => {
                            let code = es.code();
                            match code {
                                Some(n) => {
                                    last_status = n;
                                    if last_status != 0 {
                                        break;
                                    }
                                }
                                _ => {
                                    let err_str = format!("command execution failed");
                                    self.print_error(&err_str);
                                    return 0;
                                }
                            }
                        }
                        Err(e) => {
                            let err_str = format!("command execution failed: {}", e);
                            self.print_error(&err_str);
                            return 0;
                        }
                    }
                }
                Err(e) => {
                    let err_str = format!("unable to execute command: {}", e);
                    self.print_error(&err_str);
                    return 0;
                }
            }
        }
        self.stack.push(Value::Int(last_status));
        1
    }

    /// Takes a generator and a command as its arguments.  Takes
    /// output from the generator and pipes it to the standard input
    /// of the command, and places a generator over the command's
    /// standard output onto the stack.
    pub fn core_pipe(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("| requires two arguments");
            return 0;
        }

        let cmd_rr = self.stack.pop().unwrap();

        match cmd_rr {
            Value::Command(s, _) => {
                let prepared_cmd_opt =
                    self.prepare_and_split_command(&s, false);
                if prepared_cmd_opt.is_none() {
                    return 0;
                }
                let (executable, args, env, del_env, _, _) =
                    prepared_cmd_opt.unwrap();

                let process_ = Command::new(executable)
                    .args(args)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .stdin(Stdio::piped())
                    .spawn();
                restore_env(env, del_env);
                match process_ {
                    Ok(mut process) => {
                        self.child_processes.insert(process.id(), s.to_string());
                        let pipe_pid = process.id();
                        let upstream_stdin_opt = process.stdin;
                        if upstream_stdin_opt.is_none() {
                            let err_str = "unable to get stdin from parent".to_string();
                            self.print_error(&err_str);
                            return 0;
                        }
                        let mut upstream_stdin = upstream_stdin_opt.unwrap();
                        unsafe {
                            match fork() {
                                Ok(ForkResult::Parent { child }) => {
                                    let input_value = self.stack.pop().unwrap();
                                    let upstream_stdout_opt = process.stdout.take();
                                    if upstream_stdout_opt.is_none() {
                                        let err_str = "unable to get stdout from parent".to_string();
                                        self.print_error(&err_str);
                                        return 0;
                                    }
                                    let upstream_stdout = upstream_stdout_opt.unwrap();

                                    let upstream_stderr_opt = process.stderr.take();
                                    if upstream_stderr_opt.is_none() {
                                        let err_str = "unable to get stderr from parent".to_string();
                                        self.print_error(&err_str);
                                        return 0;
                                    }
                                    let upstream_stderr = upstream_stderr_opt.unwrap();

                                    let cmd_generator = Value::CommandGenerator(Rc::new(RefCell::new(
                                        CommandGenerator::new(
                                            Some(child),
                                            Some(nix::unistd::Pid::from_raw(pipe_pid as i32)),
                                            Some(input_value),
                                            NonBlockingReader::from_fd(upstream_stdout).unwrap(),
                                            NonBlockingReader::from_fd(upstream_stderr).unwrap(),
                                            true,
                                            false,
                                            false,
                                            false,
                                        ),
                                    )));
                                    self.stack.push(cmd_generator);
                                }
                                Ok(ForkResult::Child) => {
                                    loop {
                                        let dup_res = self.opcode_dup();
                                        if dup_res == 0 {
                                            return 0;
                                        }
                                        let shift_res = self.opcode_shift();
                                        if shift_res == 0 {
                                            return 0;
                                        }
                                        let element_rr = self.stack.pop().unwrap();
                                        if let Value::Null = element_rr {
                                            break;
                                        }
                                        let element_str_opt: Option<&str>;
                                        to_str!(element_rr, element_str_opt);

                                        match element_str_opt {
                                            Some(s) => {
                                                let res = upstream_stdin.write(s.as_bytes());
                                                match res {
                                                    Ok(_) => {}
                                                    _ => {
                                                        eprintln!("unable to write to parent process!");
                                                        std::process::abort();
                                                    }
                                                }
                                            }
                                            _ => {
                                                break;
                                            }
                                        }
                                    }
                                    std::process::exit(0);
                                }
                                _ => {
                                    eprintln!("unexpected fork result!");
                                    std::process::abort();
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let err_str = format!("unable to run command: {}", e);
                        self.print_error(&err_str);
                        return 0;
                    }
                }
            }
            _ => {
                self.print_error("| argument must be a command");
            }
        }
        1
    }

    /// Takes a command generator as its single argument, and returns
    /// the exit status, terminating the process if required.
    pub fn core_status(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("status requires one argument");
            return 0;
        }

        let mut cg_rr = self.stack.pop().unwrap();
        match cg_rr {
            Value::CommandGenerator(ref mut cg) => {
                cg.borrow_mut().cleanup();
                self.stack.push(cg.borrow().status());
                return 1;
            }
            _ => {
                self.print_error("status argument must be command generator");
                return 0;
            }
        }
    }

    /// Takes a string as its single argument, and runs the string as
    /// a command (uncaptured).
    pub fn core_exec(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("exec requires one argument");
            return 0;
        }

        let cmd_rr = self.stack.pop().unwrap();
        let cmd_str_opt: Option<&str>;
        to_str!(cmd_rr, cmd_str_opt);

        match cmd_str_opt {
            None => {
                self.print_error("exec argument must be a string");
                return 0;
            }
            Some(s) => {
                let i = self.core_command_uncaptured(&s);
                if i == 0 {
                    return 0;
                }
            }
        }

        return 1;
    }

    /// Takes a string and a set of parameters as its arguments, and
    /// runs the string as a command (captured).
    pub fn core_cmd_internal(&mut self, params: HashSet<char>) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("cmd requires one argument");
            return 0;
        }

        let cmd_rr = self.stack.pop().unwrap();
        let cmd_str_opt: Option<&str>;
        to_str!(cmd_rr, cmd_str_opt);

        match cmd_str_opt {
            None => {
                self.print_error("exec argument must be a string");
                return 0;
            }
            Some(s) => {
                return self.core_command(&s, params);
            }
        }
    }

    pub fn core_cmd(&mut self) -> i32 {
        let params: HashSet<char> = HashSet::new();
        return self.core_cmd_internal(params);
    }

    pub fn core_cmde(&mut self) -> i32 {
        let mut params: HashSet<char> = HashSet::new();
        params.insert('e');
        return self.core_cmd_internal(params);
    }


    pub fn core_cmdo(&mut self) -> i32 {
        let mut params: HashSet<char> = HashSet::new();
        params.insert('o');
        return self.core_cmd_internal(params);
    }

    pub fn core_cmdeo(&mut self) -> i32 {
        let mut params: HashSet<char> = HashSet::new();
        params.insert('e');
        params.insert('o');
        return self.core_cmd_internal(params);
    }

    pub fn core_cmdc(&mut self) -> i32 {
        let mut params: HashSet<char> = HashSet::new();
        params.insert('c');
        return self.core_cmd_internal(params);
    }
}

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::BufReader;
use std::io::Write;
use std::rc::Rc;
use std::str;

use lazy_static::lazy_static;
use nix::unistd::{fork, ForkResult};
use regex::Regex;
use std::process::{Command, Stdio};

use chunk::Value;
use vm::*;

lazy_static! {
    static ref START_DOUBLE_QUOTE: Regex = Regex::new(r#"^\s*""#).unwrap();
    static ref END_DOUBLE_QUOTE: Regex = Regex::new(r#""\s*$"#).unwrap();
    static ref START_SINGLE_QUOTE: Regex = Regex::new(r#"^\s*'"#).unwrap();
    static ref END_SINGLE_QUOTE: Regex = Regex::new(r#"'\s*$"#).unwrap();
    static ref END_SLASH_EXTRA: Regex = Regex::new(r#".*\\$"#).unwrap();
    static ref END_SLASH: Regex = Regex::new(r#"\\$"#).unwrap();
    static ref CAPTURE_NUM: Regex = Regex::new("\\{(\\d+)\\}").unwrap();
    static ref CAPTURE_WITHOUT_NUM: Regex = Regex::new("\\{\\}").unwrap();
    static ref HOME_DIR_TILDE: Regex = Regex::new("\\s~").unwrap();
    static ref LEADING_WS: Regex = Regex::new("^\\s*").unwrap();
}

/// Splits a string on whitespace, taking into account quoted values
/// (for both single-quotes and double-quotes).
fn split_command(s: &str) -> Option<VecDeque<String>> {
    let elements = s.split_whitespace();
    let mut final_elements = Vec::new();
    let mut buffer = Vec::new();
    let mut delimiter = '"';
    for e in elements {
        let e_str = e.to_string();
        if buffer.len() > 0 {
            if e_str.len() > 0 {
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
            if delimiter == '"' {
                let new_str2 = START_DOUBLE_QUOTE.replace(&e_str, "");
                let new_str3 = END_DOUBLE_QUOTE.replace(&new_str2, "");
                final_elements.push(new_str3.to_string());
            } else {
                let new_str2 = START_SINGLE_QUOTE.replace(&e_str, "");
                let new_str3 = END_SINGLE_QUOTE.replace(&new_str2, "");
                final_elements.push(new_str3.to_string());
            }
        }
    }
    if buffer.len() > 0 {
        return None;
    }

    let mut lst = VecDeque::new();
    for e in final_elements.iter() {
        if lst.len() == 0 {
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
    return Some(lst);
}

impl VM {
    /// Takes a command string, substitutes for the {num} and {}
    /// stack element placeholders as well as the ~ home directory
    /// placeholder, and returns the resulting string.
    fn prepare_command(&mut self, s: &str) -> Option<String> {
        let captures = CAPTURE_NUM.captures_iter(s);
        let mut final_s = s.to_string();
        for capture in captures {
            let capture_str = capture.get(1).unwrap().as_str();
            let capture_num_res = capture_str.parse::<usize>();
            let capture_num = match capture_num_res {
                Ok(n) => n,
                Err(_) => {
                    self.print_error("invalid stack element");
                    return None;
                }
            };

            let capture_el_rr_opt = self.stack.get(self.stack.len() - 1 - capture_num);
            match capture_el_rr_opt {
                Some(capture_el_rr) => {
                    let capture_el_str_s;
                    let capture_el_str_b;
                    let capture_el_str_str;
                    let capture_el_str_bk: Option<String>;
                    let capture_el_str_opt: Option<&str> = match capture_el_rr {
                        Value::String(sp) => {
                            capture_el_str_s = sp;
                            capture_el_str_b = capture_el_str_s.borrow();
                            Some(&capture_el_str_b.s)
                        }
                        _ => {
                            capture_el_str_bk = capture_el_rr.to_string();
                            match capture_el_str_bk {
                                Some(s) => {
                                    capture_el_str_str = s;
                                    Some(&capture_el_str_str)
                                }
                                _ => None,
                            }
                        }
                    };
                    match capture_el_str_opt {
                        Some(capture_el_str) => {
                            let capture_str_with_brackets = format!("\\{{{}\\}}", capture_str);
                            let cswb_regex = Regex::new(&capture_str_with_brackets).unwrap();
                            final_s = cswb_regex.replace_all(&final_s, capture_el_str).to_string();
                        }
                        _ => {
                            self.print_error("unable to parse command");
                            return None;
                        }
                    }
                }
                None => {
                    let err_str = format!("stack element {} not present", capture_num);
                    self.print_error(&err_str);
                    return None;
                }
            }
        }

        while CAPTURE_WITHOUT_NUM.is_match(&final_s) {
            if self.stack.len() < 1 {
                self.print_error("no more elements to pop from stack");
                return None;
            }

            let value_rr = self.stack.pop().unwrap();
            let value_s;
            let value_b;
            let value_str;
            let value_bk: Option<String>;
            let value_opt: Option<&str> = match value_rr {
                Value::String(sp) => {
                    value_s = sp;
                    value_b = value_s.borrow();
                    Some(&value_b.s)
                }
                _ => {
                    value_bk = value_rr.to_string();
                    match value_bk {
                        Some(s) => {
                            value_str = s;
                            Some(&value_str)
                        }
                        _ => None,
                    }
                }
            };

            match value_opt {
                Some(s) => {
                    final_s = CAPTURE_WITHOUT_NUM.replace(&final_s, s).to_string();
                }
                _ => {
                    self.print_error("unable to parse command");
                    return None;
                }
            }
        }

        let homedir_res = std::env::var("HOME");
        match homedir_res {
            Ok(homedir) => {
                let s = " ".to_owned() + &homedir;
                final_s = HOME_DIR_TILDE.replace_all(&final_s, &*s).to_string();
            }
            _ => {}
        }

        return Some(final_s);
    }

    fn prepare_and_split_command(
        &mut self,
        cmd: &str,
    ) -> Option<(String, Vec<String>)> {
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
        let elements = elements_opt.unwrap();

        let mut element_iter = elements.iter();
        let executable_opt = element_iter.next();
        if executable_opt.is_none() {
            self.print_error("unable to execute empty command");
            return None;
        }
        let executable = executable_opt.unwrap();
        let executable_final = LEADING_WS.replace_all(&executable, "").to_string();
        let args = element_iter.map(|v| v.to_string()).collect::<Vec<_>>();
        return Some((executable_final.to_string(), args));
    }

    /// Takes a command string as its single argument.  Substitutes
    /// for placeholders, executes the command, and places a generator
    /// over the standard output of the command onto the stack.
    pub fn core_command(&mut self, cmd: &str) -> i32 {
        let prepared_cmd_opt = self.prepare_and_split_command(cmd);
        if prepared_cmd_opt.is_none() {
            return 0;
        }
        let (executable, args) = prepared_cmd_opt.unwrap();

        let process_res = Command::new(executable)
            .args(args)
            .stdout(Stdio::piped())
            .spawn();
        match process_res {
            Ok(process) => {
                let upstream_stdout = process.stdout.unwrap();
                let cmd_generator =
                    Value::CommandGenerator(Rc::new(RefCell::new(BufReader::new(upstream_stdout))));
                self.stack.push(cmd_generator);
            }
            Err(e) => {
                let err_str = format!("unable to run command: {}", e.to_string());
                self.print_error(&err_str);
                return 0;
            }
        }
        return 1;
    }

    /// As per `core_command`, except that the output isn't captured
    /// and nothing is placed onto the stack.
    pub fn core_command_uncaptured(&mut self, cmd: &str) -> i32 {
        let prepared_cmd_opt = self.prepare_and_split_command(cmd);
        if prepared_cmd_opt.is_none() {
            return 0;
        }
        let (executable, args) = prepared_cmd_opt.unwrap();

        let process_res = Command::new(executable).args(args).spawn();
        match process_res {
            Ok(mut process) => {
                let res = process.wait();
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        let err_str = format!("command execution failed: {}", e.to_string());
                        self.print_error(&err_str);
                        return 0;
                    }
                }
            }
            Err(e) => {
                let err_str = format!("unable to execute command: {}", e.to_string());
                self.print_error(&err_str);
                return 0;
            }
        }
        return 1;
    }

    /// Takes a generator and a command as its arguments.  Takes
    /// output from the generator and pipes it to the standard input
    /// of the command, and places a generator over the command's
    /// output onto the stack.
    pub fn core_pipe(
        &mut self,
    ) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("| requires two arguments");
            return 0;
        }

        let cmd_rr = self.stack.pop().unwrap();

        match cmd_rr {
            Value::Command(s) => {
                let prepared_cmd_opt =
                    self.prepare_and_split_command(&s.borrow());
                if prepared_cmd_opt.is_none() {
                    return 0;
                }
                let (executable, args) = prepared_cmd_opt.unwrap();

                let process_ = Command::new(executable)
                    .args(args)
                    .stdout(Stdio::piped())
                    .stdin(Stdio::piped())
                    .spawn();
                match process_ {
                    Ok(process) => {
                        let upstream_stdin_opt = process.stdin;
                        if upstream_stdin_opt.is_none() {
                            let err_str = format!("unable to get stdin from parent");
                            self.print_error(&err_str);
                            return 0;
                        }
                        let mut upstream_stdin = upstream_stdin_opt.unwrap();
                        match fork() {
                            Ok(ForkResult::Parent { .. }) => {
                                self.stack.pop();
                                let upstream_stdout_opt = process.stdout;
                                if upstream_stdout_opt.is_none() {
                                    let err_str = format!("unable to get stdout from parent");
                                    self.print_error(&err_str);
                                    return 0;
                                }
                                let upstream_stdout = upstream_stdout_opt.unwrap();
                                let cmd_generator = Value::CommandGenerator(Rc::new(RefCell::new(
                                    BufReader::new(upstream_stdout),
                                )));
                                self.stack.push(cmd_generator);
                            }
                            Ok(ForkResult::Child) => {
                                loop {
                                    let dup_res =
                                        self.opcode_dup();
                                    if dup_res == 0 {
                                        return 0;
                                    }
                                    let shift_res = self.opcode_shift();
                                    if shift_res == 0 {
                                        return 0;
                                    }
                                    let element_rr = self.stack.pop().unwrap();
                                    match element_rr {
                                        Value::Null => {
                                            break;
                                        }
                                        _ => {}
                                    }
                                    let element_s;
                                    let element_b;
                                    let element_str;
                                    let element_bk: Option<String>;
                                    let element_str_opt: Option<&str> = match element_rr {
                                        Value::String(sp) => {
                                            element_s = sp;
                                            element_b = element_s.borrow();
                                            Some(&element_b.s)
                                        }
                                        _ => {
                                            element_bk = element_rr.to_string();
                                            match element_bk {
                                                Some(s) => {
                                                    element_str = s;
                                                    Some(&element_str)
                                                }
                                                _ => None,
                                            }
                                        }
                                    };

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
                    Err(e) => {
                        let err_str = format!("unable to run command: {}", e.to_string());
                        self.print_error(&err_str);
                        return 0;
                    }
                }
            }
            _ => {
                self.print_error("| argument must be a command");
            }
        }
        return 1;
    }
}

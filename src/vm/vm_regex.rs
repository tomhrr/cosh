use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use regex::Regex;

use chunk::{print_error, Chunk, StringPair, Value};
use vm::*;

lazy_static! {
    static ref RE_ADJUST: Regex = Regex::new(r"\\([\d+])").unwrap();
}

impl VM {
    /// Takes a value that can be stringified and a regex string as
    /// its arguments.  Tests whether the value matches as against the
    /// regex and puts a boolean onto the stack accordingly.
    pub fn core_m(&mut self, chunk: Rc<Chunk>, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk.clone(), i, "m requires two arguments");
            return 0;
        }

        let regex_rr = self.stack.pop().unwrap();
        let regex_str_rr_opt = VM::to_string_value(regex_rr);
        if regex_str_rr_opt.is_none() {
            print_error(chunk.clone(), i, "regex must be a string");
            return 0;
        }
        let mut regex_str_rr = regex_str_rr_opt.unwrap();

        {
            let res = regex_str_rr.gen_regex(chunk.clone(), i);
            if !res {
                return 0;
            }
        }

        let regex_s;
        let regex_b;
        let regex_rb;
        let regex_opt: Option<&Regex> = match regex_str_rr {
            Value::String(sp) => {
                regex_s = sp;
                regex_b = regex_s.borrow();
                match regex_b.r {
                    Some(ref rb) => {
                        regex_rb = rb;
                        Some(regex_rb)
                    }
                    None => {
                        eprintln!("gen_regex must be called before to_regex!");
                        std::process::abort();
                    }
                }
            }
            _ => {
                eprintln!("gen_regex must be called before to_regex!");
                std::process::abort();
            }
        };

        let str_rr = self.stack.pop().unwrap();
        let str_s;
        let str_b;
        let str_str;
        let str_bk: Option<String>;
        let str_opt: Option<&str> = match str_rr {
            Value::String(sp) => {
                str_s = sp;
                str_b = str_s.borrow();
                Some(&str_b.s)
            }
            _ => {
                str_bk = str_rr.to_string();
                match str_bk {
                    Some(s) => {
                        str_str = s;
                        Some(&str_str)
                    }
                    _ => None,
                }
            }
        };

        match (regex_opt, str_opt) {
            (Some(regex), Some(s)) => {
                let res = if regex.is_match(s) { 1 } else { 0 };
                self.stack.push(Value::Int(res));
            }
            (_, Some(_)) => {
                print_error(chunk.clone(), i, "first m argument must be string");
                return 0;
            }
            (_, _) => {
                print_error(chunk.clone(), i, "second m argument must be string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a value that can be stringified, a regex string, and a
    /// replacement string as its arguments.  Runs a
    /// search-and-replace against the string based on the regex, and
    /// puts the resulting string onto the stack.
    pub fn core_s(&mut self, chunk: Rc<Chunk>, i: usize) -> i32 {
        if self.stack.len() < 3 {
            print_error(chunk.clone(), i, "s requires three arguments");
            return 0;
        }

        let repl_rr = self.stack.pop().unwrap();
        let repl_str_rr_opt = VM::to_string_value(repl_rr);
        if repl_str_rr_opt.is_none() {
            print_error(chunk.clone(), i, "replacement must be a string");
            return 0;
        }
        let repl_str_rr = repl_str_rr_opt.unwrap();

        let regex_rr = self.stack.pop().unwrap();
        let regex_str_rr_opt = VM::to_string_value(regex_rr);
        if regex_str_rr_opt.is_none() {
            print_error(chunk.clone(), i, "regex must be a string");
            return 0;
        }
        let mut regex_str_rr = regex_str_rr_opt.unwrap();

        {
            let res = regex_str_rr.gen_regex(chunk.clone(), i);
            if !res {
                return 0;
            }
        }
        let regex_s;
        let regex_b;
        let regex_rb;
        let regex_opt: Option<&Regex> = match regex_str_rr {
            Value::String(sp) => {
                regex_s = sp;
                regex_b = regex_s.borrow();
                match regex_b.r {
                    Some(ref rb) => {
                        regex_rb = rb;
                        Some(regex_rb)
                    }
                    None => {
                        eprintln!("gen_regex must be called before to_regex!");
                        std::process::abort();
                    }
                }
            }
            _ => {
                eprintln!("gen_regex must be called before to_regex!");
                std::process::abort();
            }
        };

        let repl_str_s;
        let repl_str_b;
        let repl_str_str;
        let repl_str_bk: Option<String>;
        let repl_str_opt: Option<&str> = match repl_str_rr {
            Value::String(sp) => {
                repl_str_s = sp;
                repl_str_b = repl_str_s.borrow();
                Some(&repl_str_b.s)
            }
            _ => {
                repl_str_bk = repl_str_rr.to_string();
                match repl_str_bk {
                    Some(s) => {
                        repl_str_str = s;
                        Some(&repl_str_str)
                    }
                    _ => None,
                }
            }
        };

        let str_rr = self.stack.pop().unwrap();
        let str_s;
        let str_b;
        let str_str;
        let str_bk: Option<String>;
        let str_opt: Option<&str> = match str_rr {
            Value::String(sp) => {
                str_s = sp;
                str_b = str_s.borrow();
                Some(&str_b.s)
            }
            _ => {
                str_bk = str_rr.to_string();
                match str_bk {
                    Some(s) => {
                        str_str = s;
                        Some(&str_str)
                    }
                    _ => None,
                }
            }
        };

        match (repl_str_opt, regex_opt, str_opt) {
            (Some(repl_str), Some(regex), Some(s)) => {
                let updated_repl = RE_ADJUST.replace_all(repl_str, "$${$1}");
                let updated_repl_str = updated_repl.to_string();
                let updated_str = regex.replace_all(s, &updated_repl_str[..]);
                self.stack
                    .push(Value::String(Rc::new(RefCell::new(StringPair::new(
                        updated_str.to_string(),
                        None,
                    )))));
            }
            (_, _, Some(_)) => {
                print_error(chunk.clone(), i, "first s argument must be string");
                return 0;
            }
            (_, Some(_), _) => {
                print_error(chunk.clone(), i, "second s argument must be string");
                return 0;
            }
            (_, _, _) => {
                print_error(chunk.clone(), i, "third s argument must be string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a value that can be stringified and a regex string as
    /// its arguments.  Gets the regex captures from the value, puts
    /// them into a list, and then puts that list onto the stack.
    pub fn core_c(&mut self, chunk: Rc<Chunk>, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk.clone(), i, "c requires two arguments");
            return 0;
        }

        let regex_rr = self.stack.pop().unwrap();
        let regex_str_rr_opt = VM::to_string_value(regex_rr);
        if regex_str_rr_opt.is_none() {
            print_error(chunk.clone(), i, "regex must be a string");
            return 0;
        }
        let mut regex_str_rr = regex_str_rr_opt.unwrap();

        {
            let res = regex_str_rr.gen_regex(chunk.clone(), i);
            if !res {
                return 0;
            }
        }
        let regex_s;
        let regex_b;
        let regex_rb;
        let regex_opt: Option<&Regex> = match regex_str_rr {
            Value::String(sp) => {
                regex_s = sp;
                regex_b = regex_s.borrow();
                match regex_b.r {
                    Some(ref rb) => {
                        regex_rb = rb;
                        Some(regex_rb)
                    }
                    None => {
                        eprintln!("gen_regex must be called before to_regex!");
                        std::process::abort();
                    }
                }
            }
            _ => {
                eprintln!("gen_regex must be called before to_regex!");
                std::process::abort();
            }
        };

        let str_rr = self.stack.pop().unwrap();
        let str_s;
        let str_b;
        let str_str;
        let str_bk: Option<String>;
        let str_opt: Option<&str> = match str_rr {
            Value::String(sp) => {
                str_s = sp;
                str_b = str_s.borrow();
                Some(&str_b.s)
            }
            _ => {
                str_bk = str_rr.to_string();
                match str_bk {
                    Some(s) => {
                        str_str = s;
                        Some(&str_str)
                    }
                    _ => None,
                }
            }
        };

        match (regex_opt, str_opt) {
            (Some(regex), Some(s)) => {
                let captures = regex.captures_iter(s);
                let mut lst = VecDeque::new();
                for capture in captures {
                    lst.push_back(Value::String(Rc::new(RefCell::new(StringPair::new(
                        capture.get(0).unwrap().as_str().to_string(),
                        None,
                    )))));
                }
                self.stack.push(Value::List(Rc::new(RefCell::new(lst))));
            }
            (_, Some(_)) => {
                print_error(chunk.clone(), i, "first c argument must be string");
                return 0;
            }
            (_, _) => {
                print_error(chunk.clone(), i, "second c argument must be string");
                return 0;
            }
        }
        return 1;
    }
}

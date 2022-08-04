use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use regex::Regex;

use chunk::{StringPair, Value};
use vm::*;

lazy_static! {
    static ref RE_ADJUST: Regex = Regex::new(r"\\([\d+])").unwrap();
}

impl VM {
    /// Takes a value that can be stringified and a regex string as
    /// its arguments.  Tests whether the value matches as against the
    /// regex and puts a boolean onto the stack accordingly.
    pub fn core_m(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("m requires two arguments");
            return 0;
        }

        let regex_rr = self.stack.pop().unwrap();
        let regex_opt = self.gen_regex(regex_rr);
        if regex_opt.is_none() {
            return 0;
        }

        let str_rr = self.stack.pop().unwrap();
	let str_opt: Option<&str>;
	to_str!(str_rr, str_opt);

        match (regex_opt, str_opt) {
            (Some(regex), Some(s)) => {
                let res = regex.is_match(s);
                self.stack.push(Value::Bool(res));
            }
            (_, Some(_)) => {
                self.print_error("first m argument must be string");
                return 0;
            }
            (_, _) => {
                self.print_error("second m argument must be string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a value that can be stringified, a regex string, and a
    /// replacement string as its arguments.  Runs a
    /// search-and-replace against the string based on the regex, and
    /// puts the resulting string onto the stack.
    pub fn core_s(&mut self) -> i32 {
        if self.stack.len() < 3 {
            self.print_error("s requires three arguments");
            return 0;
        }

        let repl_rr = self.stack.pop().unwrap();
        let repl_str_rr_opt = VM::to_string_value(repl_rr);
        if repl_str_rr_opt.is_none() {
            self.print_error("replacement must be a string");
            return 0;
        }
        let repl_str_rr = repl_str_rr_opt.unwrap();

        let regex_rr = self.stack.pop().unwrap();
        let regex_opt = self.gen_regex(regex_rr);
        if regex_opt.is_none() {
            return 0;
        }

	let repl_str_opt: Option<&str>;
	to_str!(repl_str_rr, repl_str_opt);

        let str_rr = self.stack.pop().unwrap();
	let str_opt: Option<&str>;
	to_str!(str_rr, str_opt);

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
                self.print_error("first s argument must be string");
                return 0;
            }
            (_, Some(_), _) => {
                self.print_error("second s argument must be string");
                return 0;
            }
            (_, _, _) => {
                self.print_error("third s argument must be string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a value that can be stringified and a regex string as
    /// its arguments.  Gets the regex captures from the value, puts
    /// them into a list, and then puts that list onto the stack.
    pub fn core_c(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("c requires two arguments");
            return 0;
        }

        let regex_rr = self.stack.pop().unwrap();
        let regex_opt = self.gen_regex(regex_rr);
        if regex_opt.is_none() {
            return 0;
        }

        let str_rr = self.stack.pop().unwrap();
	let str_opt: Option<&str>;
	to_str!(str_rr, str_opt);

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
                self.print_error("first c argument must be string");
                return 0;
            }
            (_, _) => {
                self.print_error("second c argument must be string");
                return 0;
            }
        }
        return 1;
    }
}

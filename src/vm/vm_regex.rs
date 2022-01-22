use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use regex::Regex;

use chunk::{print_error, Chunk, Value};
use vm::*;

lazy_static! {
    static ref RE_ADJUST: Regex = Regex::new(r"\\([\d+])").unwrap();
}

/// Takes a wrapped value as its single argument, and returns a
/// wrapped value for the stringified representation of the argument.
fn to_string_value(
    value_rr: Rc<RefCell<Value>>,
) -> Option<Rc<RefCell<Value>>> {
    let is_string;
    {
        let value_rrb = value_rr.borrow();
        match *value_rrb {
            Value::String(_, _) => {
                is_string = true;
            }
            _ => {
                is_string = false;
            }
        }
    }
    if is_string {
        return Some(value_rr);
    } else {
        let value_rrb = value_rr.borrow();
        let value_str_pre = value_rrb.to_string();
        let value_str_opt = to_string_2(&value_str_pre);
        match value_str_opt {
            Some(s) => Some(Rc::new(RefCell::new(Value::String(
                s.to_string(),
                None,
            )))),
            _ => None,
        }
    }
}

impl VM {
    /// Takes a value that can be stringified and a regex string as
    /// its arguments.  Tests whether the value matches as against the
    /// regex and puts a boolean onto the stack accordingly.
    pub fn core_m(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "m requires two arguments");
            return 0;
        }

        let regex_rr = self.stack.pop().unwrap();
        let regex_str_rr_opt = to_string_value(regex_rr);
        if regex_str_rr_opt.is_none() {
            print_error(chunk, i, "regex must be a string");
            return 0;
        }
        let regex_str_rr = regex_str_rr_opt.unwrap();

        {
            let mut regex_str_rrb = regex_str_rr.borrow_mut();
            let res = regex_str_rrb.gen_regex(chunk, i);
            if !res {
                return 0;
            }
        }
        let regex_str_rrb = regex_str_rr.borrow();
        let regex_opt = regex_str_rrb.to_regex();

        let str_rr = self.stack.pop().unwrap();
        let str_rrb = str_rr.borrow();
        let str_pre = str_rrb.to_string();
        let str_opt = to_string_2(&str_pre);

        match (regex_opt, str_opt) {
            (Some(regex), Some(s)) => {
                let res = if regex.is_match(s) { 1 } else { 0 };
                self.stack.push(Rc::new(RefCell::new(Value::Int(res))));
            }
            (_, Some(_)) => {
                print_error(chunk, i, "first m argument must be string");
                return 0;
            }
            (_, _) => {
                print_error(chunk, i, "second m argument must be string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a value that can be stringified, a regex string, and a
    /// replacement string as its arguments.  Runs a
    /// search-and-replace against the string based on the regex, and
    /// puts the resulting string onto the stack.
    pub fn core_s(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 3 {
            print_error(chunk, i, "s requires three arguments");
            return 0;
        }

        let repl_rr = self.stack.pop().unwrap();
        let repl_str_rr_opt = to_string_value(repl_rr);
        if repl_str_rr_opt.is_none() {
            print_error(chunk, i, "replacement must be a string");
            return 0;
        }
        let repl_str_rr = repl_str_rr_opt.unwrap();

        let regex_rr = self.stack.pop().unwrap();
        let regex_str_rr_opt = to_string_value(regex_rr);
        if regex_str_rr_opt.is_none() {
            print_error(chunk, i, "regex must be a string");
            return 0;
        }
        let regex_str_rr = regex_str_rr_opt.unwrap();

        {
            let mut regex_str_rrb = regex_str_rr.borrow_mut();
            let res = regex_str_rrb.gen_regex(chunk, i);
            if !res {
                return 0;
            }
        }
        let regex_str_rrb = regex_str_rr.borrow();
        let regex_opt = regex_str_rrb.to_regex();

        let repl_str_rrb = repl_str_rr.borrow();
        let repl_str_pre = repl_str_rrb.to_string();
        let repl_str_opt = to_string_2(&repl_str_pre);

        let str_rr = self.stack.pop().unwrap();
        let str_rrb = str_rr.borrow();
        let str_pre = str_rrb.to_string();
        let str_opt = to_string_2(&str_pre);

        match (repl_str_opt, regex_opt, str_opt) {
            (Some(repl_str), Some(regex), Some(s)) => {
                let updated_repl = RE_ADJUST.replace_all(repl_str, "$${$1}");
                let updated_repl_str = updated_repl.to_string();
                let updated_str = regex.replace_all(s, &updated_repl_str[..]);
                self.stack.push(Rc::new(RefCell::new(Value::String(
                    updated_str.to_string(),
                    None,
                ))));
            }
            (_, _, Some(_)) => {
                print_error(chunk, i, "first s argument must be string");
                return 0;
            }
            (_, Some(_), _) => {
                print_error(chunk, i, "second s argument must be string");
                return 0;
            }
            (_, _, _) => {
                print_error(chunk, i, "third s argument must be string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a value that can be stringified and a regex string as
    /// its arguments.  Gets the regex captures from the value, puts
    /// them into a list, and then puts that list onto the stack. 
    pub fn core_c(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "c requires two arguments");
            return 0;
        }

        let regex_rr = self.stack.pop().unwrap();
        let regex_str_rr_opt = to_string_value(regex_rr);
        if regex_str_rr_opt.is_none() {
            print_error(chunk, i, "regex must be a string");
            return 0;
        }
        let regex_str_rr = regex_str_rr_opt.unwrap();

        {
            let mut regex_str_rrb = regex_str_rr.borrow_mut();
            let res = regex_str_rrb.gen_regex(chunk, i);
            if !res {
                return 0;
            }
        }
        let regex_str_rrb = regex_str_rr.borrow();
        let regex_opt = regex_str_rrb.to_regex();

        let str_rr = self.stack.pop().unwrap();
        let str_rrb = str_rr.borrow();
        let str_pre = str_rrb.to_string();
        let str_opt = to_string_2(&str_pre);

        match (regex_opt, str_opt) {
            (Some(regex), Some(s)) => {
                let captures = regex.captures_iter(s);
                let mut lst = VecDeque::new();
                for capture in captures {
                    lst.push_back(Rc::new(RefCell::new(Value::String(
                        capture.get(0).unwrap().as_str().to_string(),
                        None,
                    ))));
                }
                self.stack.push(Rc::new(RefCell::new(Value::List(lst))));
            }
            (_, Some(_)) => {
                print_error(chunk, i, "first c argument must be string");
                return 0;
            }
            (_, _) => {
                print_error(chunk, i, "second c argument must be string");
                return 0;
            }
        }
        return 1;
    }
}

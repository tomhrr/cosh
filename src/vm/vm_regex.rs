use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use regex::Regex;

use chunk::Value;
use vm::*;

lazy_static! {
    static ref RE_ADJUST: Regex = Regex::new(r"\\([\d+])").unwrap();
}

impl VM {
    /// Takes a value that can be stringified and a regex string as
    /// its arguments.  Tests whether the value matches as against the
    /// regex and puts a boolean onto the stack accordingly.
    pub fn core_m(&mut self, interner: &mut StringInterner) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("m requires two arguments");
            return 0;
        }

        let regex_rr = self.stack.pop().unwrap();
        let regex_opt = self.intern_regex(interner, regex_rr);

        let str_rr = self.stack.pop().unwrap();
        let str_opt = self.intern_string_value(interner, str_rr);

        match (regex_opt, str_opt) {
            (Some(regex), Some(ss)) => {
                let s = self.interner_resolve(interner, ss);
                let res = if regex.borrow().is_match(s) { 1 } else { 0 };
                self.stack.push(Value::Int(res));
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
    pub fn core_s(&mut self, interner: &mut StringInterner) -> i32 {
        if self.stack.len() < 3 {
            self.print_error("s requires three arguments");
            return 0;
        }

        let repl_str_rr = self.stack.pop().unwrap();
        let repl_str_opt = self.intern_string_value(interner, repl_str_rr);

        let regex_rr = self.stack.pop().unwrap();
        let regex_opt = self.intern_regex(interner, regex_rr);

        let str_rr = self.stack.pop().unwrap();
        let str_opt = self.intern_string_value(interner, str_rr);

        match (repl_str_opt, regex_opt, str_opt) {
            (Some(repl_strs), Some(regex), Some(ss)) => {
                let repl_str =
                    self.interner_resolve(interner, repl_strs).to_string();
                let s =
                    self.interner_resolve(interner, ss).to_string();
                
                let updated_repl = RE_ADJUST.replace_all(&repl_str, "$${$1}");
                let updated_repl_str = updated_repl.to_string();
                let updated_str = regex.borrow().replace_all(&s, &updated_repl_str[..]);
                let c = self.intern_string_to_value(interner, &updated_str);
                self.stack.push(c);
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
    pub fn core_c(&mut self, interner: &mut StringInterner) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("c requires two arguments");
            return 0;
        }

        let regex_rr = self.stack.pop().unwrap();
        let regex_opt = self.intern_regex(interner, regex_rr);

        let str_rr = self.stack.pop().unwrap();
        let str_opt = self.intern_string_value(interner, str_rr);

        match (regex_opt, str_opt) {
            (Some(regex), Some(ss)) => {
                let s = self.interner_resolve(interner, ss).to_string();
                let rb = regex.borrow();
                let captures = rb.captures_iter(&s);
                let mut lst = VecDeque::new();
                for capture in captures {
                    let yy = self.intern_string_to_value(interner, 
                        capture.get(0).unwrap().as_str()
                    );
                    lst.push_back(yy);
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

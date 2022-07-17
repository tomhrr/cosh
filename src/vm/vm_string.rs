use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use regex::Regex;

use chunk::{print_error, Chunk, StringPair, Value};
use vm::*;

impl VM {
    /// Takes two string arguments, appends them together, and adds
    /// the resulting string back onto the stack.
    pub fn core_append(&mut self, chunk: Rc<Chunk>, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk.clone(), i, "append requires two arguments");
            return 0;
        }

        let v1 = self.stack.pop().unwrap();
        let v2 = self.stack.pop().unwrap();

        let v1_str_s;
        let v1_str_b;
        let v1_str_str;
        let v1_str_bk: Option<String>;
        let v1_str_opt: Option<&str> = match v1 {
            Value::String(sp) => {
                v1_str_s = sp;
                v1_str_b = v1_str_s.borrow();
                Some(&v1_str_b.s)
            }
            _ => {
                v1_str_bk = v1.to_string();
                match v1_str_bk {
                    Some(s) => {
                        v1_str_str = s;
                        Some(&v1_str_str)
                    }
                    _ => None,
                }
            }
        };

        let v2_str_s;
        let v2_str_b;
        let v2_str_str;
        let v2_str_bk: Option<String>;
        let v2_str_opt: Option<&str> = match v2 {
            Value::String(sp) => {
                v2_str_s = sp;
                v2_str_b = v2_str_s.borrow();
                Some(&v2_str_b.s)
            }
            _ => {
                v2_str_bk = v2.to_string();
                match v2_str_bk {
                    Some(s) => {
                        v2_str_str = s;
                        Some(&v2_str_str)
                    }
                    _ => None,
                }
            }
        };

        match (v1_str_opt, v2_str_opt) {
            (Some(s1), Some(s2)) => {
                let s3 = format!("{}{}", s2, s1);
                self.stack
                    .push(Value::String(Rc::new(RefCell::new(StringPair::new(
                        s3, None,
                    )))));
            }
            (Some(_), _) => {
                print_error(chunk.clone(), i, "second append argument must be string");
                return 0;
            }
            (_, _) => {
                print_error(chunk.clone(), i, "first append argument must be string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a string and a separator as its arguments.  Splits the
    /// string using the separator, treated as a regex, and puts the
    /// resulting list onto the stack.
    pub fn core_splitr(&mut self, chunk: Rc<Chunk>, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk.clone(), i, "split requires two arguments");
            return 0;
        }

        let regex_rr = self.stack.pop().unwrap();
        let list_str_rr = self.stack.pop().unwrap();

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

        let list_str_s;
        let list_str_b;
        let list_str_str;
        let list_str_bk: Option<String>;
        let list_str_opt: Option<&str> = match list_str_rr {
            Value::String(sp) => {
                list_str_s = sp;
                list_str_b = list_str_s.borrow();
                Some(&list_str_b.s)
            }
            _ => {
                list_str_bk = list_str_rr.to_string();
                match list_str_bk {
                    Some(s) => {
                        list_str_str = s;
                        Some(&list_str_str)
                    }
                    _ => None,
                }
            }
        };

        match (regex_opt, list_str_opt) {
            (Some(regex), Some(list_str)) => {
                let elements = regex.split(list_str);
                let mut final_elements = VecDeque::new();
                for e in elements {
                    final_elements.push_back(
                        Value::String(Rc::new(RefCell::new(StringPair::new(
                            e.to_string(),
                            None,
                        ))))
                    );
                }
                self.stack.push(Value::List(Rc::new(RefCell::new(final_elements))));
            }
            (Some(_), _) => {
                print_error(chunk.clone(), i, "first splitr argument must be string");
                return 0;
            }
            _ => {
                print_error(chunk.clone(), i, "second splitr argument must be string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a string and a separator as its arguments.  Splits the
    /// string using the separator, and puts the resulting list onto
    /// the stack.  Quotation by way of the double-quote character is
    /// taken into account.
    pub fn core_split(&mut self, chunk: Rc<Chunk>, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk.clone(), i, "split requires two arguments");
            return 0;
        }

        let separator_rr = self.stack.pop().unwrap();
        let list_str_rr = self.stack.pop().unwrap();

        let separator_s;
        let separator_b;
        let separator_str;
        let separator_bk: Option<String>;
        let separator_opt: Option<&str> = match separator_rr {
            Value::String(sp) => {
                separator_s = sp;
                separator_b = separator_s.borrow();
                Some(&separator_b.s)
            }
            _ => {
                separator_bk = separator_rr.to_string();
                match separator_bk {
                    Some(s) => {
                        separator_str = s;
                        Some(&separator_str)
                    }
                    _ => None,
                }
            }
        };

        let list_str_s;
        let list_str_b;
        let list_str_str;
        let list_str_bk: Option<String>;
        let list_str_opt: Option<&str> = match list_str_rr {
            Value::String(sp) => {
                list_str_s = sp;
                list_str_b = list_str_s.borrow();
                Some(&list_str_b.s)
            }
            _ => {
                list_str_bk = list_str_rr.to_string();
                match list_str_bk {
                    Some(s) => {
                        list_str_str = s;
                        Some(&list_str_str)
                    }
                    _ => None,
                }
            }
        };

        match (separator_opt, list_str_opt) {
            (Some(separator), Some(list_str)) => {
                let elements = list_str.split(separator);
                // The final set of separated elements.
                let mut final_elements = Vec::new();
                // A list containing a partially-complete element, if
                // applicable.
                let mut buffer = Vec::new();
                for e in elements {
                    let mut e_str = e.to_string();
                    if buffer.len() > 0 {
                        if e_str.len() > 0 {
                            if e_str.chars().last().unwrap() == '"' {
                                buffer.push(e_str);
                                let mut new_str = buffer.join(separator);
                                if new_str.len() > 0 {
                                    if new_str.chars().next().unwrap() == '"' {
                                        new_str.remove(0);
                                    }
                                    if new_str.len() > 0 {
                                        if new_str.chars().last().unwrap() == '"' {
                                            new_str.remove(new_str.len() - 1);
                                        }
                                    }
                                }
                                final_elements.push(new_str.to_string());
                                buffer.clear();
                            } else {
                                buffer.push(e_str);
                            }
                        }
                    } else if (e_str.len() > 0)
                                && (e_str.chars().next().unwrap() == '"')
                                && (e_str.chars().last().unwrap() != '"') {
                        buffer.push(e_str);
                    } else {
                        if e_str.len() > 0 {
                            if e_str.chars().next().unwrap() == '"' {
                                e_str.remove(0);
                            }
                            if e_str.len() > 0 {
                                if e_str.chars().last().unwrap() == '"' {
                                    e_str.remove(e_str.len() - 1);
                                }
                            }
                        }
                        final_elements.push(e_str);
                    }
                }
                if buffer.len() > 0 {
                    print_error(chunk.clone(), i, "error in string syntax in split");
                    return 0;
                }

                let mut lst = VecDeque::new();
                for e in final_elements.iter() {
                    lst.push_back(Value::String(Rc::new(RefCell::new(StringPair::new(
                        e.to_string(),
                        None,
                    )))));
                }
                self.stack.push(Value::List(Rc::new(RefCell::new(lst))));
            }
            (Some(_), _) => {
                print_error(chunk.clone(), i, "first split argument must be string");
                return 0;
            }
            _ => {
                print_error(chunk.clone(), i, "second split argument must be string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a shiftable object and a separator as its arguments.
    /// Joins the elements retrieved from the shiftable object by
    /// using the separator string between the elements, and puts the
    /// resulting joined string onto the stack.
    pub fn core_join(
        &mut self,
        scopes: &mut Vec<Rc<RefCell<HashMap<String, Value>>>>,
        global_functions: &mut HashMap<String, Rc<Chunk>>,
        prev_localvarstacks: &mut Vec<Rc<RefCell<Vec<Value>>>>,
        chunk: Rc<Chunk>,
        i: usize,
        line_col: (u32, u32),
        running: Arc<AtomicBool>,
    ) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk.clone(), i, "join requires two arguments");
            return 0;
        }

        let separator_rr = self.stack.pop().unwrap();
        let separator_s;
        let separator_b;
        let separator_str;
        let separator_bk: Option<String>;
        let separator_opt: Option<&str> = match separator_rr {
            Value::String(sp) => {
                separator_s = sp;
                separator_b = separator_s.borrow();
                Some(&separator_b.s)
            }
            _ => {
                separator_bk = separator_rr.to_string();
                match separator_bk {
                    Some(s) => {
                        separator_str = s;
                        Some(&separator_str)
                    }
                    _ => None,
                }
            }
        };

        let esc_quotes = Regex::new(r#"""#).unwrap();

        match separator_opt {
            Some(separator) => {
                // If the separator is an empty string, then matching
                // it against the values to determine whether they
                // need quoting won't work, so skip that in that case.
                let separator_is_empty_string = separator.len() == 0;
                let separator_regex_res = Regex::new(separator);
                let mut final_elements = Vec::new();
                match separator_regex_res {
                    Ok(separator_regex) => loop {
                        let dup_res = self.opcode_dup(chunk.clone(), i);
                        if dup_res == 0 {
                            return 0;
                        }
                        let shift_res = self.opcode_shift(
                            scopes,
                            global_functions,
                            prev_localvarstacks,
                            chunk.clone(),
                            i,
                            line_col,
                            running.clone(),
                        );
                        if shift_res == 0 {
                            return 0;
                        }
                        let element_rr = self.stack.pop().unwrap();
                        match element_rr {
                            Value::Null => {
                                break;
                            }
                            Value::String(sp) => {
                                if !separator_is_empty_string
                                    && (separator_regex.is_match(&sp.borrow().s)
                                        || esc_quotes.is_match(&sp.borrow().s))
                                {
                                    let s1 = &sp.borrow();
                                    let s2 = esc_quotes.replace_all(&s1.s, "\\\"");
                                    final_elements.push(format!("\"{}\"", s2));
                                } else {
                                    final_elements.push(sp.borrow().s.to_string());
                                }
                            }
                            _ => {
                                let element_s;
                                let element_b;
                                let element_str;
                                let element_bk: Option<String>;
                                let element_opt: Option<&str> = match element_rr {
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

                                match element_opt {
                                    Some(s) => {
                                        if !separator_is_empty_string
                                            && (separator_regex.is_match(&s)
                                                || esc_quotes.is_match(&s))
                                        {
                                            let s2 = esc_quotes.replace_all(s, "\\\"");
                                            final_elements.push(format!("\"{}\"", s2));
                                        } else {
                                            final_elements.push(s.to_string());
                                        }
                                    }
                                    _ => {
                                        print_error(chunk.clone(), i, "cannot join non-string");
                                        return 0;
                                    }
                                }
                            }
                        }
                    },
                    Err(_) => {
                        print_error(chunk.clone(), i, "invalid separator regular expression");
                        return 0;
                    }
                }
                let drop_res = self.opcode_drop(chunk, i);
                if drop_res == 0 {
                    return 0;
                }
                let final_str = final_elements.join(separator);
                self.stack
                    .push(Value::String(Rc::new(RefCell::new(StringPair::new(
                        final_str, None,
                    )))));
            }
            _ => {
                print_error(chunk.clone(), i, "second join argument must be string");
                return 0;
            }
        }
        return 1;
    }
}

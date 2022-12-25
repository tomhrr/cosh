use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use regex::Regex;

use chunk::{StringPair, Value};
use vm::*;

impl VM {
    /// Takes two string/list arguments, appends them together, and
    /// adds the resulting string/list back onto the stack.
    pub fn core_append(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("++ requires two arguments");
            return 0;
        }

        let v2 = self.stack.pop().unwrap();
        let mut v1 = self.stack.pop().unwrap();

        if v1.is_generator() && v2.is_generator() {
            match v1 {
                Value::MultiGenerator(ref mut genlist) => {
                    genlist.borrow_mut().push_back(v2);
                    self.stack.push(v1);
                }
                _ => {
                    let mut genlist = VecDeque::new();
                    genlist.push_back(v1);
                    genlist.push_back(v2);
                    let mg =
                        Value::MultiGenerator(
                            Rc::new(RefCell::new(genlist))
                        );
                    self.stack.push(mg);
                }
            }
        } else {
            match (v1.clone(), v2.clone()) {
                (Value::List(lst1_ref), Value::List(lst2_ref)) => {
                    let mut lst = lst1_ref.borrow().clone();
                    for k in lst2_ref.borrow().iter() {
                        lst.push_back(k.clone());
                    }
                    let res = Value::List(Rc::new(RefCell::new(lst)));
                    self.stack.push(res);
                }
                (Value::Hash(hs1_ref), Value::Hash(hs2_ref)) => {
                    let mut hsh = hs1_ref.borrow().clone();
                    for (k, v) in hs2_ref.borrow().iter() {
                        hsh.insert(k.clone(), v.clone());
                    }
                    let res = Value::Hash(Rc::new(RefCell::new(hsh)));
                    self.stack.push(res);
                }
                (_, _) => {
                    if v1.is_generator() && v2.is_generator() {
                    } else {
                        let v1_str_opt: Option<&str>;
                        to_str!(v1, v1_str_opt);
                        let v2_str_opt: Option<&str>;
                        to_str!(v2, v2_str_opt);

                        match (v1_str_opt, v2_str_opt) {
                            (Some(s1), Some(s2)) => {
                                let s3 = format!("{}{}", s1, s2);
                                self.stack
                                    .push(Value::String(Rc::new(RefCell::new(StringPair::new(
                                        s3, None,
                                    )))));
                            }
                            (Some(_), _) => {
                                self.print_error("second append argument must be string");
                                return 0;
                            }
                            (_, _) => {
                                self.print_error("first append argument must be string");
                                return 0;
                            }
                        }
                    }
                }
            }
        }
        return 1;
    }

    /// Takes a string and a separator as its arguments.  Splits the
    /// string using the separator, treated as a regex, and puts the
    /// resulting list onto the stack.
    pub fn core_splitr(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("split requires two arguments");
            return 0;
        }

        let regex_rr = self.stack.pop().unwrap();
        let regex_opt = self.gen_regex(regex_rr);
        if regex_opt.is_none() {
            return 0;
        }
        let list_str_rr = self.stack.pop().unwrap();

	let list_str_opt: Option<&str>;
	to_str!(list_str_rr, list_str_opt);

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
                self.print_error("first splitr argument must be string");
                return 0;
            }
            _ => {
                self.print_error("second splitr argument must be string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a string and a separator as its arguments.  Splits the
    /// string using the separator, and puts the resulting list onto
    /// the stack.  Quotation by way of the double-quote character is
    /// taken into account.
    pub fn core_split(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("split requires two arguments");
            return 0;
        }

        let separator_rr = self.stack.pop().unwrap();
        let list_str_rr = self.stack.pop().unwrap();

	let separator_opt: Option<&str>;
	to_str!(separator_rr, separator_opt);

	let list_str_opt: Option<&str>;
	to_str!(list_str_rr, list_str_opt);

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
                    self.print_error("error in string syntax in split");
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
                self.print_error("first split argument must be string");
                return 0;
            }
            _ => {
                self.print_error("second split argument must be string");
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
    ) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("join requires two arguments");
            return 0;
        }

        let separator_rr = self.stack.pop().unwrap();
	let separator_opt: Option<&str>;
	to_str!(separator_rr, separator_opt);

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
                        let dup_res = self.opcode_dup();
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
                                        self.print_error("cannot join non-string");
                                        return 0;
                                    }
                                }
                            }
                        }
                    },
                    Err(_) => {
                        self.print_error("invalid separator regular expression");
                        return 0;
                    }
                }
                let drop_res = self.opcode_drop();
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
                self.print_error("second join argument must be string");
                return 0;
            }
        }
        return 1;
    }
}

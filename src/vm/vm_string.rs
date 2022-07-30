use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use regex::Regex;

use chunk::Value;
use vm::*;

impl VM {
    /// Takes two string arguments, appends them together, and adds
    /// the resulting string back onto the stack.
    pub fn core_append(&mut self, interner: &mut StringInterner) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("append requires two arguments");
            return 0;
        }

        let v1 = self.stack.pop().unwrap();
        let v2 = self.stack.pop().unwrap();

        let v1_str_opt = self.intern_string_value(interner, v1);
        let v2_str_opt = self.intern_string_value(interner, v2);

        match (v1_str_opt, v2_str_opt) {
            (Some(s1s), Some(s2s)) => {
                let s1 = self.interner_resolve(interner,
                    s1s).to_string();
                let s2 = self.interner_resolve(interner, s2s);
                let s3 = format!("{}{}", s2, s1);
                let c = self.intern_string_to_value(interner, &s3);
                self.stack.push(c);
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
        return 1;
    }

    /// Takes a string and a separator as its arguments.  Splits the
    /// string using the separator, treated as a regex, and puts the
    /// resulting list onto the stack.
    pub fn core_splitr(&mut self, interner: &mut StringInterner) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("split requires two arguments");
            return 0;
        }

        let regex_rr = self.stack.pop().unwrap();
        let list_str_rr = self.stack.pop().unwrap();

        let regex_opt = self.intern_regex(interner, regex_rr);
        let list_str_opt = self.intern_string_value(interner, list_str_rr);

        match (regex_opt, list_str_opt) {
            (Some(regex), Some(list_strs)) => {
                let list_str =
                    self.interner_resolve(interner, list_strs).to_string();
                //let ss = list_str.to_string();
                let rb = regex.borrow();
                let elements = rb.split(&list_str);
                let mut final_elements = VecDeque::new();
                for e in elements {
                    let s = e.clone().to_string();
                    let v = self.intern_string_to_value(interner, &s);
                    final_elements.push_back(v);
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
    pub fn core_split(&mut self, interner: &mut StringInterner) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("split requires two arguments");
            return 0;
        }

        let separator_rr = self.stack.pop().unwrap();
        let list_str_rr = self.stack.pop().unwrap();

        let separator_opt = self.intern_string_value(interner, separator_rr);
        let list_str_opt = self.intern_string_value(interner, list_str_rr);

        match (separator_opt, list_str_opt) {
            (Some(separators), Some(list_strs)) => {
                let separator =
                    self.interner_resolve(interner,
                        separators).to_string();
                let list_str =
                    self.interner_resolve(interner, list_strs);
                let elements = list_str.split(&separator);
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
                                let mut new_str = buffer.join(&separator);
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
                    lst.push_back(
                        self.intern_string_to_value(interner, &e.to_string())
                    );
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
        interner: &mut StringInterner,
    ) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("join requires two arguments");
            return 0;
        }

        let separator_rr = self.stack.pop().unwrap();
        let separator_opt = self.intern_string_value(interner, separator_rr);

        let esc_quotes = Regex::new(r#"""#).unwrap();

        match separator_opt {
            Some(separators) => {
                let separator =
                    self.interner_resolve(interner, separators).to_string();
                // If the separator is an empty string, then matching
                // it against the values to determine whether they
                // need quoting won't work, so skip that in that case.
                let separator_is_empty_string = separator.len() == 0;
                let separator_regex_res = Regex::new(&separator);
                let mut final_elements = Vec::new();
                match separator_regex_res {
                    Ok(separator_regex) => loop {
                        let dup_res = self.opcode_dup(interner);
                        if dup_res == 0 {
                            return 0;
                        }
                        let shift_res = self.opcode_shift(interner);
                        if shift_res == 0 {
                            return 0;
                        }
                        let element_rr = self.stack.pop().unwrap();
                        match element_rr {
                            Value::Null => {
                                break;
                            }
                            Value::String(sp) => {
                                let s = self.interner_resolve(interner, sp);
                                if !separator_is_empty_string
                                    && (separator_regex.is_match(s)
                                        || esc_quotes.is_match(s))
                                {
                                    let s2 = esc_quotes.replace_all(s, "\\\"");
                                    final_elements.push(format!("\"{}\"", s2));
                                } else {
                                    final_elements.push(s.to_string());
                                }
                            }
                            _ => {
                                let element_opt =
                                    self.intern_string_value(interner, element_rr);

                                match element_opt {
                                    Some(ss) => {
                                        let s = self.interner_resolve(interner, ss);
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
                let drop_res = self.opcode_drop(interner);
                if drop_res == 0 {
                    return 0;
                }
                let final_str = final_elements.join(&separator);
                let c = self.intern_string_to_value(interner, &final_str);
                self.stack.push(c);
            }
            _ => {
                self.print_error("second join argument must be string");
                return 0;
            }
        }
        return 1;
    }
}

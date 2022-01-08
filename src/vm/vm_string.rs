use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use lazy_static::lazy_static;
use regex::Regex;

use chunk::{print_error, Chunk, Value};
use vm::*;

lazy_static! {
    static ref START_QUOTE: Regex = Regex::new(r#"^\s*""#).unwrap();
    static ref END_QUOTE: Regex = Regex::new(r#""\s*$"#).unwrap();
}

impl VM {
    /// Takes two string arguments, appends them together, and adds
    /// the resulting string back onto the stack.
    pub fn core_append(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "append requires two arguments");
            return 0;
        }

        let v1 = self.stack.pop().unwrap();
        let v1_b = v1.borrow();
        let v2 = self.stack.pop().unwrap();
        let v2_b = v2.borrow();

        let v1_str_pre = v1_b.to_string();
        let v1_str_opt = to_string_2(&v1_str_pre);
        let v2_str_pre = v2_b.to_string();
        let v2_str_opt = to_string_2(&v2_str_pre);

        match (v1_str_opt, v2_str_opt) {
            (Some(s1), Some(s2)) => {
                let s3 = format!("{}{}", s2, s1);
                self.stack
                    .push(Rc::new(RefCell::new(Value::String(s3, None))));
            }
            (Some(_), _) => {
                print_error(chunk, i, "second append argument must be string");
                return 0;
            }
            (_, _) => {
                print_error(chunk, i, "first append argument must be string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a string and a separator as its arguments.  Splits the
    /// string using the separator, and puts the resulting list onto
    /// the stack.  Quotation by way of the double-quote character is
    /// taken into account.
    pub fn core_split(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "split requires two arguments");
            return 0;
        }

        let separator_rr = self.stack.pop().unwrap();
        let separator_rrb = separator_rr.borrow();
        let list_str_rr = self.stack.pop().unwrap();
        let list_str_rrb = list_str_rr.borrow();

        let separator_pre = separator_rrb.to_string();
        let separator_opt = to_string_2(&separator_pre);
        let list_str_pre = list_str_rrb.to_string();
        let list_str_opt = to_string_2(&list_str_pre);

        match (separator_opt, list_str_opt) {
            (Some(separator), Some(list_str)) => {
                let elements = list_str.split(separator);
                // The final set of separated elements.
                let mut final_elements = Vec::new();
                // A list containing a partially-complete element, if
                // applicable.
                let mut buffer = Vec::new();
                for e in elements {
                    let e_str = e.to_string();
                    if buffer.len() > 0 {
                        if e_str.len() > 0 {
                            if e_str.chars().last().unwrap() == '"' {
                                buffer.push(e_str);
                                let new_str = buffer.join(separator);
                                let new_str2 = START_QUOTE.replace(&new_str, "");
                                let new_str3 = END_QUOTE.replace(&new_str2, "");
                                final_elements.push(new_str3.to_string());
                                buffer.clear();
                            } else {
                                buffer.push(e_str);
                            }
                        }
                    } else if START_QUOTE.is_match(&e_str) && !END_QUOTE.is_match(&e_str) {
                        buffer.push(e_str);
                    } else {
                        let new_str = START_QUOTE.replace(&e_str, "");
                        let new_str2 = END_QUOTE.replace(&new_str, "");
                        final_elements.push(new_str2.to_string());
                    }
                }
                if buffer.len() > 0 {
                    print_error(chunk, i, "error in string syntax in split");
                    return 0;
                }

                let mut lst = VecDeque::new();
                for e in final_elements.iter() {
                    lst.push_back(Rc::new(RefCell::new(Value::String(
                        e.to_string(),
                        None,
                    ))));
                }
                self.stack.push(Rc::new(RefCell::new(Value::List(lst))));
            }
            (Some(_), _) => {
                print_error(chunk, i, "first split argument must be string");
                return 0;
            }
            _ => {
                print_error(chunk, i, "second split argument must be string");
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
        scopes: &mut Vec<RefCell<HashMap<String, Rc<RefCell<Value>>>>>,
        global_functions: &mut RefCell<HashMap<String, Chunk>>,
        prev_localvarstacks: &mut Vec<Rc<RefCell<Vec<Rc<RefCell<Value>>>>>>,
        chunk: &Chunk, i: usize, line_col: (u32, u32),
        running: Arc<AtomicBool>,
    ) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "join requires two arguments");
            return 0;
        }

        let separator_rr = self.stack.pop().unwrap();
        let separator_rrb = separator_rr.borrow();
        let separator_pre = separator_rrb.to_string();
        let separator_opt = to_string_2(&separator_pre);

        let esc_quotes = Regex::new(r#"""#).unwrap();

        match separator_opt {
            Some(separator) => {
                let separator_regex_res = Regex::new(separator);
                let mut final_elements = Vec::new();
                match separator_regex_res {
                    Ok(separator_regex) => loop {
                        let dup_res = self.opcode_dup(chunk, i);
                        if dup_res == 0 {
                            return 0;
                        }
                        let shift_res = self.opcode_shift(
                            scopes,
                            global_functions,
                            prev_localvarstacks,
                            chunk,
                            i,
                            line_col,
                            running.clone(),
                        );
                        if shift_res == 0 {
                            return 0;
                        }
                        let element_rr = self.stack.pop().unwrap();
                        let element_rrb = element_rr.borrow();
                        match &*element_rrb {
                            Value::Null => {
                                break;
                            }
                            Value::String(s, _) => {
                                let s2 = esc_quotes.replace_all(s, "\\\"");
                                if separator_regex.is_match(&s) || esc_quotes.is_match(&s) {
                                    final_elements.push(format!("\"{}\"", s2));
                                } else {
                                    final_elements.push(s2.to_string());
                                }
                            }
                            _ => {
                                let element_pre = element_rrb.to_string();
                                let element_opt = to_string_2(&element_pre);
                                match element_opt {
                                    Some(s) => {
                                        let s2 = esc_quotes.replace_all(s, "\\\"");
                                        if separator_regex.is_match(&s)
                                            || esc_quotes.is_match(&s)
                                        {
                                            final_elements
                                                .push(format!("\"{}\"", s2));
                                        } else {
                                            final_elements.push(s2.to_string());
                                        }
                                    }
                                    _ => {
                                        print_error(
                                            chunk,
                                            i,
                                            "cannot join non-string",
                                        );
                                        return 0;
                                    }
                                }
                            }
                        }
                    },
                    Err(_) => {
                        print_error(
                            chunk,
                            i,
                            "invalid separator regular expression",
                        );
                        return 0;
                    }
                }
                let drop_res = self.opcode_drop(chunk, i);
                if drop_res == 0 {
                    return 0;
                }
                let final_str = final_elements.join(separator);
                self.stack
                    .push(Rc::new(RefCell::new(Value::String(final_str, None))));
            }
            _ => {
                print_error(chunk, i, "second join argument must be string");
                return 0;
            }
        }
        return 1;
    }
}

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use lazy_static::lazy_static;
use regex::Regex;

use crate::chunk::Value;
use crate::vm::*;

lazy_static! {
    static ref CAPTURE_NUM: Regex = Regex::new("\\{(\\d+)\\}").unwrap();
    static ref CAPTURE_WITHOUT_NUM: Regex = Regex::new("\\{\\}").unwrap();
}

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

        if v1.is_shiftable() && v2.is_shiftable() {
            match v1 {
                Value::MultiGenerator(ref mut genlist) => {
                    match v2 {
                        Value::MultiGenerator(ref genlist_v2) => {
                            if Rc::ptr_eq(genlist, genlist_v2) {
                                self.print_error("++ cannot be used to append generator to itself");
                                return 0;
                            }
                        }
                        _ => {}
                    }
                    genlist.borrow_mut().push_back(v2);
                    self.stack.push(v1);
                }
                _ => {
                    let mut genlist = VecDeque::new();
                    genlist.push_back(v1);
                    genlist.push_back(v2);
                    let mg = Value::MultiGenerator(Rc::new(RefCell::new(genlist)));
                    self.stack.push(mg);
                }
            }
        } else {
            match (v1.clone(), v2.clone()) {
                (Value::Hash(hs1_ref), Value::Hash(hs2_ref)) => {
                    let mut hsh = hs1_ref.borrow().clone();
                    for (k, v) in hs2_ref.borrow().iter() {
                        hsh.insert(k.clone(), v.clone());
                    }
                    let res = Value::Hash(Rc::new(RefCell::new(hsh)));
                    self.stack.push(res);
                }
                (_, _) => {
                    let v1_str_opt: Option<&str>;
                    to_str!(v1, v1_str_opt);
                    let v2_str_opt: Option<&str>;
                    to_str!(v2, v2_str_opt);

                    match (v1_str_opt, v2_str_opt) {
                        (Some(s1), Some(s2)) => {
                            let s3 = format!("{}{}", s1, s2);
                            self.stack.push(new_string_value(s3));
                        }
                        (Some(_), _) => {
                            self.print_error("second ++ argument must be string");
                            return 0;
                        }
                        (_, _) => {
                            self.print_error("first ++ argument must be string");
                            return 0;
                        }
                    }
                }
            }
        }
        1
    }

    /// Takes a string and a separator as its arguments.  Splits the
    /// string using the separator, treated as a regex, and puts the
    /// resulting list onto the stack.
    pub fn core_splitr(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("splitr requires two arguments");
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
            (Some((regex, _)), Some(list_str)) => {
                let elements = regex.split(list_str);
                let mut final_elements = VecDeque::new();
                for e in elements {
                    final_elements.push_back(new_string_value(e.to_string()));
                }
                self.stack
                    .push(Value::List(Rc::new(RefCell::new(final_elements))));
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
        1
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
                /* The final set of separated elements. */
                let mut final_elements = Vec::new();
                /* A list containing a partially-complete element, if
                 * applicable. */
                let mut buffer = Vec::new();
                for e in elements {
                    let mut e_str = e.to_string();
                    if !buffer.is_empty() {
                        if !e_str.is_empty() {
                            if e_str.ends_with('\"') {
                                buffer.push(e_str);
                                let mut new_str = buffer.join(separator);
                                if !new_str.is_empty() {
                                    if new_str.starts_with('\"') {
                                        new_str.remove(0);
                                    }
                                    if !new_str.is_empty() && new_str.ends_with('\"') {
                                        new_str.remove(new_str.len() - 1);
                                    }
                                }
                                final_elements.push(new_str.to_string());
                                buffer.clear();
                            } else {
                                buffer.push(e_str);
                            }
                        }
                    } else if !e_str.is_empty() && e_str.starts_with('\"') && !e_str.ends_with('\"')
                    {
                        buffer.push(e_str);
                    } else {
                        if !e_str.is_empty() {
                            if e_str.starts_with('\"') {
                                e_str.remove(0);
                            }
                            if !e_str.is_empty() && e_str.ends_with('\"') {
                                e_str.remove(e_str.len() - 1);
                            }
                        }
                        final_elements.push(e_str);
                    }
                }
                if !buffer.is_empty() {
                    self.print_error("first split argument has syntax error");
                    return 0;
                }

                let mut lst = VecDeque::new();
                for e in final_elements.iter() {
                    lst.push_back(new_string_value(e.to_string()));
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
        1
    }

    /// Takes a shiftable object and a separator as its arguments.
    /// Joins the elements retrieved from the shiftable object by
    /// using the separator string between the elements, and puts the
    /// resulting joined string onto the stack.
    pub fn core_join(&mut self) -> i32 {
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
                let separator_is_empty_string = separator.is_empty();
                let separator_regex_res = Regex::new(separator);
                let mut final_elements = Vec::new();
                match separator_regex_res {
                    Ok(separator_regex) => {
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
                            match element_rr {
                                Value::Null => {
                                    break;
                                }
                                Value::String(st) => {
                                    if !separator_is_empty_string
                                        && (separator_regex.is_match(&st.borrow().string)
                                            || esc_quotes.is_match(&st.borrow().string))
                                    {
                                        let s1 = &st.borrow();
                                        let s2 = esc_quotes.replace_all(&s1.string, "\\\"");
                                        final_elements.push(format!("\"{}\"", s2));
                                    } else {
                                        final_elements.push(st.borrow().string.to_string());
                                    }
                                }
                                _ => {
                                    let element_s;
                                    let element_b;
                                    let element_str;
                                    let element_bk: Option<String>;
                                    let element_opt: Option<&str> = match element_rr {
                                        Value::String(st) => {
                                            element_s = st;
                                            element_b = element_s.borrow();
                                            Some(&element_b.string)
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
                                            self.print_error("first join argument must be a generator over strings");
                                            return 0;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {
                        self.print_error(
                            "second join argument must be valid separator regular expression",
                        );
                        return 0;
                    }
                }
                let drop_res = self.opcode_drop();
                if drop_res == 0 {
                    return 0;
                }
                let final_str = final_elements.join(separator);
                self.stack.push(new_string_value(final_str));
            }
            _ => {
                self.print_error("second join argument must be string");
                return 0;
            }
        }
        1
    }

    /// Takes a format string as its argument, and substitutes into
    /// that string values from the stack, as required.
    pub fn core_fmt(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("fmt requires one argument");
            return 0;
        }

        let str_rr = self.stack.pop().unwrap();
        let str_opt: Option<&str>;
        to_str!(str_rr, str_opt);

        match str_opt {
            Some(s) => {
                let mut updated_str = String::new();
                let mut escaping = false;
                let mut in_substitution = false;
                let mut substitution_index: Option<usize> = None;
                let mut stack_cache = Vec::<Option<String>>::new();
                for c in s.chars() {
                    if c == '\\' {
                        if escaping {
                            updated_str.push('\\');
                            escaping = false;
                        } else {
                            escaping = true;
                        }
                    } else if c == '{' {
                        if escaping {
                            updated_str.push('{');
                            escaping = false;
                        } else if in_substitution {
                            self.print_error("1 fmt string contains invalid element reference");
                            return 0;
                        } else {
                            in_substitution = true;
                        }
                    } else if c == '}' {
                        if escaping {
                            updated_str.push('}');
                            escaping = false;
                        } else if !in_substitution {
                            self.print_error("2 fmt string contains invalid element reference");
                            return 0;
                        } else {
                            in_substitution = false;
                            match substitution_index {
                                Some(n) => {
                                    if n >= self.stack.len() {
                                        self.print_error("fmt string contains invalid stack element reference");
                                        return 0;
                                    }
                                    let sc_opt = stack_cache.get(n);
                                    match sc_opt {
                                        Some(Some(sc)) => {
                                            updated_str.push_str(sc);
                                        }
                                        _ => {
                                            let capture_el_rr_opt = self.stack.get(self.stack.len() - 1 - n);
                                            match capture_el_rr_opt {
                                                Some(capture_el_rr) => {
                                                    let capture_el_str_opt: Option<&str>;
                                                    to_str!(capture_el_rr, capture_el_str_opt);
                                                    match capture_el_str_opt {
                                                        Some(capture_el_str) => {
                                                            stack_cache.resize(n as usize, None);
                                                            stack_cache.insert(n as usize, Some(capture_el_str.to_string()));
                                                            updated_str.push_str(capture_el_str);
                                                        }
                                                        _ => {
                                                            self.print_error("fmt string is not able to be parsed");
                                                            return 0;
                                                        }
                                                    }
                                                }
                                                None => {
                                                    self.print_error("fmt string contains invalid stack element reference");
                                                    return 0;
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    if self.stack.is_empty() {
                                        self.print_error("fmt string has exhausted stack");
                                        return 0;
                                    }
                                    let value_rr = self.stack.pop().unwrap();
                                    let value_opt: Option<&str>;
                                    to_str!(value_rr, value_opt);

                                    match value_opt {
                                        Some(s) => {
                                            updated_str.push_str(s);
                                        }
                                        _ => {
                                            self.print_error("fmt string is not able to be parsed");
                                            return 0;
                                        }
                                    }
                                }
                            }
                        }
                    } else if in_substitution {
                        let digit_opt = c.to_digit(10);
                        match digit_opt {
                            Some(n) => {
                                substitution_index = Some(n as usize);
                            }
                            _ => {
                                self.print_error("fmt index is not able to be parsed");
                                return 0;
                            }
                        }
                    } else {
                        if escaping {
                            updated_str.push('\\');
                        }
                        updated_str.push(c);
                        escaping = false;
                    }
                }
                self.stack.push(new_string_value(updated_str));
                1
            }
            _ => {
                self.print_error("fmt argument must be a string");
                0
            }
        }
    }
}

use std::cmp::Ordering;

use chunk::Value;
use vm::*;

impl VM {
    /// Sorts the elements of a list or generator using behaviour per
    /// the default cmp operation.
    pub fn core_sort(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("sort requires one argument");
            return 0;
        }

        let mut value_rr = self.stack.pop().unwrap();
        if value_rr.is_generator() {
            self.stack.push(value_rr);
            let res = self.generator_to_list();
            if res == 0 {
                return 0;
            }
            return self.core_sort();
        }

        match value_rr {
            Value::List(ref mut lst) => {
                let mut all_strings = true;
                for e in lst.borrow().iter() {
                    match e {
                        Value::String(_) => {}
                        _ => {
                            all_strings = false;
                            break;
                        }
                    }
                }

                /* If the list comprises strings only, then
                 * short-circuit the call to opcode_cmp_inner. */
                if all_strings {
                    lst.borrow_mut()
                        .make_contiguous()
                        .sort_by(|a, b| match (a, b) {
                            (Value::String(sp1), Value::String(sp2)) => {
                                let s1 = &(sp1.borrow().string);
                                let s2 = &(sp2.borrow().string);
                                s1.cmp(s2)
                            }
                            _ => {
                                eprintln!("expected string in sort!");
                                std::process::abort();
                            }
                        });
                } else {
                    let mut success = true;
                    lst.borrow_mut().make_contiguous().sort_by(|a, b| {
                        let res = self.opcode_cmp_inner(b, a);
                        if res == -2 {
                            success = false;
                            Ordering::Equal
                        } else if res == 1 {
                            Ordering::Greater
                        } else if res == 0 {
                            Ordering::Equal
                        } else {
                            Ordering::Less
                        }
                    });
                    if !success {
                        self.print_error("unable to sort elements");
                        return 0;
                    }
                }
            }
            _ => {
                self.print_error("unable to sort value");
                return 0;
            }
        }

        self.stack.push(value_rr);

        1
    }

    /// Sorts the elements of a list or generator using behaviour per
    /// the provided predicate.
    pub fn core_sortp(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("sortp requires two arguments");
            return 0;
        }

        let res = self.opcode_tofunction();
        if res == 0 {
            return 0;
        }

        let fn_rr = self.stack.pop().unwrap();

        let mut value_rr = self.stack.pop().unwrap();
        if value_rr.is_generator() {
            self.stack.push(value_rr);
            let res = self.generator_to_list();
            if res == 0 {
                return 0;
            }
            self.stack.push(fn_rr);
            return self.core_sortp();
        }

        match value_rr {
            Value::List(ref mut lst) => {
                let mut success = true;
                lst.borrow_mut().make_contiguous().sort_by(|a, b| {
                    if !success {
                        return Ordering::Equal;
                    }
                    self.stack.push(a.clone());
                    self.stack.push(b.clone());
                    let res = self.call(OpCode::Call, fn_rr.clone());
                    if !res {
                        success = false;
                        return Ordering::Equal;
                    }
                    if self.stack.is_empty() {
                        self.print_error("sortp predicate should return a value");
                        success = false;
                        return Ordering::Equal;
                    }
                    let ret = self.stack.pop().unwrap();
                    match ret {
                        Value::Int(n) => {
                            if n == -1 {
                                Ordering::Less
                            } else if n == 0 {
                                Ordering::Equal
                            } else if n == 1 {
                                Ordering::Greater
                            } else {
                                self.print_error("sortp predicate should return an int");
                                success = false;
                                Ordering::Equal
                            }
                        }
                        _ => {
                            self.print_error("sortp predicate should return an int");
                            success = false;
                            Ordering::Equal
                        }
                    }
                });
                if !success {
                    return 0;
                }
            }
            _ => {
                self.print_error("unable to sort value");
                return 0;
            }
        }

        self.stack.push(value_rr);

        1
    }
}

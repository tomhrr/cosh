use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

use chunk::Value;
use vm::*;

impl VM {
    pub fn core_sort(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("sort requires one argument");
            return 0;
        }

        let mut value_rr = self.stack.pop().unwrap();
        if value_rr.is_generator() {
            let mut lst = VecDeque::new();
            self.stack.push(value_rr);
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
                        self.stack.pop();
                        break;
                    },
                    _ => {
                        lst.push_back(element_rr);
                    }
                }
            }
            self.stack.push(Value::List(Rc::new(RefCell::new(lst))));
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

                if all_strings {
                    lst.borrow_mut().make_contiguous().sort_by(|a, b| {
                        match (a, b) {
                            (Value::String(sp1), Value::String(sp2)) => {
                                let s1 = &(sp1.borrow().s);
                                let s2 = &(sp2.borrow().s);
                                return s1.cmp(&s2);
                            }
                            _ => {
                                eprintln!("expected string in sort!");
                                std::process::abort();
                            }
                        }
                    });
                } else {
                    let mut success = true;
                    lst.borrow_mut().make_contiguous().sort_by(|a, b| {
                        let res = self.opcode_cmp_inner(a, b);
                        if res == -2 {
                            success = false;
                            return Ordering::Equal;
                        } else if res == 1 {
                            return Ordering::Greater;
                        } else if res == 0 {
                            return Ordering::Equal;
                        } else {
                            return Ordering::Less;
                        }
                    });
                    if !success {
                        eprintln!("unable to sort elements");
                        return 0;
                    }
                }
            }
            _ => {
                eprintln!("unable to sort value");
                return 0;
            }
        }

        self.stack.push(value_rr);

        return 1;
    }
}

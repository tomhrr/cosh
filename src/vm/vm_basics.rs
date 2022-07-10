use rand::Rng;

use chunk::{print_error, Chunk, StringPair, Value};
use vm::*;

impl VM {
    /// Remove the top element from the stack.
    pub fn opcode_drop(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() == 0 {
            print_error(chunk, i, "drop requires one argument");
            return 0;
        }
        self.stack.pop();
        return 1;
    }

    /// Remove all elements from the stack.
    #[allow(unused_variables)]
    pub fn opcode_clear(&mut self, chunk: &Chunk, i: usize) -> i32 {
        self.stack.clear();
        return 1;
    }

    /// Take the top element from the stack, duplicate it, and add it
    /// onto the stack.
    pub fn opcode_dup(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() == 0 {
            print_error(chunk, i, "dup requires one argument");
            return 0;
        }
        self.stack.push(self.stack.last().unwrap().clone());
        return 1;
    }

    /// Take the second element from the top from the stack, duplicate
    /// it, and add it onto the stack.
    pub fn opcode_over(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "over requires two arguments");
            return 0;
        }
        self.stack.push(self.stack[self.stack.len() - 2].clone());
        return 1;
    }

    /// Swap the top two elements from the stack.
    pub fn opcode_swap(&mut self, chunk: &Chunk, i: usize) -> i32 {
        let len = self.stack.len();
        if len < 2 {
            print_error(chunk, i, "swap requires two arguments");
            return 0;
        }
        self.stack.swap(len - 1, len - 2);
        return 1;
    }

    /// Rotate the top three elements from the stack: the top element
    /// becomes the second from top element, the second from top
    /// element becomes the third from top element, and the third from
    /// top element becomes the top element.
    pub fn opcode_rot(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 3 {
            print_error(chunk, i, "rot requires three arguments");
            return 0;
        }
        let first_rr = self.stack.pop().unwrap();
        let second_rr = self.stack.pop().unwrap();
        let third_rr = self.stack.pop().unwrap();
        self.stack.push(second_rr);
        self.stack.push(first_rr);
        self.stack.push(third_rr);
        return 1;
    }

    /// Push the current depth of the stack onto the stack.
    #[allow(unused_variables)]
    pub fn opcode_depth(&mut self, chunk: &Chunk, i: usize) -> i32 {
        self.stack.push(Value::Int(self.stack.len() as i32));
        return 1;
    }

    /// If the topmost element is a list, adds the length of that list
    /// onto the stack.  If the topmost element is a string, adds the
    /// length of that sting onto the stack.
    pub fn core_len(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "len requires one argument");
            return 0;
        }

        let lst_rr = self.stack.pop().unwrap();
        match lst_rr {
            Value::List(lst) => {
                let len = lst.borrow().len();
                self.stack.push(Value::Int(len as i32));
            }
            Value::String(sp) => {
                let len = sp.borrow().s.len();
                self.stack.push(Value::Int(len as i32));
            }
            _ => {
                print_error(chunk, i, "len argument must be a list or a string");
                return 0;
            }
        }
        return 1;
    }

    /// Adds a boolean onto the stack indicating whether the topmost
    /// element is a null value.
    pub fn opcode_isnull(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "is-null requires one argument");
            return 0;
        }

        let i1_rr = self.stack.pop().unwrap();
        let is_null = match i1_rr {
            Value::Null => 1,
            _ => 0,
        };
        self.stack.push(Value::Int(is_null));
        return 1;
    }

    pub fn opcode_dupisnull(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "is-null requires one argument");
            return 0;
        }

        let i1_rr = self.stack.last().unwrap();
        let is_null = match i1_rr {
            &Value::Null => 1,
            _ => 0,
        };
        self.stack.push(Value::Int(is_null));
        return 1;
    }

    /// Adds a boolean onto the stack indicating whether the topmost
    /// element is a list.
    pub fn opcode_islist(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "is-list requires one argument");
            return 0;
        }

        let i1_rr = self.stack.pop().unwrap();
        let is_list = match i1_rr {
            Value::List(_) => 1,
            _ => 0,
        };
        self.stack.push(Value::Int(is_list));
        return 1;
    }

    /// Adds a boolean onto the stack indicating whether the topmost
    /// element can be called.  (In the case of a string, this doesn't
    /// currently check that the string name maps to a function or
    /// core form, though.)
    pub fn opcode_iscallable(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "is-callable requires one argument");
            return 0;
        }

        let i1_rr = self.stack.pop().unwrap();
        let is_callable = match i1_rr {
            Value::Function(_) => 1,
            Value::CoreFunction(_) => 1,
            Value::ShiftFunction(_) => 1,
            Value::NamedFunction(_, _) => 1,
            /* This could be better. */
            Value::String(_) => 1,
            _ => 0,
        };
        self.stack.push(Value::Int(is_callable));
        return 1;
    }

    /// Convert a value into a string value.
    pub fn opcode_str(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "str requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let is_string;
        {
            match value_rr {
                Value::String(_) => {
                    is_string = true;
                }
                _ => {
                    let value_s;
                    let value_b;
                    let value_str;
                    let value_bk: Option<String>;
                    let value_opt: Option<&str> = match value_rr {
                        Value::String(sp) => {
                            value_s = sp;
                            value_b = value_s.borrow();
                            Some(&value_b.s)
                        }
                        _ => {
                            value_bk = value_rr.to_string();
                            match value_bk {
                                Some(s) => {
                                    value_str = s;
                                    Some(&value_str)
                                }
                                _ => None,
                            }
                        }
                    };

                    match value_opt {
                        Some(s) => {
                            self.stack
                                .push(Value::String(Rc::new(RefCell::new(StringPair::new(
                                    s.to_string(),
                                    None,
                                )))));
                            return 1;
                        }
                        _ => {
                            print_error(chunk, i, "unable to convert argument to string");
                            return 0;
                        }
                    }
                }
            }
        }
        if is_string {
            self.stack.push(value_rr);
        }
        return 1;
    }

    /// Convert a value into an integer/bigint value.
    pub fn opcode_int(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "int requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let is_int;
        {
            match value_rr {
                Value::Int(_) => {
                    is_int = true;
                }
                Value::BigInt(_) => {
                    is_int = true;
                }
                _ => {
                    let value_opt = value_rr.to_int();
                    match value_opt {
                        Some(n) => {
                            self.stack.push(Value::Int(n));
                            return 1;
                        }
                        _ => {
                            let value_opt = value_rr.to_bigint();
                            match value_opt {
                                Some(n) => {
                                    self.stack.push(Value::BigInt(n));
                                    return 1;
                                }
                                _ => {
                                    print_error(chunk, i, "unable to convert argument to int");
                                    return 0;
                                }
                            }
                        }
                    }
                }
            }
        }
        if is_int {
            self.stack.push(value_rr);
        }
        return 1;
    }

    /// Convert a value into a floating-point value.
    pub fn opcode_flt(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "flt requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let is_float;
        {
            match value_rr {
                Value::Float(_) => {
                    is_float = true;
                }
                _ => {
                    let value_opt = value_rr.to_float();
                    match value_opt {
                        Some(n) => {
                            self.stack.push(Value::Float(n));
                            return 1;
                        }
                        _ => {
                            print_error(chunk, i, "unable to convert argument to float");
                            return 0;
                        }
                    }
                }
            }
        }
        if is_float {
            self.stack.push(value_rr);
        }
        return 1;
    }

    /// Get a random floating-point value.
    pub fn opcode_rand(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "rand requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let value_opt = value_rr.to_float();
        match value_opt {
            Some(n) => {
                let mut rng = rand::thread_rng();
                let rand_value = rng.gen_range(0.0..n);
                self.stack.push(Value::Float(rand_value));
            }
            _ => {
                print_error(chunk, i, "unable to convert argument to float");
                return 0;
            }
        }

        return 1;
    }
}

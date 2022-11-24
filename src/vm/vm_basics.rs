use rand::Rng;

use chunk::{StringPair, Value};
use vm::*;

impl VM {
    /// Remove the top element from the stack.
    pub fn opcode_drop(&mut self) -> i32 {
        if self.stack.len() == 0 {
            self.print_error("drop requires one argument");
            return 0;
        }
        self.stack.pop();
        return 1;
    }

    /// Remove all elements from the stack.
    #[allow(unused_variables)]
    pub fn opcode_clear(&mut self) -> i32 {
        self.stack.clear();
        return 1;
    }

    /// Take the top element from the stack, duplicate it, and add it
    /// onto the stack.
    pub fn opcode_dup(&mut self) -> i32 {
        if self.stack.len() == 0 {
            self.print_error("dup requires one argument");
            return 0;
        }
        self.stack.push(self.stack.last().unwrap().clone());
        return 1;
    }

    /// Take the second element from the top from the stack, duplicate
    /// it, and add it onto the stack.
    pub fn opcode_over(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("over requires two arguments");
            return 0;
        }
        self.stack.push(self.stack[self.stack.len() - 2].clone());
        return 1;
    }

    /// Swap the top two elements from the stack.
    pub fn opcode_swap(&mut self) -> i32 {
        let len = self.stack.len();
        if len < 2 {
            self.print_error("swap requires two arguments");
            return 0;
        }
        self.stack.swap(len - 1, len - 2);
        return 1;
    }

    /// Rotate the top three elements from the stack: the top element
    /// becomes the second from top element, the second from top
    /// element becomes the third from top element, and the third from
    /// top element becomes the top element.
    pub fn opcode_rot(&mut self) -> i32 {
        if self.stack.len() < 3 {
            self.print_error("rot requires three arguments");
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
    pub fn opcode_depth(&mut self) -> i32 {
        self.stack.push(Value::Int(self.stack.len() as i32));
        return 1;
    }

    /// If the topmost element is a list, adds the length of that list
    /// onto the stack.  If the topmost element is a string, adds the
    /// length of that sting onto the stack.
    pub fn core_len(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("len requires one argument");
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
                self.print_error("len argument must be a list or a string");
                return 0;
            }
        }
        return 1;
    }

    /// Adds a boolean onto the stack indicating whether the topmost
    /// element is a null value.
    pub fn opcode_isnull(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("is-null requires one argument");
            return 0;
        }

        let i1_rr = self.stack.pop().unwrap();
        let is_null = match i1_rr {
            Value::Null => true,
            _ => false,
        };
        self.stack.push(Value::Bool(is_null));
        return 1;
    }

    pub fn opcode_dupisnull(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("is-null requires one argument");
            return 0;
        }

        let i1_rr = self.stack.last().unwrap();
        let is_null = match i1_rr {
            &Value::Null => true,
            _ => false,
        };
        self.stack.push(Value::Bool(is_null));
        return 1;
    }

    /// Adds a boolean onto the stack indicating whether the topmost
    /// element is a list.
    pub fn opcode_islist(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("is-list requires one argument");
            return 0;
        }

        let i1_rr = self.stack.pop().unwrap();
        let is_list = match i1_rr {
            Value::List(_) => true,
            _ => false,
        };
        self.stack.push(Value::Bool(is_list));
        return 1;
    }

    /// Adds a boolean onto the stack indicating whether the topmost
    /// element can be called.  (In the case of a string, this doesn't
    /// currently check that the string name maps to a function or
    /// core form, though.)
    pub fn opcode_iscallable(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("is-callable requires one argument");
            return 0;
        }

        let i1_rr = self.stack.pop().unwrap();
        let is_callable = match i1_rr {
            Value::AnonymousFunction(_, _) => true,
            Value::CoreFunction(_) => true,
            Value::NamedFunction(_) => true,
            /* This could be better. */
            Value::String(_) => true,
            _ => false,
        };
        self.stack.push(Value::Bool(is_callable));
        return 1;
    }

    /// Convert a value into a string value.
    pub fn opcode_str(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("str requires one argument");
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
                    let value_opt: Option<&str>;
                    to_str!(value_rr, value_opt);

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
                            self.stack.push(Value::Null);
                            return 1;
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
    pub fn opcode_int(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("int requires one argument");
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
                                    self.stack.push(Value::Null);
                                    return 1;
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
    pub fn opcode_flt(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("flt requires one argument");
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
                            self.stack.push(Value::Null);
                            return 1;
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

    /// Convert a value into a boolean value.
    pub fn opcode_bool(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("bool requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let new_value = Value::Bool(value_rr.to_bool());
        self.stack.push(new_value);
        return 1;
    }

    /// Check whether a value is of boolean type.
    pub fn opcode_is_bool(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("is-bool requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let res = match value_rr {
            Value::Bool(_) => true,
            _              => false
        };
        self.stack.push(Value::Bool(res));
        return 1;
    }

    /// Check whether a value is of int type.
    pub fn opcode_is_int(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("is-int requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let res = match value_rr {
            Value::Int(_) => true,
            _             => false
        };
        self.stack.push(Value::Bool(res));
        return 1;
    }

    /// Check whether a value is of bigint type.
    pub fn opcode_is_bigint(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("is-bigint requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let res = match value_rr {
            Value::BigInt(_) => true,
            _                => false
        };
        self.stack.push(Value::Bool(res));
        return 1;
    }

    /// Check whether a value is of string type.
    pub fn opcode_is_str(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("is-str requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let res = match value_rr {
            Value::String(_) => true,
            _                => false
        };
        self.stack.push(Value::Bool(res));
        return 1;
    }

    /// Check whether a value is of floating-point type.
    pub fn opcode_is_flt(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("is-flt requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let res = match value_rr {
            Value::Float(_) => true,
            _               => false
        };
        self.stack.push(Value::Bool(res));
        return 1;
    }

    /// Get a random floating-point value.
    pub fn opcode_rand(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("rand requires one argument");
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
                self.print_error("unable to convert argument to float");
                return 0;
            }
        }

        return 1;
    }

    pub fn opcode_clone(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("clone requires one argument");
            return 0;
        }
        let value_rr = self.stack.pop().unwrap();
        let cloned_value_rr = value_rr.value_clone();
        self.stack.push(cloned_value_rr);
        return 1;
    }
}

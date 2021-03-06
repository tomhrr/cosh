use std::cell::RefCell;
use std::rc::Rc;

use chunk::{HashWithIndex, Value};
use vm::*;

impl VM {
    /// Takes a hash value and a key string as its arguments.  Puts
    /// the value at that hash key onto the stack, or the null value
    /// if no such hash key exists.
    pub fn core_at(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("at requires two arguments");
            return 0;
        }

        let key_str_rr = self.stack.pop().unwrap();
	let key_str_opt: Option<&str>;
	to_str!(key_str_rr, key_str_opt);

        let hash_rr = self.stack.pop().unwrap();

        match (hash_rr, key_str_opt) {
            (Value::Hash(map), Some(s)) => {
                let mapp = map.borrow();
                let v = mapp.get(s);
                match v {
                    None => {
                        self.stack.push(Value::Null);
                    }
                    Some(r) => {
                        self.stack.push(r.clone());
                    }
                }
            }
            (_, Some(_)) => {
                self.print_error("first at argument must be hash");
                return 0;
            }
            _ => {
                self.print_error("second at argument must be string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a hash value, a key string, and a value as its
    /// arguments.  Puts the value into the hash against the specified
    /// key, and puts the updated hash back onto the stack.
    pub fn core_at_em(&mut self) -> i32 {
        if self.stack.len() < 3 {
            self.print_error("at! requires three arguments");
            return 0;
        }

        let val_rr = self.stack.pop().unwrap();

        let key_str_rr = self.stack.pop().unwrap();
	let key_str_opt: Option<&str>;
	to_str!(key_str_rr, key_str_opt);

        let mut hash_rr = self.stack.pop().unwrap();

        {
            match (&mut hash_rr, key_str_opt) {
                (Value::Hash(map), Some(s)) => {
                    map.borrow_mut().insert(s.to_string(), val_rr);
                }
                (_, Some(_)) => {
                    self.print_error("first at! argument must be hash");
                    return 0;
                }
                _ => {
                    self.print_error("second at! argument must be key string");
                    return 0;
                }
            }
        }
        self.stack.push(hash_rr);
        return 1;
    }

    /// Takes a hash value and returns a generator over the keys of
    /// the hash.
    pub fn core_keys(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("keys requires one argument");
            return 0;
        }

        let hash_rr = self.stack.pop().unwrap();
        let is_hash;
        {
            match hash_rr {
                Value::Hash(_) => {
                    is_hash = true;
                }
                _ => {
                    self.print_error("keys argument must be hash");
                    return 0;
                }
            }
        }
        if is_hash {
            self.stack.push(Value::KeysGenerator(Rc::new(RefCell::new(
                HashWithIndex::new(0, hash_rr),
            ))));
        }
        return 1;
    }

    /// Takes a hash value and returns a generator over the values of
    /// the hash.
    pub fn core_values(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("values requires one argument");
            return 0;
        }

        let hash_rr = self.stack.pop().unwrap();
        let is_hash;
        {
            match hash_rr {
                Value::Hash(_) => {
                    is_hash = true;
                }
                _ => {
                    self.print_error("values argument must be hash");
                    return 0;
                }
            }
        }
        if is_hash {
            self.stack.push(Value::ValuesGenerator(Rc::new(RefCell::new(
                HashWithIndex::new(0, hash_rr),
            ))));
        }
        return 1;
    }

    /// Takes a hash value and returns a generator over the key-value
    /// pairs from that hash.
    pub fn core_each(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("each requires one argument");
            return 0;
        }

        let hash_rr = self.stack.pop().unwrap();
        let is_hash;
        {
            match hash_rr {
                Value::Hash(_) => {
                    is_hash = true;
                }
                _ => {
                    self.print_error("each argument must be hash");
                    return 0;
                }
            }
        }
        if is_hash {
            self.stack.push(Value::EachGenerator(Rc::new(RefCell::new(
                HashWithIndex::new(0, hash_rr),
            ))));
        }
        return 1;
    }
}

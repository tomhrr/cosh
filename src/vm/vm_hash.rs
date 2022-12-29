use std::cell::RefCell;
use std::rc::Rc;

use chunk::{HashWithIndex, Value};
use vm::*;

impl VM {
    /// Takes a hash value and a key string as its arguments.  Puts
    /// the value at that hash key onto the stack, or the null value
    /// if no such hash key exists.
    pub fn core_get(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("get requires two arguments");
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
                self.print_error("first get argument must be hash");
                return 0;
            }
            _ => {
                self.print_error("second get argument must be string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a set or hash value and a key string or element value as
    /// its arguments.  For a hash, removes the value recorded against
    /// the hash key from the hash.  For a set, removes the element
    /// from the set.
    pub fn core_delete(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("delete requires two arguments");
            return 0;
        }

        let key_str_rr = self.stack.pop().unwrap();
        let key_str_opt: Option<&str>;
        to_str!(key_str_rr, key_str_opt);
        if key_str_opt.is_none() {
            self.print_error("second delete argument must be string");
            return 0;
        }
        let key_str = key_str_opt.unwrap();

        let object_rr = self.stack.pop().unwrap();

        match object_rr {
            Value::Hash(map) => {
                let mut mapp = map.borrow_mut();
                mapp.remove(key_str);
            }
            Value::Set(map) => {
                let mut mapp = map.borrow_mut();
                mapp.remove(key_str);
            }
            _ => {
                self.print_error("first delete argument must be set/hash");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a set or hash value and a key string or element value as
    /// its arguments.  Returns a boolean indicating whether the set
    /// or hash value contains the specified key/element.
    pub fn core_exists(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("exists requires two arguments");
            return 0;
        }

        let key_str_rr = self.stack.pop().unwrap();
        let key_str_opt: Option<&str>;
        to_str!(key_str_rr, key_str_opt);
        if key_str_opt.is_none() {
            self.print_error("second exists argument must be string");
            return 0;
        }
        let key_str = key_str_opt.unwrap();

        let object_rr = self.stack.pop().unwrap();

        match object_rr {
            Value::Hash(map) => {
                let mapp = map.borrow();
                let res = mapp.contains_key(key_str);
                self.stack.push(Value::Bool(res));
            }
            Value::Set(map) => {
                let mapp = map.borrow();
                let res = mapp.contains_key(key_str);
                self.stack.push(Value::Bool(res));
            }
            _ => {
                self.print_error("first exists argument must be set/hash");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a hash value, a key string, and a value as its
    /// arguments.  Puts the value into the hash against the specified
    /// key, and puts the updated hash back onto the stack.
    pub fn core_set(&mut self) -> i32 {
        if self.stack.len() < 3 {
            self.print_error("set requires three arguments");
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
                    self.print_error("first set argument must be hash");
                    return 0;
                }
                _ => {
                    self.print_error("second set argument must be key string");
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

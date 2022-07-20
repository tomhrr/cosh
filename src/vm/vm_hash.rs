use std::cell::RefCell;
use std::rc::Rc;

use chunk::{print_error, Chunk, HashWithIndex, Value};
use vm::*;

impl VM {
    /// Takes a hash value and a key string as its arguments.  Puts
    /// the value at that hash key onto the stack, or the null value
    /// if no such hash key exists.
    pub fn core_at(&mut self, chunk: Rc<RefCell<Chunk>>, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "at requires two arguments");
            return 0;
        }

        let key_str_rr = self.stack.pop().unwrap();
        let key_str_s;
        let key_str_b;
        let key_str_str;
        let key_str_bk: Option<String>;
        let key_str_opt: Option<&str> = match key_str_rr {
            Value::String(sp) => {
                key_str_s = sp;
                key_str_b = key_str_s.borrow();
                Some(&key_str_b.s)
            }
            _ => {
                key_str_bk = key_str_rr.to_string();
                match key_str_bk {
                    Some(s) => {
                        key_str_str = s;
                        Some(&key_str_str)
                    }
                    _ => None,
                }
            }
        };

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
                print_error(chunk, i, "first at argument must be hash");
                return 0;
            }
            _ => {
                print_error(chunk, i, "second at argument must be string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a hash value, a key string, and a value as its
    /// arguments.  Puts the value into the hash against the specified
    /// key, and puts the updated hash back onto the stack.
    pub fn core_at_em(&mut self, chunk: Rc<RefCell<Chunk>>, i: usize) -> i32 {
        if self.stack.len() < 3 {
            print_error(chunk, i, "at! requires three arguments");
            return 0;
        }

        let val_rr = self.stack.pop().unwrap();

        let key_str_rr = self.stack.pop().unwrap();
        let key_str_s;
        let key_str_b;
        let key_str_str;
        let key_str_bk: Option<String>;
        let key_str_opt: Option<&str> = match key_str_rr {
            Value::String(sp) => {
                key_str_s = sp;
                key_str_b = key_str_s.borrow();
                Some(&key_str_b.s)
            }
            _ => {
                key_str_bk = key_str_rr.to_string();
                match key_str_bk {
                    Some(s) => {
                        key_str_str = s;
                        Some(&key_str_str)
                    }
                    _ => None,
                }
            }
        };

        let mut hash_rr = self.stack.pop().unwrap();

        {
            match (&mut hash_rr, key_str_opt) {
                (Value::Hash(map), Some(s)) => {
                    map.borrow_mut().insert(s.to_string(), val_rr);
                }
                (_, Some(_)) => {
                    print_error(chunk, i, "first at! argument must be hash");
                    return 0;
                }
                _ => {
                    print_error(chunk, i, "second at! argument must be key string");
                    return 0;
                }
            }
        }
        self.stack.push(hash_rr);
        return 1;
    }

    /// Takes a hash value and returns a generator over the keys of
    /// the hash.
    pub fn core_keys(&mut self, chunk: Rc<RefCell<Chunk>>, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "keys requires one argument");
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
                    print_error(chunk, i, "keys argument must be hash");
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
    pub fn core_values(&mut self, chunk: Rc<RefCell<Chunk>>, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "values requires one argument");
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
                    print_error(chunk, i, "values argument must be hash");
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
    pub fn core_each(&mut self, chunk: Rc<RefCell<Chunk>>, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "each requires one argument");
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
                    print_error(chunk, i, "each argument must be hash");
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

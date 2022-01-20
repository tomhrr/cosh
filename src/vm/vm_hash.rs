use std::cell::RefCell;
use std::rc::Rc;

use chunk::{print_error, Chunk, Value};
use vm::*;

impl VM {
    /// Takes a hash value and a key string as its arguments.  Puts
    /// the value at that hash key onto the stack, or the null value
    /// if no such hash key exists.
    pub fn core_at(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "at requires two arguments");
            return 0;
        }

        let key_str_rr = self.stack.pop().unwrap();
        let key_str_rrb = key_str_rr.borrow();
        let key_str_pre = key_str_rrb.to_string();
        let key_str_opt = to_string_2(&key_str_pre);

        let hash_rr = self.stack.pop().unwrap();
        let hash_rrb = hash_rr.borrow();

        match (&*hash_rrb, key_str_opt) {
            (Value::Hash(map), Some(s)) => {
                let v = map.get(s);
                match v {
                    None => {
                        self.stack.push(Rc::new(RefCell::new(Value::Null)));
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
    pub fn core_at_em(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 3 {
            print_error(chunk, i, "at! requires three arguments");
            return 0;
        }

        let val_rr = self.stack.pop().unwrap();

        let key_str_rr = self.stack.pop().unwrap();
        let key_str_rrb = key_str_rr.borrow();
        let key_str_pre = key_str_rrb.to_string();
        let key_str_opt = to_string_2(&key_str_pre);

        let hash_rr = self.stack.pop().unwrap();

        {
            let mut hash_rrb = hash_rr.borrow_mut();
            match (&mut *hash_rrb, key_str_opt) {
                (Value::Hash(map), Some(s)) => {
                    map.insert(s.to_string(), val_rr);
                }
                (_, Some(_)) => {
                    print_error(chunk, i, "first at! argument must be hash");
                    return 0;
                }
                _ => {
                    print_error(
                        chunk,
                        i,
                        "second at! argument must be key string",
                    );
                    return 0;
                }
            }
        }
        self.stack.push(hash_rr);
        return 1;
    }

    /// Takes a hash value and returns a generator over the keys of
    /// the hash.
    pub fn core_keys(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "keys requires one argument");
            return 0;
        }

        let hash_rr = self.stack.pop().unwrap();
        let is_hash;
        {
            let hash_rrb = hash_rr.borrow();
            match &*hash_rrb {
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
            self.stack
                .push(Rc::new(RefCell::new(Value::KeysGenerator(0, hash_rr))));
        }
        return 1;
    }

    /// Takes a hash value and returns a generator over the values of
    /// the hash.
    pub fn core_values(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "values requires one argument");
            return 0;
        }

        let hash_rr = self.stack.pop().unwrap();
        let is_hash;
        {
            let hash_rrb = hash_rr.borrow();
            match &*hash_rrb {
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
            self.stack
                .push(Rc::new(RefCell::new(Value::ValuesGenerator(0, hash_rr))));
        }
        return 1;
    }

    /// Takes a hash value and returns a generator over the key-value
    /// pairs from that hash.
    pub fn core_each(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "each requires one argument");
            return 0;
        }

        let hash_rr = self.stack.pop().unwrap();
        let is_hash;
        {
            let hash_rrb = hash_rr.borrow();
            match &*hash_rrb {
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
            self.stack
                .push(Rc::new(RefCell::new(Value::EachGenerator(0, hash_rr))));
        }
        return 1;
    }
}

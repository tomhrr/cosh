use std::cell::RefCell;
use std::rc::Rc;

use chunk::{HashWithIndex, Value};
use vm::*;

impl VM {
    /// Takes a hash or list (or generator) and a key string or list
    /// index (or list of keys/indexes) as its arguments.  Puts the
    /// specified value (or list of values) onto the stack, or the
    /// null value if the specified value (or list of values) doesn't
    /// exist.
    pub fn core_get(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("get requires two arguments");
            return 0;
        }

        let specifier_rr = self.stack.pop().unwrap();
        let object_rr = self.stack.pop().unwrap();

        if object_rr.is_generator() {
            self.stack.push(object_rr);
            let res = self.generator_to_list();
            if res == 0 {
                return 0;
            }
            self.stack.push(specifier_rr);
            return self.core_get();
        }

        let specifier_opt: Option<&str>;
        to_str!(specifier_rr.clone(), specifier_opt);

        match (object_rr, specifier_opt) {
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
            (Value::Hash(map), None) => {
                match specifier_rr {
                    Value::List(lst) => {
                        let mapb = map.borrow();
                        let mut results = VecDeque::new();
                        for e in lst.borrow().iter() {
                            let e_opt: Option<&str>;
                            to_str!(e, e_opt);
                            if let Some(s) = e_opt {
                                let v = mapb.get(s);
                                match v {
                                    None => {
                                        results.push_back(Value::Null);
                                    }
                                    Some(r) => {
                                        results.push_back(r.clone());
                                    }
                                }
                            } else {
                                self.print_error("second get argument must be list of strings");
                                return 0;
                            }
                        }
                        let newlst =
                            Value::List(Rc::new(RefCell::new(results)));
                        self.stack.push(newlst);
                        return 1;
                    }
                    _ => {
                        self.print_error("second get argument must be field specifier");
                        return 0;
                    }
                }
            }
            (Value::List(lst), _) => {
		let num_int_opt = specifier_rr.to_int();
                if let Some(n) = num_int_opt {
		    if lst.borrow().len() <= (n as usize) {
                        self.stack.push(Value::Null);
                        return 1;
		    }
		    let element = lst.borrow()[n as usize].clone();
		    self.stack.push(element);
		    return 1;
                }
                match specifier_rr {
                    Value::List(ilst) => {
                        let lstb = lst.borrow();
                        let mut results = VecDeque::new();
                        for e in ilst.borrow().iter() {
                            let e_opt = e.to_int();
                            if let Some(n) = e_opt {
                                if lstb.len() <= (n as usize) {
                                    results.push_back(Value::Null);
                                } else {
                                    results.push_back(lstb[n as usize].clone());
                                }
                            } else {
                                self.print_error("second get argument must be list of integers");
                                return 0;
                            }
                        }
                        let newlst =
                            Value::List(Rc::new(RefCell::new(results)));
                        self.stack.push(newlst);
                        return 1;
                    }
                    _ => {
                        self.print_error("second get argument must be field specifier");
                        return 0;
                    }
                }
            }
            _ => {
                self.print_error("first get argument must be list/hash");
                return 0;
            }
        }
        1
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
        1
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
        1
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

        let specifier_rr = self.stack.pop().unwrap();
        let specifier_opt: Option<&str>;
        to_str!(specifier_rr.clone(), specifier_opt);

        let mut object_rr = self.stack.pop().unwrap();

        {
            match (&mut object_rr, specifier_opt) {
                (Value::Hash(map), Some(s)) => {
                    map.borrow_mut().insert(s.to_string(), val_rr);
                }
                (Value::Hash(_), None) => {
                    self.print_error("second set argument must be key string");
                }
                (Value::List(lst), _) => {
                    let num_int_opt = specifier_rr.to_int();
                    match num_int_opt {
                        Some(n) => {
                            if lst.borrow().len() <= (n as usize) {
                                self.print_error("second set argument must fall within list bounds");
                                return 0;
                            }
                            lst.borrow_mut()[n as usize] = val_rr;
                        }
                        _ => {
                            self.print_error("second set argument must be field specifier");
                        }
                    }
                }
                _ => {
                    self.print_error("first set argument must be list/hash");
                    return 0;
                }
            }
        }

        self.stack.push(object_rr);
        1
    }

    /// Takes a hash value and returns a generator over the keys of
    /// the hash.
    pub fn core_keys(&mut self) -> i32 {
        if self.stack.is_empty() {
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
        1
    }

    /// Takes a hash value and returns a generator over the values of
    /// the hash.
    pub fn core_values(&mut self) -> i32 {
        if self.stack.is_empty() {
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
        1
    }

    /// Takes a hash value and returns a generator over the key-value
    /// pairs from that hash.
    pub fn core_each(&mut self) -> i32 {
        if self.stack.is_empty() {
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
        1
    }
}

use std::cell::RefCell;
use std::rc::Rc;

use sha1::{Digest, Sha1};
use sha2::{Sha256, Sha512};

use chunk::{StringTriple, Value};
use vm::*;

impl VM {
    pub fn core_md5(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("md5 requires one argument");
            return 0;
        }

        let str_rr = self.stack.pop().unwrap();
        let str_opt: Option<&str>;
        to_str!(str_rr, str_opt);

        match str_opt {
            Some(s) => {
                let digest = md5::compute(s.as_bytes());
                let st = StringTriple::new(format!("{:x}", digest), None);
                self.stack.push(Value::String(Rc::new(RefCell::new(st))));
            }
            _ => {
                self.print_error("md5 argument must be string");
                return 0;
            }
        }
        return 1;
    }

    pub fn core_sha1(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("sha1 requires one argument");
            return 0;
        }

        let str_rr = self.stack.pop().unwrap();
        let str_opt: Option<&str>;
        to_str!(str_rr, str_opt);

        match str_opt {
            Some(s) => {
                let mut hasher = Sha1::new();
                hasher.update(s.as_bytes());
                let digest = hasher.finalize();
                let st = StringTriple::new(format!("{:x}", digest), None);
                self.stack.push(Value::String(Rc::new(RefCell::new(st))));
            }
            _ => {
                self.print_error("sha1 argument must be string");
                return 0;
            }
        }
        return 1;
    }

    pub fn core_sha256(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("sha256 requires one argument");
            return 0;
        }

        let str_rr = self.stack.pop().unwrap();
        let str_opt: Option<&str>;
        to_str!(str_rr, str_opt);

        match str_opt {
            Some(s) => {
                let mut hasher = Sha256::new();
                hasher.update(s.as_bytes());
                let digest = hasher.finalize();
                let st = StringTriple::new(format!("{:x}", digest), None);
                self.stack.push(Value::String(Rc::new(RefCell::new(st))));
            }
            _ => {
                self.print_error("sha1 argument must be string");
                return 0;
            }
        }
        return 1;
    }

    pub fn core_sha512(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("sha512 requires one argument");
            return 0;
        }

        let str_rr = self.stack.pop().unwrap();
        let str_opt: Option<&str>;
        to_str!(str_rr, str_opt);

        match str_opt {
            Some(s) => {
                let mut hasher = Sha512::new();
                hasher.update(s.as_bytes());
                let digest = hasher.finalize();
                let st = StringTriple::new(format!("{:x}", digest), None);
                self.stack.push(Value::String(Rc::new(RefCell::new(st))));
            }
            _ => {
                self.print_error("sha1 argument must be string");
                return 0;
            }
        }
        return 1;
    }
}

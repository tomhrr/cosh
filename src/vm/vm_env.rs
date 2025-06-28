use std::cell::RefCell;
use std::env;
use std::rc::Rc;

use indexmap::IndexMap;

use crate::hasher::new_hash_indexmap;
use crate::vm::*;

impl VM {
    /// Add a hash containing the data from the current environment to
    /// the stack.
    pub fn core_env(&mut self) -> i32 {
        let mut hsh = new_hash_indexmap();
        for (key, value) in env::vars() {
            let value_str = new_string_value(value);
            hsh.insert(key, value_str);
        }
        let hsh_rr = Value::Hash(Rc::new(RefCell::new(hsh)));
        self.stack.push(hsh_rr);
        1
    }

    /// Takes an environment variable name as its argument.  Puts the
    /// corresponding environment variable value onto the stack.  If
    /// no such environment variable exists, puts null onto the stack.
    pub fn core_getenv(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("getenv requires one argument");
            return 0;
        }

        let key_rr = self.stack.pop().unwrap();
        let key_opt: Option<&str>;
        to_str!(key_rr, key_opt);
        match key_opt {
            Some(s) => {
                let value_res = env::var(s);
                match value_res {
                    Ok(value) => {
                        self.stack.push(new_string_value(value));
                    }
                    _ => {
                        /* Assume that inability to get an environment
                         * variable means that the variable is not
                         * set. */
                        self.stack.push(Value::Null);
                    }
                }
            }
            _ => {
                self.print_error("getenv argument must be a string");
                return 0;
            }
        }

        1
    }

    /// Takes an environment variable name and a value as its
    /// arguments.  Sets the environment variable with the given name
    /// to have the given value.
    pub fn core_setenv(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("setenv requires two arguments");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let key_rr = self.stack.pop().unwrap();

        let key_opt: Option<&str>;
        to_str!(key_rr, key_opt);
        let value_opt: Option<&str>;
        to_str!(value_rr, value_opt);

        match (key_opt, value_opt) {
            (Some(key_s), Some(value_s)) => {
                if key_s.is_empty() {
                    self.print_error("first setenv argument must be a variable name");
                    return 0;
                }
                env::set_var(key_s, value_s);
            }
            (Some(_), _) => {
                self.print_error("second setenv argument must be a variable value");
                return 0;
            }
            (_, _) => {
                self.print_error("first setenv argument must be a variable name");
                return 0;
            }
        }

        1
    }
}

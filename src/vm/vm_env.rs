use std::cell::RefCell;
use std::env;
use std::rc::Rc;

use indexmap::IndexMap;

use chunk::StringPair;
use vm::*;

impl VM {
    pub fn core_env(&mut self) -> i32 {
        let mut hsh = IndexMap::new();
        for (key, value) in env::vars() {
            let value_str =
                Value::String(Rc::new(RefCell::new(StringPair::new(value, None))));
            hsh.insert(key, value_str);
        }
        let hsh_rr = Value::Hash(Rc::new(RefCell::new(hsh)));
        self.stack.push(hsh_rr);
        return 1;
    }

    pub fn core_getenv(&mut self) -> i32 {
        if self.stack.len() < 1 {
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
                        let value_sp = StringPair::new(value, None);
                        let value_rr = Rc::new(RefCell::new(value_sp));
                        self.stack.push(Value::String(value_rr));
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

        return 1;
    }

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
                if key_s.len() == 0 {
                    self.print_error("environment variable name must be set");
                    return 0;
                }
                env::set_var(key_s, value_s);
            }
            (_, Some(_)) => {
                self.print_error("environment variable value must be string");
                return 0;
            }
            (_, _) => {
                self.print_error("environment variable name must be string");
                return 0;
            }
        }

        return 1;
    }
}

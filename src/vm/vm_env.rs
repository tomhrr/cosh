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
}

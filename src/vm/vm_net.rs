use std::cell::RefCell;
use std::rc::Rc;

use sysinfo::Networks;

use crate::chunk::{HashWithIndex, Value,
                   new_string_value};
use crate::vm::*;

impl VM {
    pub fn core_ifconfig(&mut self) -> i32 {
        return 1;
    }
}

use md5;
use sha1::{Digest, Sha1};
use sha2::{Sha256, Sha512};

use crate::chunk::Value;
use crate::vm::*;

impl VM {
    /// The basic functionality is common to each function, and should
    /// be refactored so as to avoid the repetition.

    /// Takes a string as its single argument.  Hashes the string
    /// using the MD5 algorithm and adds the result to the stack.
    pub fn core_md5(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("md5 requires one argument");
            return 0;
        }

        let mut hasher = md5::Context::new();

        let input_rr = self.stack.pop().unwrap();
        if input_rr.is_shiftable() {
            self.stack.push(input_rr);
            loop {
                let dup_res = self.opcode_dup();
                if dup_res == 0 {
                    return 0;
                }
                let shift_res = self.opcode_shift();
                if shift_res == 0 {
                    return 0;
                }
                let element_rr = self.stack.pop().unwrap();
                match element_rr {
                    Value::Null => {
                        break;
                    }
                    Value::Byte(b) => {
                        let byte_array: [u8; 1] = [b];
                        hasher.consume(&byte_array);
                    }
                    _ => {
                        let str_opt: Option<&str>;
                        to_str!(element_rr, str_opt);
                        match str_opt {
                            Some(s) => {
                                hasher.consume(s.as_bytes());
                            }
                            _ => {
                                self.print_error("md5 argument is invalid");
                                return 0;
                            }
                        }
                    }
                }
            }
            self.stack.pop();
        } else {
            match input_rr {
                Value::Byte(b) => {
                    let byte_array: [u8; 1] = [b];
                    hasher.consume(&byte_array);
                }
                _ => {
                    let str_opt: Option<&str>;
                    to_str!(input_rr, str_opt);
                    match str_opt {
                        Some(s) => {
                            hasher.consume(s.as_bytes());
                        }
                        _ => {
                            self.print_error("md5 argument is invalid");
                            return 0;
                        }
                    }
                }
            };
        }

        let digest = hasher.compute();
        let mut byte_list = VecDeque::new();
        for byte in digest.into_iter() {
            byte_list.push_back(Value::Byte(byte));
        }
        self.stack.push(Value::List(Rc::new(RefCell::new(byte_list))));
        return 1;
    }

    /// Takes a string as its single argument.  Hashes the string
    /// using the SHA1 algorithm and adds the result to the stack.
    pub fn core_sha1(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("sha1 requires one argument");
            return 0;
        }

        let mut hasher = Sha1::new();

        let input_rr = self.stack.pop().unwrap();
        if input_rr.is_shiftable() {
            self.stack.push(input_rr);
            loop {
                let dup_res = self.opcode_dup();
                if dup_res == 0 {
                    return 0;
                }
                let shift_res = self.opcode_shift();
                if shift_res == 0 {
                    return 0;
                }
                let element_rr = self.stack.pop().unwrap();
                match element_rr {
                    Value::Null => {
                        break;
                    }
                    Value::Byte(b) => {
                        let byte_array: [u8; 1] = [b];
                        hasher.update(&byte_array);
                    }
                    _ => {
                        let str_opt: Option<&str>;
                        to_str!(element_rr, str_opt);
                        match str_opt {
                            Some(s) => {
                                hasher.update(s.as_bytes());
                            }
                            _ => {
                                self.print_error("sha1 argument is invalid");
                                return 0;
                            }
                        }
                    }
                }
            }
            self.stack.pop();
        } else {
            match input_rr {
                Value::Byte(b) => {
                    let byte_array: [u8; 1] = [b];
                    hasher.update(&byte_array);
                }
                _ => {
                    let str_opt: Option<&str>;
                    to_str!(input_rr, str_opt);
                    match str_opt {
                        Some(s) => {
                            hasher.update(s.as_bytes());
                        }
                        _ => {
                            self.print_error("sha1 argument is invalid");
                            return 0;
                        }
                    }
                }
            };
        }

        let digest = hasher.finalize();
        let mut byte_list = VecDeque::new();
        for byte in digest.into_iter() {
            byte_list.push_back(Value::Byte(byte));
        }
        self.stack.push(Value::List(Rc::new(RefCell::new(byte_list))));
        return 1;
    }

    /// Takes a string as its single argument.  Hashes the string
    /// using the SHA256 algorithm and adds the result to the stack.
    pub fn core_sha256(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("sha256 requires one argument");
            return 0;
        }

        let mut hasher = Sha256::new();

        let input_rr = self.stack.pop().unwrap();
        if input_rr.is_shiftable() {
            self.stack.push(input_rr);
            loop {
                let dup_res = self.opcode_dup();
                if dup_res == 0 {
                    return 0;
                }
                let shift_res = self.opcode_shift();
                if shift_res == 0 {
                    return 0;
                }
                let element_rr = self.stack.pop().unwrap();
                match element_rr {
                    Value::Null => {
                        break;
                    }
                    Value::Byte(b) => {
                        let byte_array: [u8; 1] = [b];
                        hasher.update(&byte_array);
                    }
                    _ => {
                        let str_opt: Option<&str>;
                        to_str!(element_rr, str_opt);
                        match str_opt {
                            Some(s) => {
                                hasher.update(s.as_bytes());
                            }
                            _ => {
                                self.print_error("sha256 argument is invalid");
                                return 0;
                            }
                        }
                    }
                }
            }
            self.stack.pop();
        } else {
            match input_rr {
                Value::Byte(b) => {
                    let byte_array: [u8; 1] = [b];
                    hasher.update(&byte_array);
                }
                _ => {
                    let str_opt: Option<&str>;
                    to_str!(input_rr, str_opt);
                    match str_opt {
                        Some(s) => {
                            hasher.update(s.as_bytes());
                        }
                        _ => {
                            self.print_error("sha256 argument is invalid");
                            return 0;
                        }
                    }
                }
            };
        }

        let digest = hasher.finalize();
        let mut byte_list = VecDeque::new();
        for byte in digest.into_iter() {
            byte_list.push_back(Value::Byte(byte));
        }
        self.stack.push(Value::List(Rc::new(RefCell::new(byte_list))));
        return 1;
    }

    /// Takes a string as its single argument.  Hashes the string
    /// using the SHA512 algorithm and adds the result to the stack.
    pub fn core_sha512(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("sha512 requires one argument");
            return 0;
        }

        let mut hasher = Sha512::new();

        let input_rr = self.stack.pop().unwrap();
        if input_rr.is_shiftable() {
            self.stack.push(input_rr);
            loop {
                let dup_res = self.opcode_dup();
                if dup_res == 0 {
                    return 0;
                }
                let shift_res = self.opcode_shift();
                if shift_res == 0 {
                    return 0;
                }
                let element_rr = self.stack.pop().unwrap();
                match element_rr {
                    Value::Null => {
                        break;
                    }
                    Value::Byte(b) => {
                        let byte_array: [u8; 1] = [b];
                        hasher.update(&byte_array);
                    }
                    _ => {
                        let str_opt: Option<&str>;
                        to_str!(element_rr, str_opt);
                        match str_opt {
                            Some(s) => {
                                hasher.update(s.as_bytes());
                            }
                            _ => {
                                self.print_error("sha512 argument is invalid");
                                return 0;
                            }
                        }
                    }
                }
            }
            self.stack.pop();
        } else {
            match input_rr {
                Value::Byte(b) => {
                    let byte_array: [u8; 1] = [b];
                    hasher.update(&byte_array);
                }
                _ => {
                    let str_opt: Option<&str>;
                    to_str!(input_rr, str_opt);
                    match str_opt {
                        Some(s) => {
                            hasher.update(s.as_bytes());
                        }
                        _ => {
                            self.print_error("sha512 argument is invalid");
                            return 0;
                        }
                    }
                }
            };
        }

        let digest = hasher.finalize();
        let mut byte_list = VecDeque::new();
        for byte in digest.into_iter() {
            byte_list.push_back(Value::Byte(byte));
        }
        self.stack.push(Value::List(Rc::new(RefCell::new(byte_list))));
        return 1;
    }
}

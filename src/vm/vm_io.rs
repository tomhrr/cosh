use std::cell::RefCell;
use std::fs::metadata;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Write;
use std::rc::Rc;

use chunk::{print_error, Chunk, StringPair, Value};
use vm::*;

impl VM {
    /// Takes a file path and a mode string (either 'r' or 'w') as its
    /// arguments, and puts a FileReader or FileWriter object on the
    /// stack as appropriate.
    pub fn opcode_open(&mut self, chunk: Rc<RefCell<Chunk>>, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "open requires two arguments");
            return 0;
        }

        let rw_rr = self.stack.pop().unwrap();
        let path_rr = self.stack.pop().unwrap();
        let path_str_s;
        let path_str_b;
        let path_str_str;
        let path_str_bk: Option<String>;
        let path_str_opt: Option<&str> = match path_rr {
            Value::String(sp) => {
                path_str_s = sp;
                path_str_b = path_str_s.borrow();
                Some(&path_str_b.s)
            }
            _ => {
                path_str_bk = path_rr.to_string();
                match path_str_bk {
                    Some(s) => {
                        path_str_str = s;
                        Some(&path_str_str)
                    }
                    _ => None,
                }
            }
        };

        match rw_rr {
            Value::String(sp) => match sp.borrow().s.as_ref() {
                "r" => match path_str_opt {
                    Some(s) => {
                        let file_res = File::open(s);
                        match file_res {
                            Ok(file) => {
                                self.stack.push(Value::FileReader(Rc::new(RefCell::new(
                                    BufReader::new(file),
                                ))));
                            }
                            Err(e) => {
                                let err_str = format!("unable to open file: {}", e.to_string());
                                print_error(chunk, i, &err_str);
                                return 0;
                            }
                        }
                    }
                    _ => {
                        print_error(chunk, i, "path for open must be a string");
                        return 0;
                    }
                },
                "w" => match path_str_opt {
                    Some(s) => {
                        let file_res = File::create(s);
                        match file_res {
                            Ok(file) => {
                                self.stack.push(Value::FileWriter(Rc::new(RefCell::new(
                                    BufWriter::new(file),
                                ))));
                            }
                            Err(e) => {
                                let err_str = format!("unable to open file: {}", e.to_string());
                                print_error(chunk, i, &err_str);
                                return 0;
                            }
                        }
                    }
                    _ => {
                        print_error(chunk, i, "path for open must be a string");
                        return 0;
                    }
                },
                _ => {
                    print_error(chunk, i, "mode for open must be 'r' or 'w'");
                    return 0;
                }
            },
            _ => {
                print_error(chunk, i, "mode for open must be 'r' or 'w'");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a FileReader object as its single argument.  Reads one
    /// line from that object and places it onto the stack (including
    /// the ending newline).
    pub fn opcode_readline(&mut self, chunk: Rc<RefCell<Chunk>>, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "readline requires one argument");
            return 0;
        }

        let file_reader_rr = self.stack.pop().unwrap();

        match file_reader_rr {
            Value::FileReader(bufread) => {
                let mut contents = String::new();
                let res = bufread.borrow_mut().read_line(&mut contents);
                match res {
                    Ok(0) => {
                        self.stack.push(Value::Null);
                    }
                    Ok(_) => {
                        self.stack
                            .push(Value::String(Rc::new(RefCell::new(StringPair::new(
                                contents, None,
                            )))));
                    }
                    _ => {
                        print_error(chunk, i, "unable to read line from file");
                        return 0;
                    }
                }
            }
            _ => {
                print_error(chunk, i, "readline argument must be a file reader");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a FileWriter object and a line as its arguments.  Writes
    /// the line to the file.
    pub fn core_writeline(&mut self, chunk: Rc<RefCell<Chunk>>, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "writeline requires two arguments");
            return 0;
        }

        let line_rr = self.stack.pop().unwrap();
        let line_str_s;
        let line_str_b;
        let line_str_str;
        let line_str_bk: Option<String>;
        let line_str_opt: Option<&str> = match line_rr {
            Value::String(sp) => {
                line_str_s = sp;
                line_str_b = line_str_s.borrow();
                Some(&line_str_b.s)
            }
            _ => {
                line_str_bk = line_rr.to_string();
                match line_str_bk {
                    Some(s) => {
                        line_str_str = s;
                        Some(&line_str_str)
                    }
                    _ => None,
                }
            }
        };

        match line_str_opt {
            Some(s) => {
                if s != "" {
                    let mut file_writer = self.stack.pop().unwrap();
                    match file_writer {
                        Value::FileWriter(ref mut line_writer) => {
                            let res = line_writer.borrow_mut().write_all(s.as_bytes());
                            match res {
                                Ok(_) => {
                                    return 1;
                                }
                                Err(e) => {
                                    let err_str = format!("unable to open file: {}", e.to_string());
                                    print_error(chunk, i, &err_str);
                                    return 0;
                                }
                            }
                        }
                        _ => {
                            print_error(chunk, i, "writeline argument must be a file writer");
                            return 0;
                        }
                    }
                }
            }
            _ => {
                print_error(chunk, i, "writeline argument must be a string");
                return 0;
            }
        };
        return 1;
    }

    /// Takes a FileReader or FileWriter object as its single
    /// argument.  Closes the object, if required.
    pub fn core_close(&mut self, chunk: Rc<RefCell<Chunk>>, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "close requires one argument");
            return 0;
        }

        let mut file_rr = self.stack.pop().unwrap();

        match file_rr {
            Value::FileReader(_) => {
                // No action required.
                return 1;
            }
            Value::FileWriter(ref mut line_writer) => {
                let res = line_writer.borrow_mut().flush();
                match res {
                    Ok(_) => {
                        return 1;
                    }
                    Err(e) => {
                        let err_str = format!("unable to flush data: {}", e.to_string());
                        print_error(chunk, i, &err_str);
                        return 0;
                    }
                }
            }
            _ => {
                print_error(chunk, i, "close argument must be a file reader or writer");
                return 0;
            }
        }
    }

    /// Takes a directory path as its single argument.  Opens the
    /// directory and places a DirectoryHandle object for the
    /// directory onto the stack.
    pub fn core_opendir(&mut self, chunk: Rc<RefCell<Chunk>>, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "opendir requires one argument");
            return 0;
        }

        let path_rr = self.stack.pop().unwrap();
        let path_str_s;
        let path_str_b;
        let path_str_str;
        let path_str_bk: Option<String>;
        let path_str_opt: Option<&str> = match path_rr {
            Value::String(sp) => {
                path_str_s = sp;
                path_str_b = path_str_s.borrow();
                Some(&path_str_b.s)
            }
            _ => {
                path_str_bk = path_rr.to_string();
                match path_str_bk {
                    Some(s) => {
                        path_str_str = s;
                        Some(&path_str_str)
                    }
                    _ => None,
                }
            }
        };

        match path_str_opt {
            Some(s) => {
                let dir_handle_res = std::fs::read_dir(s);
                match dir_handle_res {
                    Ok(dir_handle) => {
                        self.stack
                            .push(Value::DirectoryHandle(Rc::new(RefCell::new(dir_handle))));
                        return 1;
                    }
                    Err(e) => {
                        let err_str = format!("unable to open directory: {}", e.to_string());
                        print_error(chunk, i, &err_str);
                        return 0;
                    }
                }
            }
            _ => {
                print_error(chunk, i, "opendir argument must be a string");
                return 0;
            }
        }
    }

    /// Takes a DirectoryHandle object as its single argument.  Reads
    /// the next entry from the corresponding handle and places it
    /// onto the stack.
    pub fn core_readdir(&mut self, chunk: Rc<RefCell<Chunk>>, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "readdir requires one argument");
            return 0;
        }

        let mut dir_handle_rr = self.stack.pop().unwrap();

        let entry_value = match dir_handle_rr {
            Value::DirectoryHandle(ref mut dir_handle) => {
                let entry_opt = dir_handle.borrow_mut().next();
                match entry_opt {
                    Some(s) => {
                        let path = s.unwrap().path();
                        Value::String(Rc::new(RefCell::new(StringPair::new(
                            path.to_str().unwrap().to_string(),
                            None,
                        ))))
                    }
                    None => Value::Null,
                }
            }
            _ => {
                print_error(chunk, i, "readdir argument must be a directory handle");
                return 0;
            }
        };

        self.stack.push(entry_value);
        return 1;
    }

    /// Takes a path as its single argument.  Places a boolean onto
    /// the stack indicating whether the path maps to a directory.
    pub fn core_is_dir(&mut self, chunk: Rc<RefCell<Chunk>>, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "is-dir requires one argument");
            return 0;
        }

        let path_rr = self.stack.pop().unwrap();
        let path_str_s;
        let path_str_b;
        let path_str_str;
        let path_str_bk: Option<String>;
        let path_str_opt: Option<&str> = match path_rr {
            Value::String(sp) => {
                path_str_s = sp;
                path_str_b = path_str_s.borrow();
                Some(&path_str_b.s)
            }
            _ => {
                path_str_bk = path_rr.to_string();
                match path_str_bk {
                    Some(s) => {
                        path_str_str = s;
                        Some(&path_str_str)
                    }
                    _ => None,
                }
            }
        };

        match path_str_opt {
            Some(s) => {
                let metadata_res = metadata(s);
                match metadata_res {
                    Ok(metadata) => {
                        let is_dir = match metadata.is_dir() {
                            true => 1,
                            false => 0,
                        };
                        self.stack.push(Value::Int(is_dir));
                    }
                    _ => {
                        self.stack.push(Value::Int(0));
                    }
                }
            }
            _ => {
                print_error(chunk, i, "is-dir argument must be a string");
                return 0;
            }
        }
        return 1;
    }
}

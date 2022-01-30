use std::cell::RefCell;
use std::fs::metadata;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::LineWriter;
use std::io::Write;
use std::rc::Rc;

use chunk::{print_error, Chunk, Value};
use vm::*;

impl VM {
    /// Takes a file path and a mode string (either 'r' or 'w') as its
    /// arguments, and puts a FileReader or FileWriter object on the
    /// stack as appropriate.
    pub fn opcode_open(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "open requires two arguments");
            return 0;
        }

        let rw_rr = self.stack.pop().unwrap();
        let mut rw_rm;
        let rw_rrb = match rw_rr {
            RValue::Raw(ref v) => v,
            RValue::Ref(ref v_rc) => {
                rw_rm = v_rc.borrow();
                &*rw_rm
            }
        };
        let path_rr = self.stack.pop().unwrap();
        let mut path_rm;
        let path_rrb = match path_rr {
            RValue::Raw(ref v) => v,
            RValue::Ref(ref v_rc) => {
                path_rm = v_rc.borrow();
                &*path_rm
            }
        };
        let path_str_pre = path_rrb.to_string();
        let path_str_opt = to_string_2(&path_str_pre);

        match rw_rrb {
            Value::String(rw_str, _) => match rw_str.as_ref() {
                "r" => match path_str_opt {
                    Some(s) => {
                        let file_res = File::open(s);
                        match file_res {
                            Ok(file) => {
                                self.stack.push(RValue::Ref(Rc::new(RefCell::new(
                                    Value::FileReader(BufReader::new(file)),
                                ))));
                            }
                            Err(e) => {
                                let err_str = format!(
                                    "unable to open file: {}",
                                    e.to_string()
                                );
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
                                self.stack.push(RValue::Ref(Rc::new(RefCell::new(
                                    Value::FileWriter(LineWriter::new(file)),
                                ))));
                            }
                            Err(e) => {
                                let err_str = format!(
                                    "unable to open file: {}",
                                    e.to_string()
                                );
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
    pub fn opcode_readline(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "readline requires one argument");
            return 0;
        }

        let mut file_reader_rr = self.stack.pop().unwrap();
        let mut file_reader_rm;
        let mut file_reader_rrb = match file_reader_rr {
            RValue::Raw(ref mut v) => v,
            RValue::Ref(ref mut v_rc) => {
                file_reader_rm = v_rc.borrow_mut();
                &mut *file_reader_rm
            }
        };

        match file_reader_rrb {
            Value::FileReader(ref mut bufread) => {
                let mut contents = String::new();
                let res = BufRead::read_line(bufread, &mut contents);
                match res {
                    Ok(0) => {
                        self.stack.push(RValue::Raw(Value::Null));
                    }
                    Ok(_) => {
                        self.stack.push(RValue::Ref(Rc::new(RefCell::new(Value::String(
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
                print_error(
                    chunk,
                    i,
                    "readline argument must be a file reader",
                );
                return 0;
            }
        }
        return 1;
    }

    /// Takes a FileWriter object and a line as its arguments.  Writes
    /// the line to the file.
    pub fn core_writeline(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "writeline requires two arguments");
            return 0;
        }

        let mut line_rr = self.stack.pop().unwrap();
        let mut line_rm;
        let mut line_rrb = match line_rr {
            RValue::Raw(ref mut v) => v,
            RValue::Ref(ref mut v_rc) => {
                line_rm = v_rc.borrow_mut();
                &mut *line_rm
            }
        };
        let line_str_pre = line_rrb.to_string();
        let line_str_opt = to_string_2(&line_str_pre);

        match line_str_opt {
            Some(s) => {
                if s != "" {
                    let mut file_writer_rr = self.stack.pop().unwrap();
		    let mut file_writer_rm;
		    let mut file_writer_rrb = match file_writer_rr {
			RValue::Raw(ref mut v) => v,
			RValue::Ref(ref mut v_rc) => {
			    file_writer_rm = v_rc.borrow_mut();
			    &mut *file_writer_rm
			}
		    };
                    match file_writer_rrb {
                        Value::FileWriter(ref mut line_writer) => {
                            let res = line_writer.write_all(s.as_bytes());
                            match res {
                                Ok(_) => {
                                    return 1;
                                }
                                Err(e) => {
                                    let err_str = format!(
                                        "unable to open file: {}",
                                        e.to_string()
                                    );
                                    print_error(chunk, i, &err_str);
                                    return 0;
                                }
                            }
                        }
                        _ => {
                            print_error(
                                chunk,
                                i,
                                "writeline argument must be a file writer",
                            );
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
    pub fn core_close(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "close requires one argument");
            return 0;
        }

        let mut file_rr = self.stack.pop().unwrap();
        let mut file_rm;
        let mut file_rrb = match file_rr {
            RValue::Raw(ref mut v) => v,
            RValue::Ref(ref mut v_rc) => {
                file_rm = v_rc.borrow_mut();
                &mut *file_rm
            }
        };

        match file_rrb {
            Value::FileReader(_) => {
                // No action required.
                return 1;
            }
            Value::FileWriter(ref mut line_writer) => {
                let res = line_writer.flush();
                match res {
                    Ok(_) => {
                        return 1;
                    }
                    Err(e) => {
                        let err_str = format!(
                            "unable to flush data: {}",
                            e.to_string()
                        );
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
    pub fn core_opendir(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "opendir requires one argument");
            return 0;
        }

        let path_rr = self.stack.pop().unwrap();
        let mut path_rm;
        let path_rrb = match path_rr {
            RValue::Raw(ref v) => v,
            RValue::Ref(ref v_rc) => {
                path_rm = v_rc.borrow();
                &*path_rm
            }
        };
        let path_str_pre = path_rrb.to_string();
        let path_str_opt = to_string_2(&path_str_pre);

        match path_str_opt {
            Some(s) => {
                let dir_handle_res = std::fs::read_dir(s);
                match dir_handle_res {
                    Ok(dir_handle) => {
                        self.stack.push(
                            RValue::Ref(Rc::new(RefCell::new(Value::DirectoryHandle(dir_handle)))));
                        return 1;
                    }
                    Err(e) => {
                        let err_str = format!(
                            "unable to open directory: {}",
                            e.to_string()
                        );
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
    pub fn core_readdir(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "readdir requires one argument");
            return 0;
        }

        let mut dir_handle_rr = self.stack.pop().unwrap();
        let mut dir_handle_rm;
        let mut dir_handle_rrb = match dir_handle_rr {
            RValue::Raw(ref mut v) => v,
            RValue::Ref(ref mut v_rc) => {
                dir_handle_rm = v_rc.borrow_mut();
                &mut *dir_handle_rm
            }
        };

        let entry_value = match dir_handle_rrb {
            Value::DirectoryHandle(ref mut dir_handle) => {
                let entry_opt = dir_handle.next();
                match entry_opt {
                    Some(s) => {
                        let path = s.unwrap().path();
                        RValue::Ref(Rc::new(RefCell::new(Value::String(
                            path.to_str().unwrap().to_string(),
                            None,
                        ))))
                    }
                    None => RValue::Raw(Value::Null),
                }
            }
            _ => {
                print_error(
                    chunk,
                    i,
                    "readdir argument must be a directory handle",
                );
                return 0;
            }
        };

        self.stack.push(entry_value);
        return 1;
    }

    /// Takes a path as its single argument.  Places a boolean onto
    /// the stack indicating whether the path maps to a directory.
    pub fn core_is_dir(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "is-dir requires one argument");
            return 0;
        }

        let path_rr = self.stack.pop().unwrap();
        let mut path_rm;
        let path_rrb = match path_rr {
            RValue::Raw(ref v) => v,
            RValue::Ref(ref v_rc) => {
                path_rm = v_rc.borrow();
                &*path_rm
            }
        };
        let path_str_pre = path_rrb.to_string();
        let path_str_opt = to_string_2(&path_str_pre);

        match path_str_opt {
            Some(s) => {
                let metadata_res = metadata(s);
                match metadata_res {
                    Ok(metadata) => {
                        let is_dir = match metadata.is_dir() {
                            true => 1,
                            false => 0,
                        };
                        self.stack
                            .push(RValue::Raw(Value::Int(is_dir)));
                    }
                    _ => {
                        self.stack.push(RValue::Raw(Value::Int(0)));
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

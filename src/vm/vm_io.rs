use std::cell::RefCell;
use std::fs::metadata;
use std::fs::symlink_metadata;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Write;
use std::rc::Rc;
use std::thread;
use std::time;

use lazy_static::lazy_static;
use nix::unistd::AccessFlags;
use regex::Regex;
use tempfile::{NamedTempFile, TempDir};

use crate::chunk::{Value, BufReaderWithBuffer};
use crate::vm::*;

lazy_static! {
    static ref TRAILING_SLASHES: Regex = Regex::new("/*$").unwrap();
}

impl VM {
    /// Takes a file path and a mode string (either 'r' or 'w') as its
    /// arguments, and puts a FileReader or FileWriter object on the
    /// stack as appropriate.
    pub fn opcode_open(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("open requires two arguments");
            return 0;
        }

        let rw_rr = self.stack.pop().unwrap();
        let path_rr = self.stack.pop().unwrap();
        let path_str_opt: Option<&str>;
        to_str!(path_rr, path_str_opt);

        match rw_rr {
            Value::String(st) => match st.borrow().string.as_ref() {
                "r" => match path_str_opt {
                    Some(s) => {
                        let ss = VM::expand_tilde(s);
                        let metadata_res = metadata(ss.clone());
                        match metadata_res {
                            Ok(metadata) => {
                                let is_dir = metadata.is_dir();
                                if !is_dir {
                                    let file_res = File::open(ss);
                                    match file_res {
                                        Ok(file) => {
                                            self.stack.push(Value::FileReader(Rc::new(RefCell::new(
                                                BufReaderWithBuffer::new(
                                                    BufReader::new(file)
                                                )
                                            ))));
                                        }
                                        Err(e) => {
                                            let err_str = format!("unable to open file: {}", e);
                                            self.print_error(&err_str);
                                            return 0;
                                        }
                                    }
                                } else {
                                    let err_str = format!("unable to open file: is a directory");
                                    self.print_error(&err_str);
                                    return 0;
                                }
                            }
                            Err(e) => {
                                let err_str = format!("unable to open file: {}", e);
                                self.print_error(&err_str);
                                return 0;
                            }
                        }
                    }
                    _ => {
                        self.print_error("path for open must be a string");
                        return 0;
                    }
                },
                "w" => match path_str_opt {
                    Some(s) => {
                        let ss = VM::expand_tilde(s);
                        let file_res = File::create(ss);
                        match file_res {
                            Ok(file) => {
                                self.stack.push(Value::FileWriter(Rc::new(RefCell::new(
                                    BufWriter::new(file),
                                ))));
                            }
                            Err(e) => {
                                let err_str = format!("unable to open file: {}", e);
                                self.print_error(&err_str);
                                return 0;
                            }
                        }
                    }
                    _ => {
                        self.print_error("path for open must be a string");
                        return 0;
                    }
                },
                _ => {
                    self.print_error("mode for open must be 'r' or 'w'");
                    return 0;
                }
            },
            _ => {
                self.print_error("mode for open must be 'r' or 'w'");
                return 0;
            }
        }
        1
    }

    /// Takes a FileReader object as its single argument.  Reads one
    /// line from that object and places it onto the stack (including
    /// the ending newline).
    pub fn opcode_readline(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("readline requires one argument");
            return 0;
        }

        let mut file_reader_rr = self.stack.pop().unwrap();

        match file_reader_rr {
            Value::FileReader(ref mut brwb) => {
                let str_res = brwb.borrow_mut().readline();

                match str_res {
                    Some(v) => {
                        self.stack.push(v);
                    }
                    _ => {
                        return 0;
                    }
                }
            }
            Value::TcpSocketReader(ref mut brwb) => {
                loop {
                    let str_res = brwb.borrow_mut().readline();

                    match str_res {
                        Some(v) => {
                            self.stack.push(v);
                            return 1;
                        }
                        _ => {
			    if !self.running.load(Ordering::SeqCst) {
				self.running.store(true, Ordering::SeqCst);
				self.stack.clear();
				return 0;
			    }
                            let dur = time::Duration::from_secs_f64(0.05);
                            thread::sleep(dur);
                        }
                    }
                }
            }
            _ => {
                self.print_error("readline argument must be a file reader");
                return 0;
            }
        }
        1
    }

    /// Takes a FileReader object as its single argument.  Reads the
    /// specified number of bytes from the object and places the list
    /// of bytes onto the stack.
    pub fn opcode_read(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("read requires two arguments");
            return 0;
        }

        let bytes_rr = self.stack.pop().unwrap();
        let file_reader_rr = self.stack.pop().unwrap();

        let bytes_opt = bytes_rr.to_int();

        match (file_reader_rr, bytes_opt) {
            (Value::FileReader(ref mut brwb), Some(n)) => {
                let lst_res = brwb.borrow_mut().read(n as usize);
                match lst_res {
                    Some(lst) => {
                        self.stack.push(lst);
                    }
                    None => {
                        return 0;
                    }
                }
            }
            (Value::TcpSocketReader(ref mut brwb), Some(n)) => {
                loop {
                    let lst_res = brwb.borrow_mut().read(n as usize);

                    match lst_res {
                        Some(lst) => {
                            self.stack.push(lst);
                            return 1;
                        }
                        _ => {
			    if !self.running.load(Ordering::SeqCst) {
				self.running.store(true, Ordering::SeqCst);
				self.stack.clear();
				return 0;
			    }
                            let dur = time::Duration::from_secs_f64(0.05);
                            thread::sleep(dur);
                        }
                    }
                }
            }
            (Value::FileReader(_), _) => {
                self.print_error("second read argument must be an integer");
                return 0;
            }
            _ => {
                self.print_error("first read argument must be a file reader");
                return 0;
            }
        }
        1
    }

    /// Takes a FileWriter object and a list of bytes as its
    /// arguments.  Writes the bytes to the file.
    pub fn core_write(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("write requires two arguments");
            return 0;
        }

        let bytes_rr = self.stack.pop().unwrap();
        let mut file_writer = self.stack.pop().unwrap();

        match bytes_rr {
            Value::List(lst) => {
                let mut bytes = Vec::new();
                for v in lst.borrow().iter() {
                    match v {
                        Value::Byte(b) => {
                            bytes.push(*b);
                        }
                        _ => {
                            self.print_error("second write argument must be list of bytes");
                            return 0;
                        }
                    }
                }
                match file_writer {
                    Value::FileWriter(ref mut line_writer) => {
                        let res =
                            line_writer.borrow_mut().write_all(&bytes);
                        match res {
                            Ok(_) => {
                                return 1;
                            }
                            Err(e) => {
                                let err_str = format!("unable to write to file: {}", e);
                                self.print_error(&err_str);
                                return 0;
                            }
                        }
                    }
                    Value::TcpSocketWriter(ref mut line_writer) => {
                        let res =
                            line_writer.borrow_mut().write_all(&bytes);
                        match res {
                            Ok(_) => {
                                line_writer.borrow_mut().flush().unwrap();
                                return 1;
                            }
                            Err(e) => {
                                let err_str = format!("unable to write to socket: {}", e);
                                self.print_error(&err_str);
                                return 0;
                            }
                        }
                    }
                    _ => {
                        self.print_error("first write argument must be a file writer");
                        return 0;
                    }
                }
            }
            _ => {
                self.print_error("second write argument must be a string");
                return 0;
            }
        };
    }

    /// Takes a FileWriter object and a line as its arguments.  Writes
    /// the line to the file.
    pub fn core_writeline(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("writeline requires two arguments");
            return 0;
        }

        let line_rr = self.stack.pop().unwrap();
        let line_str_opt: Option<&str>;
        to_str!(line_rr, line_str_opt);

        match line_str_opt {
            Some(s) => {
                if !s.is_empty() {
                    let mut file_writer = self.stack.pop().unwrap();
                    match file_writer {
                        Value::FileWriter(ref mut line_writer) => {
                            let res = line_writer.borrow_mut().write_all(s.as_bytes());
                            match res {
                                Ok(_) => {
                                    return 1;
                                }
                                Err(e) => {
                                    let err_str = format!("unable to write to file: {}", e);
                                    self.print_error(&err_str);
                                    return 0;
                                }
                            }
                        }
                        Value::TcpSocketWriter(ref mut line_writer) => {
                            let res = line_writer.borrow_mut().write_all(s.as_bytes());
                            match res {
                                Ok(_) => {
                                    line_writer.borrow_mut().flush().unwrap();
                                    return 1;
                                }
                                Err(e) => {
                                    let err_str = format!("unable to write to socket: {}", e);
                                    self.print_error(&err_str);
                                    return 0;
                                }
                            }
                        }
                        _ => {
                            self.print_error("first writeline argument must be a file writer");
                            return 0;
                        }
                    }
                }
            }
            _ => {
                self.print_error("second writeline argument must be a string");
                return 0;
            }
        };
        1
    }

    /// Takes a FileReader or FileWriter object as its single
    /// argument.  Closes the object, if required.
    pub fn core_close(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("close requires one argument");
            return 0;
        }

        let mut file_rr = self.stack.pop().unwrap();

        match file_rr {
            Value::FileReader(_) => {
                // No action required.
                1
            }
            Value::FileWriter(ref mut line_writer) => {
                let res = line_writer.borrow_mut().flush();
                match res {
                    Ok(_) => 1,
                    Err(e) => {
                        let err_str = format!("unable to flush data: {}", e);
                        self.print_error(&err_str);
                        0
                    }
                }
            }
            _ => {
                self.print_error("close argument must be a file reader or writer");
                0
            }
        }
    }

    /// Takes a directory path as its single argument.  Opens the
    /// directory and places a DirectoryHandle object for the
    /// directory onto the stack.
    pub fn core_opendir(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("opendir requires one argument");
            return 0;
        }

        let path_rr = self.stack.pop().unwrap();
        let path_str_opt: Option<&str>;
        to_str!(path_rr, path_str_opt);

        match path_str_opt {
            Some(s) => {
                let ss = VM::expand_tilde(s);
                let ss2 = TRAILING_SLASHES.replace_all(&ss, "").to_string();
                let dir_handle_res = std::fs::read_dir(ss2);
                match dir_handle_res {
                    Ok(dir_handle) => {
                        self.stack
                            .push(Value::DirectoryHandle(Rc::new(RefCell::new(dir_handle))));
                        1
                    }
                    Err(e) => {
                        let err_str = format!("unable to open directory: {}", e);
                        self.print_error(&err_str);
                        0
                    }
                }
            }
            _ => {
                self.print_error("opendir argument must be a string");
                0
            }
        }
    }

    /// Takes a DirectoryHandle object as its single argument.  Reads
    /// the next entry from the corresponding handle and places it
    /// onto the stack.
    pub fn core_readdir(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("readdir requires one argument");
            return 0;
        }

        let mut dir_handle_rr = self.stack.pop().unwrap();

        let entry_value = match dir_handle_rr {
            Value::DirectoryHandle(ref mut dir_handle) => {
                let entry_opt = dir_handle.borrow_mut().next();
                match entry_opt {
                    Some(s) => {
                        let path = s.unwrap().path();
                        new_string_value(path.to_str().unwrap().to_string())
                    }
                    None => Value::Null,
                }
            }
            _ => {
                self.print_error("readdir argument must be a directory handle");
                return 0;
            }
        };

        self.stack.push(entry_value);
        1
    }

    /// Takes a path as its single argument.  Places a boolean onto
    /// the stack indicating whether the path maps to a directory.
    pub fn core_is_dir(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("is-dir requires one argument");
            return 0;
        }

        let path_rr = self.stack.pop().unwrap();
        let path_str_opt: Option<&str>;
        to_str!(path_rr, path_str_opt);

        match path_str_opt {
            Some(s) => {
                let metadata_res = metadata(s);
                match metadata_res {
                    Ok(metadata) => {
                        let is_dir = metadata.is_dir();
                        self.stack.push(Value::Bool(is_dir));
                    }
                    _ => {
                        self.stack.push(Value::Bool(false));
                    }
                }
            }
            _ => {
                self.print_error("is-dir argument must be a string");
                return 0;
            }
        }
        1
    }

    /// Takes a path as its single argument.  Places a boolean onto
    /// the stack indicating whether the path maps to a file.
    pub fn core_is_file(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("is-file requires one argument");
            return 0;
        }

        let path_rr = self.stack.pop().unwrap();
        let path_str_opt: Option<&str>;
        to_str!(path_rr, path_str_opt);

        match path_str_opt {
            Some(s) => {
                let metadata_res = metadata(s);
                match metadata_res {
                    Ok(metadata) => {
                        let is_file = metadata.is_file();
                        self.stack.push(Value::Bool(is_file));
                    }
                    _ => {
                        self.stack.push(Value::Bool(false));
                    }
                }
            }
            _ => {
                self.print_error("is-file argument must be a string");
                return 0;
            }
        }
        1
    }

    /// Takes a path as its single argument.  Places a boolean onto
    /// the stack indicating whether the path maps to a symbolic link.
    pub fn core_is_link(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("is-link requires one argument");
            return 0;
        }

        let path_rr = self.stack.pop().unwrap();
        let path_str_opt: Option<&str>;
        to_str!(path_rr, path_str_opt);

        match path_str_opt {
            Some(s) => {
                let metadata_res = symlink_metadata(s);
                match metadata_res {
                    Ok(metadata) => {
                        let is_link = metadata.is_symlink();
                        self.stack.push(Value::Bool(is_link));
                    }
                    _ => {
                        self.stack.push(Value::Bool(false));
                    }
                }
            }
            _ => {
                self.print_error("is-link argument must be a string");
                return 0;
            }
        }
        1
    }

    /// Takes a path as its single argument.  Places a boolean onto
    /// the stack indicating whether the path is readable.
    pub fn core_is_r(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("is-r requires one argument");
            return 0;
        }

        let path_rr = self.stack.pop().unwrap();
        let path_str_opt: Option<&str>;
        to_str!(path_rr, path_str_opt);

        match path_str_opt {
            Some(s) => {
                let is_readable =
                    nix::unistd::access(s, AccessFlags::R_OK).is_ok();
                self.stack.push(Value::Bool(is_readable));
            }
            _ => {
                self.print_error("is-r argument must be a string");
                return 0;
            }
        }
        1
    }

    /// Takes a path as its single argument.  Places a boolean onto
    /// the stack indicating whether the path is writable.
    pub fn core_is_w(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("is-w requires one argument");
            return 0;
        }

        let path_rr = self.stack.pop().unwrap();
        let path_str_opt: Option<&str>;
        to_str!(path_rr, path_str_opt);

        match path_str_opt {
            Some(s) => {
                let is_writable =
                    nix::unistd::access(s, AccessFlags::W_OK).is_ok();
                self.stack.push(Value::Bool(is_writable));
            }
            _ => {
                self.print_error("is-w argument must be a string");
                return 0;
            }
        }
        1
    }

    /// Takes a path as its single argument.  Places a boolean onto
    /// the stack indicating whether the path is executable.
    pub fn core_is_x(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("is-x requires one argument");
            return 0;
        }

        let path_rr = self.stack.pop().unwrap();
        let path_str_opt: Option<&str>;
        to_str!(path_rr, path_str_opt);

        match path_str_opt {
            Some(s) => {
                let is_executable =
                    nix::unistd::access(s, AccessFlags::X_OK).is_ok();
                self.stack.push(Value::Bool(is_executable));
            }
            _ => {
                self.print_error("is-x argument must be a string");
                return 0;
            }
        }
        1
    }

    /// Puts a path and a FileReader on the stack for a new temporary
    /// file.
    pub fn opcode_tempfile(&mut self) -> i32 {
        let file_res = NamedTempFile::new();

        match file_res {
            Ok(ntf) => match ntf.keep() {
                Ok((file, path)) => {
                    self.stack
                        .push(new_string_value(path.to_str().unwrap().to_string()));
                    self.stack
                        .push(Value::FileWriter(Rc::new(RefCell::new(BufWriter::new(
                            file,
                        )))));
                    1
                }
                Err(e) => {
                    let err_str = format!("unable to open temporary file: {}", e);
                    self.print_error(&err_str);
                    0
                }
            },
            Err(e) => {
                let err_str = format!("unable to open temporary file: {}", e);
                self.print_error(&err_str);
                0
            }
        }
    }

    /// Puts a path on the stack for a new temporary directory.
    pub fn opcode_tempdir(&mut self) -> i32 {
        let dir = TempDir::new();

        match dir {
            Ok(td) => {
                let path = td.into_path();
                self.stack
                    .push(new_string_value(path.to_str().unwrap().to_string()));
                1
            }
            Err(e) => {
                let err_str = format!("unable to open temporary directory: {}", e);
                self.print_error(&err_str);
                0
            }
        }
    }
}

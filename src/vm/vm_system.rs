use nix::sys::signal::Signal;
use nix::unistd::Pid;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::rc::Rc;
use std::time::SystemTime;

use indexmap::IndexMap;
use num::FromPrimitive;
use num_bigint::BigInt;
use sysinfo::{ProcessExt, SystemExt};
use utime::*;

use chunk::{print_error, Chunk, StringPair, Value};
use vm::*;

impl VM {
    /// Takes a value that can be stringified as its single argument.
    /// Removes the file corresponding to that path.
    pub fn core_rm(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "rm requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let value_s;
        let value_b;
        let value_str;
        let value_bk: Option<String>;
        let value_opt: Option<&str> = match value_rr {
            Value::String(sp) => {
                value_s = sp;
                value_b = value_s.borrow();
                Some(&value_b.s)
            }
            _ => {
                value_bk = value_rr.to_string();
                match value_bk {
                    Some(s) => {
                        value_str = s;
                        Some(&value_str)
                    }
                    _ => None,
                }
            }
        };

        match value_opt {
            Some(s) => {
                let res = std::fs::remove_file(s);
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        let err_str = format!("unable to remove file: {}", e.to_string());
                        print_error(chunk, i, &err_str);
                        return 0;
                    }
                }
            }
            _ => {
                print_error(chunk, i, "rm argument must be a string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes two values that can be stringified as its arguments.
    /// Copies the file corresponding to the first path to the second
    /// path.
    pub fn core_cp(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "cp requires two arguments");
            return 0;
        }

        let dst_rr = self.stack.pop().unwrap();
        let dst_s;
        let dst_b;
        let dst_str;
        let dst_bk: Option<String>;
        let dst_opt: Option<&str> = match dst_rr {
            Value::String(sp) => {
                dst_s = sp;
                dst_b = dst_s.borrow();
                Some(&dst_b.s)
            }
            _ => {
                dst_bk = dst_rr.to_string();
                match dst_bk {
                    Some(s) => {
                        dst_str = s;
                        Some(&dst_str)
                    }
                    _ => None,
                }
            }
        };

        let src_rr = self.stack.pop().unwrap();
        let src_s;
        let src_b;
        let src_str;
        let src_bk: Option<String>;
        let src_opt: Option<&str> = match src_rr {
            Value::String(sp) => {
                src_s = sp;
                src_b = src_s.borrow();
                Some(&src_b.s)
            }
            _ => {
                src_bk = src_rr.to_string();
                match src_bk {
                    Some(s) => {
                        src_str = s;
                        Some(&src_str)
                    }
                    _ => None,
                }
            }
        };

        match (src_opt, dst_opt) {
            (Some(src), Some(dst)) => {
                let res = std::fs::copy(src, dst);
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        let err_str = format!("unable to copy file: {}", e.to_string());
                        print_error(chunk, i, &err_str);
                        return 0;
                    }
                }
            }
            _ => {
                print_error(chunk, i, "source and destination must be strings");
                return 0;
            }
        }
        return 1;
    }

    /// Takes two values that can be stringified as its arguments.
    /// Moves the file corresponding to the first path to the second
    /// path.  (Not quite the same semantics as mv(1), because it uses
    /// rename(2) underneath, so that should be fixed.)
    pub fn core_mv(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "mv requires two arguments");
            return 0;
        }

        let dst_rr = self.stack.pop().unwrap();
        let dst_s;
        let dst_b;
        let dst_str;
        let dst_bk: Option<String>;
        let dst_opt: Option<&str> = match dst_rr {
            Value::String(sp) => {
                dst_s = sp;
                dst_b = dst_s.borrow();
                Some(&dst_b.s)
            }
            _ => {
                dst_bk = dst_rr.to_string();
                match dst_bk {
                    Some(s) => {
                        dst_str = s;
                        Some(&dst_str)
                    }
                    _ => None,
                }
            }
        };

        let src_rr = self.stack.pop().unwrap();
        let src_s;
        let src_b;
        let src_str;
        let src_bk: Option<String>;
        let src_opt: Option<&str> = match src_rr {
            Value::String(sp) => {
                src_s = sp;
                src_b = src_s.borrow();
                Some(&src_b.s)
            }
            _ => {
                src_bk = src_rr.to_string();
                match src_bk {
                    Some(s) => {
                        src_str = s;
                        Some(&src_str)
                    }
                    _ => None,
                }
            }
        };

        match (src_opt, dst_opt) {
            (Some(src), Some(dst)) => {
                let res = std::fs::rename(src, dst);
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        let err_str = format!("unable to move file: {}", e.to_string());
                        print_error(chunk, i, &err_str);
                        return 0;
                    }
                }
            }
            _ => {
                print_error(chunk, i, "source and destination must be strings");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a value that can be stringified as its single argument.
    /// Changes the current working directory to that directory.  If
    /// no arguments are provided, then this changes the current
    /// working directory to the user's home directory.
    pub fn core_cd(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() == 0 {
            let home_res = std::env::var("HOME");
            match home_res {
                Ok(home) => {
                    let res = env::set_current_dir(&home);
                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            let err_str = format!("unable to cd to home: {}", e.to_string());
                            print_error(chunk, i, &err_str);
                            return 0;
                        }
                    }
                }
                Err(e) => {
                    let err_str = format!("unable to cd to home: {}", e.to_string());
                    print_error(chunk, i, &err_str);
                    return 0;
                }
            }
        } else {
            let dir_rr = self.stack.pop().unwrap();
            let dir_s;
            let dir_b;
            let dir_str;
            let dir_bk: Option<String>;
            let dir_opt: Option<&str> = match dir_rr {
                Value::String(sp) => {
                    dir_s = sp;
                    dir_b = dir_s.borrow();
                    Some(&dir_b.s)
                }
                _ => {
                    dir_bk = dir_rr.to_string();
                    match dir_bk {
                        Some(s) => {
                            dir_str = s;
                            Some(&dir_str)
                        }
                        _ => None,
                    }
                }
            };

            match dir_opt {
                Some(dir) => {
                    let path_dir = Path::new(&dir);
                    let res = env::set_current_dir(&path_dir);
                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            let err_str = format!("unable to cd: {}", e.to_string());
                            print_error(chunk, i, &err_str);
                            return 0;
                        }
                    }
                }
                _ => {
                    print_error(chunk, i, "cd argument must be a string");
                    return 0;
                }
            }
        }
        return 1;
    }

    /// Puts the string representation of the current working
    /// directory onto the stack.
    pub fn core_pwd(&mut self, chunk: &Chunk, i: usize) -> i32 {
        let current_dir_res = std::env::current_dir();
        match current_dir_res {
            Ok(current_dir) => {
                self.stack
                    .push(Value::String(Rc::new(RefCell::new(StringPair::new(
                        current_dir.to_str().unwrap().to_string(),
                        None,
                    )))));
            }
            Err(e) => {
                let err_str = format!("unable to pwd: {}", e.to_string());
                print_error(chunk, i, &err_str);
                return 0;
            }
        }
        return 1;
    }

    /// Takes a value that can be stringified as its single argument.
    /// Creates the file if it doesn't exist, and updates its
    /// modification timestamp to the current time if it does exist,
    /// similarly to touch(1).
    pub fn core_touch(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "touch requires one argument");
            return 0;
        }

        let path_rr = self.stack.pop().unwrap();
        let path_s;
        let path_b;
        let path_str;
        let path_bk: Option<String>;
        let path_opt: Option<&str> = match path_rr {
            Value::String(sp) => {
                path_s = sp;
                path_b = path_s.borrow();
                Some(&path_b.s)
            }
            _ => {
                path_bk = path_rr.to_string();
                match path_bk {
                    Some(s) => {
                        path_str = s;
                        Some(&path_str)
                    }
                    _ => None,
                }
            }
        };

        match path_opt {
            Some(path_str) => {
                let path = Path::new(&path_str);
                if !path.exists() {
                    let res = fs::write(&path_str, "");
                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            let err_str = format!("unable to write file: {}", e.to_string());
                            print_error(chunk, i, &err_str);
                            return 0;
                        }
                    }
                } else {
                    let times_res = get_file_times(&path_str);
                    match times_res {
                        Ok((accessed, _)) => {
                            let mtime = SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap()
                                .as_secs();
                            let sft_res = set_file_times(&path_str, accessed, mtime as i64);
                            match sft_res {
                                Ok(_) => {}
                                Err(e) => {
                                    let err_str =
                                        format!("unable to write file: {}", e.to_string());
                                    print_error(chunk, i, &err_str);
                                    return 0;
                                }
                            }
                        }
                        Err(e) => {
                            let err_str = format!("unable to write file: {}", e.to_string());
                            print_error(chunk, i, &err_str);
                            return 0;
                        }
                    }
                }
            }
            _ => {
                print_error(chunk, i, "touch argument must be a string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a value that can be stringified as its single argument.
    /// Puts a hash onto the stack containing the metadata of the
    /// associated file, where "dev" is the device number, "ino" is
    /// the inode, "mode" is the file mode, "nlink" is the number of
    /// hard links to the file, "uid" is the user ID of the owner,
    /// "gid" is the group ID of the owner, "rdev" is the device ID
    /// (for special files), "size" is the total size in bytes,
    /// "atime_nsec"/"ctime_nsec"/"mtime_nsec" are various file
    /// modification times, "blksize" is the block size, and "blocks"
    /// is the number of blocks allocated to the file.
    pub fn core_stat(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "stat requires one argument");
            return 0;
        }

        let path_rr = self.stack.pop().unwrap();
        let path_s;
        let path_b;
        let path_str;
        let path_bk: Option<String>;
        let path_opt: Option<&str> = match path_rr {
            Value::String(sp) => {
                path_s = sp;
                path_b = path_s.borrow();
                Some(&path_b.s)
            }
            _ => {
                path_bk = path_rr.to_string();
                match path_bk {
                    Some(s) => {
                        path_str = s;
                        Some(&path_str)
                    }
                    _ => None,
                }
            }
        };

        match path_opt {
            Some(s) => {
                let meta_res = fs::metadata(&s);
                match meta_res {
                    Ok(meta) => {
                        let mut map = IndexMap::new();
                        map.insert(
                            "dev".to_string(),
                            Value::BigInt(BigInt::from_u64(meta.dev()).unwrap()),
                        );
                        map.insert(
                            "ino".to_string(),
                            Value::BigInt(BigInt::from_u64(meta.ino()).unwrap()),
                        );
                        map.insert(
                            "mode".to_string(),
                            Value::BigInt(BigInt::from_u32(meta.mode()).unwrap()),
                        );
                        map.insert(
                            "nlink".to_string(),
                            Value::BigInt(BigInt::from_u64(meta.nlink()).unwrap()),
                        );
                        map.insert(
                            "uid".to_string(),
                            Value::BigInt(BigInt::from_u32(meta.uid()).unwrap()),
                        );
                        map.insert(
                            "gid".to_string(),
                            Value::BigInt(BigInt::from_u32(meta.gid()).unwrap()),
                        );
                        map.insert(
                            "rdev".to_string(),
                            Value::BigInt(BigInt::from_u64(meta.rdev()).unwrap()),
                        );
                        map.insert(
                            "size".to_string(),
                            Value::BigInt(BigInt::from_u64(meta.size()).unwrap()),
                        );
                        map.insert(
                            "atime_nsec".to_string(),
                            Value::BigInt(BigInt::from_i64(meta.atime_nsec()).unwrap()),
                        );
                        map.insert(
                            "mtime_nsec".to_string(),
                            Value::BigInt(BigInt::from_i64(meta.mtime_nsec()).unwrap()),
                        );
                        map.insert(
                            "ctime_nsec".to_string(),
                            Value::BigInt(BigInt::from_i64(meta.ctime_nsec()).unwrap()),
                        );
                        map.insert(
                            "blksize".to_string(),
                            Value::BigInt(BigInt::from_u64(meta.blksize()).unwrap()),
                        );
                        map.insert(
                            "blocks".to_string(),
                            Value::BigInt(BigInt::from_u64(meta.blocks()).unwrap()),
                        );
                        self.stack.push(Value::Hash(Rc::new(RefCell::new(map))));
                    }
                    Err(e) => {
                        let err_str = format!("unable to stat file: {}", e.to_string());
                        print_error(chunk, i, &err_str);
                        return 0;
                    }
                }
            }
            _ => {
                print_error(chunk, i, "stat argument must be a string");
                return 0;
            }
        }
        return 1;
    }

    /// Puts current process information onto the stack, in the form
    /// of a list of hashes.  Each hash has elements for "pid", "uid",
    /// and "name".
    #[allow(unused_variables)]
    pub fn core_ps(&mut self, chunk: &Chunk, i: usize) -> i32 {
        let sys = &mut self.sys;
        sys.refresh_processes();

        let mut lst = VecDeque::new();
        for (pid, process) in self.sys.processes() {
            let mut map = IndexMap::new();
            map.insert(
                "pid".to_string(),
                Value::BigInt(BigInt::from_i32(*pid).unwrap()),
            );
            map.insert(
                "uid".to_string(),
                Value::BigInt(BigInt::from_u32(process.uid).unwrap()),
            );
            map.insert(
                "name".to_string(),
                Value::String(Rc::new(RefCell::new(StringPair::new(
                    process.name().to_string(),
                    None,
                )))),
            );
            lst.push_back(Value::Hash(Rc::new(RefCell::new(map))))
        }
        self.stack.push(Value::List(Rc::new(RefCell::new(lst))));
        return 1;
    }

    /// Takes a process identifier and a signal name as its arguments.
    /// Sends the relevant signal to the process.
    pub fn core_kill(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "kill requires two arguments");
            return 0;
        }

        let sig_rr = self.stack.pop().unwrap();
        let sig_s;
        let sig_b;
        let sig_str;
        let sig_bk: Option<String>;
        let sig_opt: Option<&str> = match sig_rr {
            Value::String(sp) => {
                sig_s = sp;
                sig_b = sig_s.borrow();
                Some(&sig_b.s)
            }
            _ => {
                sig_bk = sig_rr.to_string();
                match sig_bk {
                    Some(s) => {
                        sig_str = s;
                        Some(&sig_str)
                    }
                    _ => None,
                }
            }
        };

        let pid_rr = self.stack.pop().unwrap();
        let pid_int_opt = pid_rr.to_int();

        match (pid_int_opt, sig_opt) {
            (Some(pid), Some(sig)) => {
                let sig_lc = sig.to_lowercase();
                let sig_obj = match &sig_lc[..] {
                    "hup" => Signal::SIGHUP,
                    "int" => Signal::SIGINT,
                    "term" => Signal::SIGTERM,
                    "kill" => Signal::SIGKILL,
                    "usr1" => Signal::SIGUSR1,
                    "usr2" => Signal::SIGUSR2,
                    "cont" => Signal::SIGCONT,
                    "stop" => Signal::SIGSTOP,
                    _ => {
                        print_error(chunk, i, "invalid signal");
                        return 0;
                    }
                };
                let res = nix::sys::signal::kill(Pid::from_raw(pid), sig_obj);
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        let err_str = format!("unable to kill process: {}", e.to_string());
                        print_error(chunk, i, &err_str);
                        return 0;
                    }
                }
                return 1;
            }
            (_, Some(_)) => {
                print_error(chunk, i, "first kill argument must be process");
                return 0;
            }
            (_, _) => {
                print_error(chunk, i, "second kill argument must be signal");
                return 0;
            }
        }
    }
}

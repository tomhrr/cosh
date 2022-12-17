use nix::sys::signal::Signal;
use nix::unistd::{Group, User, Pid};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::rc::Rc;
use std::time::SystemTime;

use indexmap::IndexMap;
use num::FromPrimitive;
use num_bigint::BigInt;
use sysinfo::{ProcessExt, SystemExt};
use utime::*;

use chunk::{StringPair, Value};
use vm::*;

impl VM {
    /// Takes a value that can be stringified as its single argument.
    /// Removes the file corresponding to that path.
    pub fn core_rm(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("rm requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
	let value_opt: Option<&str>;
	to_str!(value_rr, value_opt);

        match value_opt {
            Some(s) => {
                let res = std::fs::remove_file(s);
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        let err_str = format!("unable to remove file: {}", e.to_string());
                        self.print_error(&err_str);
                        return 0;
                    }
                }
            }
            _ => {
                self.print_error("rm argument must be a string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes two values that can be stringified as its arguments.
    /// Copies the file corresponding to the first path to the second
    /// path.
    pub fn core_cp(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("cp requires two arguments");
            return 0;
        }

        let dst_rr = self.stack.pop().unwrap();
	let dst_opt: Option<&str>;
	to_str!(dst_rr, dst_opt);

        let src_rr = self.stack.pop().unwrap();
	let src_opt: Option<&str>;
	to_str!(src_rr, src_opt);

        match (src_opt, dst_opt) {
            (Some(src), Some(dst)) => {
                let res = std::fs::copy(src, dst);
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        let err_str = format!("unable to copy file: {}", e.to_string());
                        self.print_error(&err_str);
                        return 0;
                    }
                }
            }
            _ => {
                self.print_error("source and destination must be strings");
                return 0;
            }
        }
        return 1;
    }

    /// Takes two values that can be stringified as its arguments.
    /// Moves the file corresponding to the first path to the second
    /// path.
    pub fn core_mv(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("mv requires two arguments");
            return 0;
        }

        let dst_rr = self.stack.pop().unwrap();
	let dst_opt: Option<&str>;
	to_str!(dst_rr, dst_opt);

        let src_rr = self.stack.pop().unwrap();
	let src_opt: Option<&str>;
	to_str!(src_rr, src_opt);

        match (src_opt, dst_opt) {
            (Some(src), Some(dst)) => {
                let res = std::fs::copy(src, dst);
                match res {
                    Ok(_) => {
                        let res = std::fs::remove_file(src);
                        match res {
                            Ok(_) => {
                                return 1;
                            }
                            Err(e) => {
                                let err_str = format!(
                                    "unable to remove original file: {}",
                                    e.to_string()
                                );
                                self.print_error(&err_str);
                                return 0;
                            }
                        }
                    }
                    Err(e) => {
                        let err_str = format!(
                            "unable to copy file to destination: {}",
                            e.to_string()
                        );
                        self.print_error(&err_str);
                        return 0;
                    }
                }
            }
            _ => {
                self.print_error("source and destination must be strings");
                return 0;
            }
        }
    }

    /// Takes two values that can be stringified as its arguments.
    /// Renames the file with the first path such that it has the
    /// second path.  (The two paths have to be on the same filesystem
    /// for this to work correctly.  If they aren't, see core_mv.)
    pub fn core_rename(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("mv requires two arguments");
            return 0;
        }

        let dst_rr = self.stack.pop().unwrap();
	let dst_opt: Option<&str>;
	to_str!(dst_rr, dst_opt);

        let src_rr = self.stack.pop().unwrap();
	let src_opt: Option<&str>;
	to_str!(src_rr, src_opt);

        match (src_opt, dst_opt) {
            (Some(src), Some(dst)) => {
		let res = std::fs::rename(src, dst);
                match res {
                    Ok(_) => {
                        return 1;
                    }
                    Err(e) => {
                        let err_str = format!(
                            "unable to rename file: {}",
                            e.to_string()
                        );
                        self.print_error(&err_str);
                        return 0;
                    }
                }
            }
            _ => {
                self.print_error("source and destination must be strings");
                return 0;
            }
        }
    }

    /// Takes a value that can be stringified as its single argument.
    /// Changes the current working directory to that directory.  If
    /// no arguments are provided, then this changes the current
    /// working directory to the user's home directory.
    pub fn core_cd(&mut self) -> i32 {
        if self.stack.len() == 0 {
            let home_res = std::env::var("HOME");
            match home_res {
                Ok(home) => {
                    let res = env::set_current_dir(&home);
                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            let err_str = format!("unable to cd to home: {}", e.to_string());
                            self.print_error(&err_str);
                            return 0;
                        }
                    }
                }
                Err(e) => {
                    let err_str = format!("unable to cd to home: {}", e.to_string());
                    self.print_error(&err_str);
                    return 0;
                }
            }
        } else {
	    let dir_rr = self.stack.pop().unwrap();
	    let dir_opt: Option<&str>;
	    to_str!(dir_rr, dir_opt);

            match dir_opt {
                Some(dir) => {
                    let path_dir = Path::new(&dir);
                    let res = env::set_current_dir(&path_dir);
                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            let err_str = format!("unable to cd: {}", e.to_string());
                            self.print_error(&err_str);
                            return 0;
                        }
                    }
                }
                _ => {
                    self.print_error("cd argument must be a string");
                    return 0;
                }
            }
        }
        return 1;
    }

    /// Puts the string representation of the current working
    /// directory onto the stack.
    pub fn core_pwd(&mut self) -> i32 {
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
                self.print_error(&err_str);
                return 0;
            }
        }
        return 1;
    }

    /// Takes a value that can be stringified as its single argument.
    /// Creates the file if it doesn't exist, and updates its
    /// modification timestamp to the current time if it does exist,
    /// similarly to touch(1).
    pub fn core_touch(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("touch requires one argument");
            return 0;
        }

	let path_rr = self.stack.pop().unwrap();
	let path_opt: Option<&str>;
	to_str!(path_rr, path_opt);

        match path_opt {
            Some(path_str) => {
                let path = Path::new(&path_str);
                if !path.exists() {
                    let res = fs::write(&path_str, "");
                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            let err_str = format!("unable to write file: {}", e.to_string());
                            self.print_error(&err_str);
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
                                    self.print_error(&err_str);
                                    return 0;
                                }
                            }
                        }
                        Err(e) => {
                            let err_str = format!("unable to write file: {}", e.to_string());
                            self.print_error(&err_str);
                            return 0;
                        }
                    }
                }
            }
            _ => {
                self.print_error("touch argument must be a string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a value that can be stringified and a boolean indicating
    /// whether to use the link itself or its target (if the value is
    /// a link) as its arguments.  Puts a hash onto the stack
    /// containing the metadata of the associated file, where "dev" is
    /// the device number, "ino" is the inode, "mode" is the file
    /// mode, "nlink" is the number of hard links to the file, "uid"
    /// is the user ID of the owner, "gid" is the group ID of the
    /// owner, "rdev" is the device ID (for special files), "size" is
    /// the total size in bytes,
    /// "atime_nsec"/"ctime_nsec"/"mtime_nsec" are various file
    /// modification times, "blksize" is the block size, and "blocks"
    /// is the number of blocks allocated to the file.
    fn stat_inner(&mut self, use_symlink: bool) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("stat requires one argument");
            return 0;
        }

	let path_rr = self.stack.pop().unwrap();
	let path_opt: Option<&str>;
	to_str!(path_rr, path_opt);

        match path_opt {
            Some(s) => {
                let meta_res =
                    if use_symlink {
                        fs::symlink_metadata(&s)
                    } else {
                        fs::metadata(&s)
                    };
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
                        self.print_error(&err_str);
                        return 0;
                    }
                }
            }
            _ => {
                self.print_error("stat argument must be a string");
                return 0;
            }
        }
        return 1;
    }

    /// See stat_inner.
    pub fn core_stat(&mut self) -> i32 {
        return self.stat_inner(false);
    }

    /// See stat_inner.
    pub fn core_lstat(&mut self) -> i32 {
        return self.stat_inner(true);
    }

    /// Puts current process information onto the stack, in the form
    /// of a list of hashes.  Each hash has elements for "pid", "uid",
    /// and "name".
    #[allow(unused_variables)]
    pub fn core_ps(&mut self) -> i32 {
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
    pub fn core_kill(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("kill requires two arguments");
            return 0;
        }

	let sig_rr = self.stack.pop().unwrap();
	let sig_opt: Option<&str>;
	to_str!(sig_rr, sig_opt);

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
                        self.print_error("invalid signal");
                        return 0;
                    }
                };
                let res = nix::sys::signal::kill(Pid::from_raw(pid), sig_obj);
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        let err_str = format!("unable to kill process: {}", e.to_string());
                        self.print_error(&err_str);
                        return 0;
                    }
                }
                return 1;
            }
            (_, Some(_)) => {
                self.print_error("first kill argument must be process");
                return 0;
            }
            (_, _) => {
                self.print_error("second kill argument must be signal");
                return 0;
            }
        }
    }

    /// Takes a path and a numeric mode as its arguments, and updates
    /// the path's mode accordingly.
    pub fn core_chmod(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("chmod requires two arguments");
            return 0;
        }

	let mode_rr = self.stack.pop().unwrap();
        let mode_opt = mode_rr.to_int();

        let path_rr = self.stack.pop().unwrap();
        let path_opt: Option<&str>;
	to_str!(path_rr, path_opt);

        match (path_opt, mode_opt) {
            (Some(path), Some(mode)) => {
                let f_opt = fs::metadata(&path);
                if f_opt.is_err() {
                    self.print_error("unable to get metadata for path");
                    return 0;
                }
                let f = f_opt.unwrap();
                let mut perms = f.permissions();
                perms.set_mode(mode.try_into().unwrap());
                let res = fs::set_permissions(&path, perms);
                match res {
                    Ok(_) => {
                        return 1;
                    }
                    Err(e) => {
                        let s = format!("unable to chmod: {}", e);
                        self.print_error(&s);
                        return 0;
                    }
                }
            }
            (Some(_), _) => {
                self.print_error("second chmod argument must be mode");
                return 0;
            }
            (_, _) => {
                self.print_error("first chmod argument must be path");
                return 0;
            }
        }
    }

    /// Takes a path, a user name, and a group name, and updates the
    /// ownership of the path accordingly.
    pub fn core_chown(&mut self) -> i32 {
        if self.stack.len() < 3 {
            self.print_error("chown requires three arguments");
            return 0;
        }

        let group_rr = self.stack.pop().unwrap();
        let group_opt: Option<&str>;
	to_str!(group_rr, group_opt);

        let user_rr = self.stack.pop().unwrap();
        let user_opt: Option<&str>;
	to_str!(user_rr, user_opt);

        let path_rr = self.stack.pop().unwrap();
        let path_opt: Option<&str>;
	to_str!(path_rr, path_opt);

        match (path_opt, user_opt, group_opt) {
            (Some(path), Some(user), Some(group)) => {
                let user_opt_res = User::from_name(user);
                if user_opt_res.is_err() {
                    self.print_error("user not found");
                    return 0;
                }
                let user_opt = user_opt_res.unwrap();
                if user_opt.is_none() {
                    self.print_error("user not found");
                    return 0;
                }
                let user_obj = user_opt.unwrap();

                let group_opt_res = Group::from_name(group);
                if group_opt_res.is_err() {
                    self.print_error("group not found");
                    return 0;
                }
                let group_opt = group_opt_res.unwrap();
                if group_opt.is_none() {
                    self.print_error("group not found");
                    return 0;
                }
                let group_obj = group_opt.unwrap();

                let chown_res =
                    nix::unistd::chown(path, Some(user_obj.uid),
                                             Some(group_obj.gid));
                match chown_res {
                    Ok(_) => {
                        return 1;
                    }
                    Err(e) => {
                        let s = format!("unable to chown path: {}", e);
                        self.print_error(&s);
                        return 0;
                    }
                }
            }
            (Some(_), Some(_), _) => {
                self.print_error("third chown argument must be group");
                return 0;
            }
            (Some(_), _, _) => {
                self.print_error("second chown argument must be user");
                return 0;
            }
            (_, _, _) => {
                self.print_error("first chown argument must be path");
                return 0;
            }
        }
    }

    /// Takes a path as its single argument, and attempts to make a
    /// directory at that path.
    pub fn core_mkdir(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("mkdir requires one argument");
            return 0;
        }

        let dir_rr = self.stack.pop().unwrap();
        let dir_opt: Option<&str>;
	to_str!(dir_rr, dir_opt);

        match dir_opt {
            Some(dir) => {
                let res = std::fs::create_dir(dir);
                match res {
                    Ok(_) => {
                        return 1;
                    }
                    Err(e) => {
                        let s = format!("unable to make directory: {}", e);
                        self.print_error(&s);
                        return 0;
                    }
                }
            }
            None => {
                self.print_error("first mkdir argument must be path");
                return 0;
            }
        }
    }

    /// Takes a path as its single argument, and attempts to remove
    /// the directory at that path.
    pub fn core_rmdir(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("rmdir requires one argument");
            return 0;
        }

        let dir_rr = self.stack.pop().unwrap();
        let dir_opt: Option<&str>;
	to_str!(dir_rr, dir_opt);

        match dir_opt {
            Some(dir) => {
                let res = std::fs::remove_dir(dir);
                match res {
                    Ok(_) => {
                        return 1;
                    }
                    Err(e) => {
                        let s = format!("unable to remove directory: {}", e);
                        self.print_error(&s);
                        return 0;
                    }
                }
            }
            None => {
                self.print_error("first rmdir argument must be path");
                return 0;
            }
        }
    }
}

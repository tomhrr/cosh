use nix::sys::signal::Signal;
use nix::unistd::{Group, Pid, User};
use std::cell::RefCell;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::env;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::rc::Rc;
use std::time::SystemTime;

use chrono::{DateTime, NaiveDateTime, Utc};
use indexmap::IndexMap;
use num::FromPrimitive;
use num_bigint::BigInt;
use sysinfo::{PidExt, ProcessExt, SystemExt, CpuRefreshKind, UserExt};
use utime::*;

use crate::chunk::{StringTriple, Value};
use crate::vm::*;

impl VM {
    /// Takes a value that can be stringified as its single argument.
    /// Removes the file corresponding to that path.
    pub fn core_rm(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("rm requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let value_opt: Option<&str>;
        to_str!(value_rr, value_opt);

        match value_opt {
            Some(s) => {
                let ss = VM::expand_tilde(s);
                let res = std::fs::remove_file(ss);
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        let err_str = format!("unable to remove file: {}", e);
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
        1
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
                let srcs = VM::expand_tilde(src);
                let dsts = VM::expand_tilde(dst);
                let dst_meta_opt = fs::metadata(&dsts);
                let dst_path =
                    if !dst_meta_opt.is_err() {
                        let dst_meta = dst_meta_opt.unwrap();
                        if dst_meta.is_dir() {
                            let src_path = Path::new(&srcs);
                            let file_name = src_path.file_name();
                            match file_name {
                                Some(s) => {
                                    format!("{}/{}", dsts, s.to_str().unwrap())
                                }
                                None => {
                                    self.print_error("unable to copy directory to directory");
                                    return 0;
                                }
                            }
                        } else {
                            dsts.to_string()
                        }
                    } else {
                        dsts.to_string()
                    };
                let res = std::fs::copy(srcs, dst_path);
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        let err_str = format!("unable to copy file: {}", e);
                        self.print_error(&err_str);
                        return 0;
                    }
                }
            }
            (Some(_), _) => {
                self.print_error("second cp argument must be string");
                return 0;
            }
            _ => {
                self.print_error("first cp argument must be string");
                return 0;
            }
        }
        1
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
                self.stack.push(
                    Value::String(
                        Rc::new(
                            RefCell::new(
                                StringTriple::new(src.to_string(), None)
                            )
                        )
                    )
                );
                self.stack.push(
                    Value::String(
                        Rc::new(
                            RefCell::new(
                                StringTriple::new(dst.to_string(), None)
                            )
                        )
                    )
                );
                let res = self.core_cp();
                if res == 0 {
                    return 0;
                }
                let srcs = VM::expand_tilde(src);
                let res = std::fs::remove_file(srcs);
                match res {
                    Ok(_) => 1,
                    Err(e) => {
                        let err_str = format!("unable to remove original file: {}", e);
                        self.print_error(&err_str);
                        0
                    }
                }
            }
            (Some(_), _) => {
                self.print_error("second mv argument must be string");
                0
            }
            _ => {
                self.print_error("first mv argument must be string");
                0
            }
        }
    }

    /// Takes two values that can be stringified as its arguments.
    /// Renames the file with the first path such that it has the
    /// second path.  (The two paths have to be on the same filesystem
    /// for this to work correctly.  If they aren't, see core_mv.)
    pub fn core_rename(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("rename requires two arguments");
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
                let srcs = VM::expand_tilde(src);
                let dsts = VM::expand_tilde(dst);
                let res = std::fs::rename(srcs, dsts);
                match res {
                    Ok(_) => 1,
                    Err(e) => {
                        let err_str = format!("unable to rename file: {}", e);
                        self.print_error(&err_str);
                        0
                    }
                }
            }
            (Some(_), _) => {
                self.print_error("second rename argument must be string");
                0
            }
            _ => {
                self.print_error("first rename argument must be string");
                0
            }
        }
    }

    /// Takes two values that can be stringified as its arguments.
    /// Creates a symbolic link from the second path to the first
    /// path.
    pub fn core_link(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("link requires two arguments");
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
                let srcs = VM::expand_tilde(src);
                let dsts = VM::expand_tilde(dst);
                let dst_meta_opt = fs::metadata(&dsts);
                let dst_path =
                    if !dst_meta_opt.is_err() {
                        let dst_meta = dst_meta_opt.unwrap();
                        if dst_meta.is_dir() {
                            let src_path = Path::new(&srcs);
                            let file_name = src_path.file_name();
                            match file_name {
                                Some(s) => {
                                    format!("{}/{}", dsts, s.to_str().unwrap())
                                }
                                None => {
                                    self.print_error("unable to copy directory to directory");
                                    return 0;
                                }
                            }
                        } else {
                            dsts.to_string()
                        }
                    } else {
                        dsts.to_string()
                    };
                let res = std::os::unix::fs::symlink(srcs, dst_path);
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        let err_str = format!("unable to create symbolic link: {}", e);
                        self.print_error(&err_str);
                        return 0;
                    }
                }
            }
            (Some(_), _) => {
                self.print_error("second link argument must be string");
                return 0;
            }
            _ => {
                self.print_error("first link argument must be string");
                return 0;
            }
        }
        1
    }

    /// Takes a value that can be stringified as its single argument.
    /// Changes the current working directory to that directory.  If
    /// no arguments are provided, then this changes the current
    /// working directory to the user's home directory.
    pub fn core_cd(&mut self) -> i32 {
        if self.stack.is_empty() {
            let home_res = std::env::var("HOME");
            match home_res {
                Ok(home) => {
                    let res = env::set_current_dir(&home);
                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            let err_str = format!("unable to cd to home: {}", e);
                            self.print_error(&err_str);
                            return 0;
                        }
                    }
                }
                Err(e) => {
                    let err_str = format!("unable to cd to home: {}", e);
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
                    let dirs = VM::expand_tilde(dir);
                    let path_dir = Path::new(&dirs);
                    let res = env::set_current_dir(&path_dir);
                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            let err_str = format!("unable to cd: {}", e);
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
        1
    }

    /// Puts the string representation of the current working
    /// directory onto the stack.
    pub fn core_pwd(&mut self) -> i32 {
        let current_dir_res = std::env::current_dir();
        match current_dir_res {
            Ok(current_dir) => {
                self.stack
                    .push(Value::String(Rc::new(RefCell::new(StringTriple::new(
                        current_dir.to_str().unwrap().to_string(),
                        None,
                    )))));
            }
            Err(e) => {
                let err_str = format!("unable to pwd: {}", e);
                self.print_error(&err_str);
                return 0;
            }
        }
        1
    }

    /// Takes a value that can be stringified as its single argument.
    /// Creates the file if it doesn't exist, and updates its
    /// modification timestamp to the current time if it does exist,
    /// similarly to touch(1).
    pub fn core_touch(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("touch requires one argument");
            return 0;
        }

        let path_rr = self.stack.pop().unwrap();
        let path_opt: Option<&str>;
        to_str!(path_rr, path_opt);

        match path_opt {
            Some(path_str) => {
                let path_strs = VM::expand_tilde(path_str);
                let path = Path::new(&path_strs);
                if !path.exists() {
                    let res = fs::write(&path_strs, "");
                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            let err_str = format!("unable to write file: {}", e);
                            self.print_error(&err_str);
                            return 0;
                        }
                    }
                } else {
                    let times_res = get_file_times(&path_strs);
                    match times_res {
                        Ok((accessed, _)) => {
                            let mtime = SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap()
                                .as_secs();
                            let sft_res = set_file_times(&path_strs, accessed, mtime as i64);
                            match sft_res {
                                Ok(_) => {}
                                Err(e) => {
                                    let err_str = format!("unable to write file: {}", e);
                                    self.print_error(&err_str);
                                    return 0;
                                }
                            }
                        }
                        Err(e) => {
                            let err_str = format!("unable to write file: {}", e);
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
        1
    }

    /// Takes a value that can be stringified and a boolean indicating
    /// whether to use the link itself or its target (if the value is
    /// a link) as its arguments.  Puts a hash onto the stack
    /// containing the metadata of the associated file, where "dev" is
    /// the device number, "ino" is the inode, "mode" is the file
    /// mode, "nlink" is the number of hard links to the file, "uid"
    /// is the user ID of the owner, "gid" is the group ID of the
    /// owner, "rdev" is the device ID (for special files), "size" is
    /// the total size in bytes, "atime"/"ctime"/"mtime" are various
    /// file modification times, "blksize" is the block size, and
    /// "blocks" is the number of blocks allocated to the file.
    fn stat_inner(&mut self, use_symlink: bool) -> i32 {
        if self.stack.is_empty() {
            self.print_error("stat requires one argument");
            return 0;
        }

        let path_rr = self.stack.pop().unwrap();
        let path_opt: Option<&str>;
        to_str!(path_rr, path_opt);

        match path_opt {
            Some(s) => {
                let ss = VM::expand_tilde(s);
                let meta_res = if use_symlink {
                    fs::symlink_metadata(&ss)
                } else {
                    fs::metadata(&ss)
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
                            "atime".to_string(),
                            Value::BigInt(BigInt::from_i64(meta.atime()).unwrap()),
                        );
                        map.insert(
                            "mtime".to_string(),
                            Value::BigInt(BigInt::from_i64(meta.mtime()).unwrap()),
                        );
                        map.insert(
                            "ctime".to_string(),
                            Value::BigInt(BigInt::from_i64(meta.ctime()).unwrap()),
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
                        let err_str = format!("unable to stat file: {}", e);
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
        1
    }

    /// See stat_inner.
    pub fn core_stat(&mut self) -> i32 {
        self.stat_inner(false)
    }

    /// See stat_inner.
    pub fn core_lstat(&mut self) -> i32 {
        self.stat_inner(true)
    }

    /// Puts current process information onto the stack, in the form
    /// of a list of hashes.  Each hash has elements for "pid", "uid",
    /// and "name".
    #[allow(unused_variables)]
    pub fn core_ps(&mut self) -> i32 {
        let tz = self.local_tz;
        self.instantiate_sys();
        let sysopt = &mut self.sys;
        let sys = &mut sysopt.as_mut().unwrap();
        sys.refresh_processes();

        /* Using the same approach as in nushell for calculating CPU
         * usage. */
        sys.refresh_cpu_specifics(CpuRefreshKind::everything());
        std::thread::sleep(System::MINIMUM_CPU_UPDATE_INTERVAL * 2);
        sys.refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage());

        /* refresh_processes does not remove processes that have
         * since completed, which is why these extra steps are
         * necessary. */
        let mut pids = Vec::new();
        for pid in sys.processes().keys() {
            pids.push(*pid);
        }
        let mut actual_pids = HashSet::new();
        for pid in pids {
            if sys.refresh_process(pid) {
                actual_pids.insert(pid);
            }
        }

        let mut lst = VecDeque::new();
        for (pid, process) in sys.processes() {
            if !actual_pids.contains(pid) {
                continue;
            }
            let mut map = IndexMap::new();
            map.insert(
                "pid".to_string(),
                Value::BigInt(BigInt::from_i32(pid.as_u32().try_into().unwrap()).unwrap()),
            );
            map.insert(
                "uid".to_string(),
                Value::BigInt(BigInt::from_u32(**(process.user_id().unwrap())).unwrap()),
            );
            map.insert(
                "gid".to_string(),
                Value::BigInt(BigInt::from_u32(*(process.group_id().unwrap())).unwrap()),
            );
            match sys.get_user_by_id(process.user_id().unwrap()) {
                None => {}
                Some(user) => {
                    map.insert(
                        "user".to_string(),
                        Value::String(Rc::new(RefCell::new(StringTriple::new(
                            user.name().to_string(),
                            None,
                        ))))
                    );
                }
            };
            map.insert(
                "name".to_string(),
                Value::String(Rc::new(RefCell::new(StringTriple::new(
                    process.name().to_string(),
                    None,
                )))),
            );
            map.insert(
                "cmd".to_string(),
                Value::String(Rc::new(RefCell::new(StringTriple::new(
                    process.cmd().join(" "),
                    None,
                )))),
            );
            map.insert(
                "cpu".to_string(),
                Value::Float(process.cpu_usage().into()),
            );
            map.insert(
                "mem".to_string(),
                Value::BigInt(BigInt::from_u64(process.memory().into()).unwrap()),
            );
            map.insert(
                "vmem".to_string(),
                Value::BigInt(BigInt::from_u64(process.virtual_memory().into()).unwrap()),
            );
            map.insert(
                "runtime".to_string(),
                Value::BigInt(BigInt::from_u64(process.run_time()).unwrap()),
            );
            let s = format!("{}", process.status());
            map.insert(
                "status".to_string(),
                Value::String(Rc::new(RefCell::new(StringTriple::new(
                    s,
                    None,
                )))),
            );

	    let epoch64 = i64::try_from(process.start_time()).unwrap();
	    let naive = NaiveDateTime::from_timestamp_opt(epoch64, 0).unwrap();
	    let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
	    let newdate = datetime.with_timezone(&tz);
            map.insert(
                "start".to_string(),
                Value::DateTimeNT(newdate)
            );

            lst.push_back(Value::Hash(Rc::new(RefCell::new(map))))
        }
        self.stack.push(Value::List(Rc::new(RefCell::new(lst))));
        1
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
                        let err_str = format!("unable to kill process: {}", e);
                        self.print_error(&err_str);
                        return 0;
                    }
                }
                1
            }
            (Some(_), _) => {
                self.print_error("second kill argument must be signal");
                0
            }
            (_, _) => {
                self.print_error("first kill argument must be process");
                0
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
                let paths = VM::expand_tilde(path);
                let f_opt = fs::metadata(&paths);
                if f_opt.is_err() {
                    self.print_error("unable to get metadata for path");
                    return 0;
                }
                let f = f_opt.unwrap();
                let mut perms = f.permissions();
                perms.set_mode(mode.try_into().unwrap());
                let res = fs::set_permissions(&paths, perms);
                match res {
                    Ok(_) => 1,
                    Err(e) => {
                        let s = format!("unable to chmod: {}", e);
                        self.print_error(&s);
                        0
                    }
                }
            }
            (Some(_), _) => {
                self.print_error("second chmod argument must be mode");
                0
            }
            (_, _) => {
                self.print_error("first chmod argument must be path");
                0
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
                    self.print_error("second chown argument must be valid user");
                    return 0;
                }
                let user_opt = user_opt_res.unwrap();
                if user_opt.is_none() {
                    self.print_error("second chown argument must be valid user");
                    return 0;
                }
                let user_obj = user_opt.unwrap();

                let group_opt_res = Group::from_name(group);
                if group_opt_res.is_err() {
                    self.print_error("third chown argument must be valid group");
                    return 0;
                }
                let group_opt = group_opt_res.unwrap();
                if group_opt.is_none() {
                    self.print_error("third chown argument must be valid group");
                    return 0;
                }
                let group_obj = group_opt.unwrap();

                let paths = VM::expand_tilde(path);
                let path_obj = Path::new(&paths);
                let chown_res = nix::unistd::chown(path_obj, Some(user_obj.uid), Some(group_obj.gid));
                match chown_res {
                    Ok(_) => 1,
                    Err(e) => {
                        let s = format!("unable to chown path: {}", e);
                        self.print_error(&s);
                        0
                    }
                }
            }
            (Some(_), Some(_), _) => {
                self.print_error("third chown argument must be group");
                0
            }
            (Some(_), _, _) => {
                self.print_error("second chown argument must be user");
                0
            }
            (_, _, _) => {
                self.print_error("first chown argument must be path");
                0
            }
        }
    }

    /// Takes a path as its single argument, and attempts to make a
    /// directory at that path.
    pub fn core_mkdir(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("mkdir requires one argument");
            return 0;
        }

        let dir_rr = self.stack.pop().unwrap();
        let dir_opt: Option<&str>;
        to_str!(dir_rr, dir_opt);

        match dir_opt {
            Some(dir) => {
                let dirs = VM::expand_tilde(dir);
                let res = std::fs::create_dir(dirs);
                match res {
                    Ok(_) => 1,
                    Err(e) => {
                        let s = format!("unable to make directory: {}", e);
                        self.print_error(&s);
                        0
                    }
                }
            }
            None => {
                self.print_error("first mkdir argument must be string");
                0
            }
        }
    }

    /// Takes a path as its single argument, and attempts to remove
    /// the directory at that path.
    pub fn core_rmdir(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("rmdir requires one argument");
            return 0;
        }

        let dir_rr = self.stack.pop().unwrap();
        let dir_opt: Option<&str>;
        to_str!(dir_rr, dir_opt);

        match dir_opt {
            Some(dir) => {
                let dirs = VM::expand_tilde(dir);
                let res = std::fs::remove_dir(dirs);
                match res {
                    Ok(_) => 1,
                    Err(e) => {
                        let s = format!("unable to remove directory: {}", e);
                        self.print_error(&s);
                        0
                    }
                }
            }
            None => {
                self.print_error("first rmdir argument must be string");
                0
            }
        }
    }
}

use std::error::Error;
use std::os::fd::{RawFd, AsRawFd, FromRawFd};
use std::process::exit;
use std::thread;
use std::time;
use std::time::Duration;

use epoll;
use nix::sys::signal::Signal;
use nix::sys::wait::waitpid;
use nix::unistd::{fork, ForkResult};
use signal_hook::{consts::SIGTERM, iterator::Signals};
use std::fs::File;
use std::io::Read;
use std::io::Write;

use crate::vm::*;
use crate::chunk::{ChannelGenerator, ValueSD,
                   value_to_valuesd, valuesd_to_value,
                   read_valuesd, write_valuesd};

pub struct Subprocess {
    pub pid: nix::unistd::Pid,
    pub value_tx: std::fs::File,
    pub reqvalue_rx: std::fs::File,
}

impl Subprocess {
    pub fn new(pid: nix::unistd::Pid,
               value_tx: std::fs::File,
               reqvalue_rx: std::fs::File) -> Subprocess {
        Subprocess { pid, value_tx, reqvalue_rx }
    }
}

impl VM {
    /// Parallel map.
    pub fn core_pmap(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("pmap requires two arguments");
            return 0;
        }

        let fn_rr = self.stack.pop().unwrap();
        let gen_rr = self.stack.pop().unwrap();

        // For transmitting results back up (processor to top).
        let mut ptt_tx;
        let mut ptt_rx;
        match nix::unistd::pipe() {
            Ok((fd1, fd2)) => {
                ptt_rx = unsafe { File::from_raw_fd(fd1) };
                ptt_tx = unsafe { File::from_raw_fd(fd2) };
            }
            Err(e) => {
                eprintln!("unable to create pipe: {}", e);
                return 0;
            }
        }

        let mut ctp_tx;
        let mut ctp_rx;
        match nix::unistd::pipe() {
            Ok((fd1, fd2)) => {
                ctp_rx = unsafe { File::from_raw_fd(fd1) };
                ctp_tx = unsafe { File::from_raw_fd(fd2) };
            }
            Err(e) => {
                eprintln!("unable to create pipe: {}", e);
                return 0;
            }
        }

        let mut ptc_tx;
        let mut ptc_rx;
        match nix::unistd::pipe() {
            Ok((fd1, fd2)) => {
                ptc_rx = unsafe { File::from_raw_fd(fd1) };
                ptc_tx = unsafe { File::from_raw_fd(fd2) };
            }
            Err(e) => {
                eprintln!("unable to create pipe: {}", e);
                return 0;
            }
        }

        let pcount = 2;

        match fork() {
            Ok(ForkResult::Parent { child }) => {
                let cg_obj = ChannelGenerator::new(ptt_rx, child);
                let cg =
                    Value::ChannelGenerator(Rc::new(RefCell::new(cg_obj)));
                self.stack.push(cg);
                return 1;
            }
            Ok(ForkResult::Child) => {
                let mut subprocesses = Vec::new();

                for i in 0..pcount {
                    let mut value_tx;
                    let mut value_rx;
                    match nix::unistd::pipe() {
                        Ok((fd1, fd2)) => {
                            value_rx = unsafe { File::from_raw_fd(fd1) };
                            value_tx = unsafe { File::from_raw_fd(fd2) };
                        }
                        Err(e) => {
                            eprintln!("unable to create pipe: {}", e);
                            return 0;
                        }
                    }

                    let mut reqvalue_tx;
                    let mut reqvalue_rx;
                    match nix::unistd::pipe() {
                        Ok((fd1, fd2)) => {
                            reqvalue_rx = unsafe { File::from_raw_fd(fd1) };
                            reqvalue_tx = unsafe { File::from_raw_fd(fd2) };
                        }
                        Err(e) => {
                            eprintln!("unable to create pipe: {}", e);
                            return 0;
                        }
                    }

                    match fork() {
                        Ok(ForkResult::Parent { child }) => {
                            subprocesses.push(
                                Subprocess::new(
                                    child,
                                    value_tx,
                                    reqvalue_rx
                                )
                            );
                        }
                        Ok(ForkResult::Child) => {
                            let sp_fn_rr = fn_rr.clone();
                            loop {
                                match reqvalue_tx.write(b"1") {
                                    Ok(_) => {}
                                    Err(e) => {
                                        eprintln!("unable to send request byte");
                                        exit(0);
                                    }
                                }

                                let vsd = read_valuesd(&mut value_rx);
                                let v;
                                match vsd {
                                    ValueSD::Null => {
                                        exit(0);
                                    }
                                    _ => {
                                        v = valuesd_to_value(vsd);
                                    }
                                }
                                self.stack.push(v);
                                let res = self.call(OpCode::Call, sp_fn_rr.clone());
                                if !res {
                                    exit(0);
                                }
                                let nv = self.stack.pop().unwrap();
                                match nv {
                                    Value::Null => {
                                        exit(0);
                                    }
                                    _ => {}
                                }
                                let vsd = value_to_valuesd(nv);
                                write_valuesd(&mut ptt_tx, vsd);
                            }
                        }
                        Err(e) => {
                            eprintln!("unable to fork: {}", e);
                            exit(0);
                        }
                    }
                }

                let mut epoll_fd = 0;
                let epoll_fd_res = epoll::create(true);
                match epoll_fd_res {
                    Ok(epoll_fd_ok) => {
                        epoll_fd = epoll_fd_ok;
                    }
                    Err(e) => {
                        eprintln!("epoll create failed: {:?}", e);
                        exit(0);
                    }
                }

                for i in 0..pcount {
                    let fd = subprocesses.get(i).unwrap()
                                .reqvalue_rx.as_raw_fd();
                    let res =
                        epoll::ctl(
                            epoll_fd,
                            epoll::ControlOptions::EPOLL_CTL_ADD,
                            fd,
                            epoll::Event::new(epoll::Events::EPOLLIN,
                                              fd as u64));
                    match res {
                        Err(e) => {
                            eprintln!("epoll ctl failed: {:?}", e);
                            exit(0);
                        }
                        _ => {}
                    }
                }

                let mut signals = Signals::new(&[SIGTERM]).unwrap();
                let pids = subprocesses.iter().map(|e| e.pid).collect::<Vec<_>>();

                thread::spawn(move || {
                    for sig in signals.forever() {
                        for i in pids.clone() {
                            let res = nix::sys::signal::kill(i, Signal::SIGTERM);
                            match res {
                                Ok(_) => {}
                                Err(nix::Error::Sys(nix::errno::Errno::ESRCH)) => {}
                                Err(e) => {
                                    eprintln!("unable to kill process: {}", e);
                                }
                            }
                        }
                        for i in pids {
                            waitpid(i, None);
                        }
                        exit(0);
                    }
                });

                self.stack.push(gen_rr);
                let mut events = [epoll::Event::new(epoll::Events::empty(), 0); 10];
                'done: loop {
                    let res = epoll::wait(epoll_fd, -1, &mut events);
                    let mut n = 0;
                    match res {
                        Err(e) => {
                            /* Assuming that "Interrupted" is due
                             * to ctrl-c, in which case there's no
                             * need to show an error message. */
                            if !e.to_string().contains("Interrupted") {
                                eprintln!("epoll wait failed: {:?}", e);
                            }
                            break 'done;
                        }
                        Ok(n_ok) => { n = n_ok; }
                    }
                    for i in 0..n {
                        let event = events.get(i).unwrap();
                        for i in 0..pcount {
                            if subprocesses.get(i).unwrap().reqvalue_rx.as_raw_fd() == event.data as i32 {
                                let subprocess = &mut subprocesses.get_mut(i).unwrap();
                                let pid = subprocess.pid;

                                let mut size_buf = vec![0u8; 1];
                                subprocess.reqvalue_rx.read_exact(&mut size_buf);

                                let dup_res = self.opcode_dup();
                                if dup_res == 0 {
                                    break 'done;
                                }
                                let shift_res = self.opcode_shift();
                                if shift_res == 0 {
                                    break 'done;
                                }
                                let element_rr = self.stack.pop().unwrap();
                                match element_rr {
                                    Value::Null => {
                                        break 'done;
                                    }
                                    _ => {
                                        let vsd = value_to_valuesd(element_rr);
                                        write_valuesd(&mut subprocess.value_tx, vsd);
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
                self.stack.pop();
                for i in 0..pcount {
                    write_valuesd(&mut subprocesses.get_mut(i).unwrap().value_tx, ValueSD::Null);
                }
                for i in 0..pcount {
                    waitpid(subprocesses.get(i).unwrap().pid, None);
                }
                write_valuesd(&mut ptt_tx, ValueSD::Null);
                exit(0);
            }
            Err(e) => {
                eprintln!("unable to fork: {}", e);
                exit(0);
            }
        }
    }
}

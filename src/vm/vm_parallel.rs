use std::os::fd::AsRawFd;
use std::process::exit;
use std::thread;
use std::time;

use epoll;
use nix::sys::signal::Signal;
use nix::sys::wait::waitpid;
use nix::unistd::{fork, ForkResult};
use signal_hook::{consts::SIGTERM, iterator::Signals};
use std::fs::File;
use std::io::Read;
use std::io::Write;
use nix::fcntl::fcntl;
use nix::fcntl::FcntlArg::F_SETFL;
use nix::fcntl::OFlag;

use crate::chunk::{ChannelGenerator,
                   ValueSD,
                   value_to_valuesd, valuesd_to_value,
                   read_valuesd, write_valuesd};
use crate::vm::*;

/// The details for a subprocess created by way of pmap.
pub struct Subprocess {
    /// The subprocess's process identifier.
    pub pid: nix::unistd::Pid,
    /// The filehandle for transmitting values to the subprocess.
    pub value_tx: std::fs::File,
    /// The filehandle for listening for requests from the subprocess
    /// for more values.
    pub reqvalue_rx: std::fs::File,
}

impl Subprocess {
    pub fn new(pid: nix::unistd::Pid,
               value_tx: std::fs::File,
               reqvalue_rx: std::fs::File) -> Subprocess {
        Subprocess { pid, value_tx, reqvalue_rx }
    }
}

fn make_pipe() -> Option<(std::fs::File, std::fs::File)> {
    let tx;
    let rx;
    match nix::unistd::pipe() {
        Ok((fd1, fd2)) => {
            fcntl(fd1.as_raw_fd(), F_SETFL(OFlag::O_NONBLOCK)).unwrap();
            rx = File::from(fd1);
            tx = File::from(fd2);
        }
        Err(e) => {
            eprintln!("unable to create pipe: {}", e);
            return None;
        }
    }

    return Some((tx, rx));
}

impl VM {
    /// Parallel map.
    pub fn core_pmap(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("pmap requires two arguments");
            return 0;
        }

        return self.pmap_inner(4);
    }

    /// Parallel map with a specified number of processes.
    pub fn core_pmapn(&mut self) -> i32 {
        if self.stack.len() < 3 {
            self.print_error("pmapn requires three arguments");
            return 0;
        }

        let procs_rr = self.stack.pop().unwrap();
        let procs_int = procs_rr.to_int();
        match procs_int {
            Some(n) => {
                if n < 1 {
                    self.print_error("third pmapn argument must be positive integer");
                    return 0;
                }
                return self.pmap_inner(n as usize);
            }
            _ => {
                self.print_error("third pmapn argument must be integer");
                return 0;
            }
        }
    }

    /// Core parallel map operation.
    pub fn pmap_inner(&mut self, procs: usize) -> i32 {
        let fn_rr = self.stack.pop().unwrap();
        let gen_rr = self.stack.pop().unwrap();

        /* For transmitting results back up (subprocesses to original
         * process). */
        let (mut ptt_tx, ptt_rx) = make_pipe().unwrap();

        unsafe {
            match fork() {
                Ok(ForkResult::Parent { child }) => {
                    let cg_obj = ChannelGenerator::new(ptt_rx, child, gen_rr);
                    let cg =
                        Value::ChannelGenerator(Rc::new(RefCell::new(cg_obj)));
                    self.stack.push(cg);
                    return 1;
                }
                Ok(ForkResult::Child) => {
                    let mut subprocesses = Vec::new();

                    for _ in 0..procs {
                        let (value_tx, mut value_rx) =
                            make_pipe().unwrap();
                        let (mut reqvalue_tx, reqvalue_rx) =
                            make_pipe().unwrap();

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
                                    /* The value used here doesn't matter,
                                    * as long as it's one byte in length.
                                    * */
                                    match reqvalue_tx.write(b"1") {
                                        Ok(_) => {}
                                        Err(_) => {
                                            eprintln!("unable to send request byte");
                                            exit(0);
                                        }
                                    }

                                    let mut vsd_res;
                                    let v;
                                    loop {
                                        vsd_res = read_valuesd(&mut value_rx);
                                        match vsd_res {
                                            None => {
						let dur = time::Duration::from_secs_f64(0.05);
						thread::sleep(dur); 
                                            }
                                            Some(ValueSD::Null) => {
                                                exit(0);
                                            }
                                            _ => {
                                                v = valuesd_to_value(vsd_res.unwrap());
                                                break;
                                            }
                                        }
                                    }
                                    self.stack.push(v);
                                    let res = self.call(OpCode::Call, sp_fn_rr.clone());
                                    if !res || self.stack.is_empty() {
                                        let vsd = value_to_valuesd(Value::Null);
                                        write_valuesd(&mut ptt_tx, vsd);
                                        exit(0);
                                    } else {
                                        let nv = self.stack.pop().unwrap();
                                        match nv {
                                            Value::Null => {
                                                let vsd = value_to_valuesd(Value::Null);
                                                write_valuesd(&mut ptt_tx, vsd);
                                                exit(0);
                                            }
                                            _ => {}
                                        }
                                        let vsd = value_to_valuesd(nv.clone());
                                        match (&vsd, nv) {
                                            (&ValueSD::Null, Value::Null) => {}
                                            (&ValueSD::Null, _) => {
                                                self.print_error("unable to serialise value for pmap");
                                            }
                                            _ => {}
                                        }
                                        write_valuesd(&mut ptt_tx, vsd);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("unable to fork: {}", e);
                                exit(0);
                            }
                        }
                    }

                    let epoll_fd;
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

                    for i in 0..procs {
                        let fd = subprocesses.get(i).unwrap()
                                    .reqvalue_rx.as_raw_fd();
                        let res =
                            epoll::ctl(
                                epoll_fd,
                                epoll::ControlOptions::EPOLL_CTL_ADD,
                                fd,
                                epoll::Event::new(epoll::Events::EPOLLIN,
                                                fd as u64)
                            );
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
                        for _ in signals.forever() {
                            for i in pids.clone() {
                                let res = nix::sys::signal::kill(i, Signal::SIGTERM);
                                match res {
                                    Ok(_) => {}
                                    Err(nix::errno::Errno::ESRCH) => {}
                                    Err(e) => {
                                        eprintln!("unable to kill process: {}", e);
                                    }
                                }
                            }
                            for i in pids {
                                let res = waitpid(i, None);
                                match res {
                                    /* Termination by way of the normal
                                    * process further down may have
                                    * happened by this time, so ignore
                                    * this error. */
                                    Err(nix::errno::Errno::ECHILD) => {},
                                    Err(e) => {
                                        eprintln!("unable to clean up process: {}", e);
                                    }
                                    _ => {}
                                }
                            }
                            exit(0);
                        }
                    });

                    self.stack.push(gen_rr);
                    let mut events =
                        [epoll::Event::new(epoll::Events::empty(), 0); 50];
                    'done: loop {
                        let res = epoll::wait(epoll_fd, -1, &mut events);
                        let n;
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
                            for i in 0..procs {
                                if subprocesses.get(i).unwrap().reqvalue_rx.as_raw_fd() == event.data as i32 {
                                    let subprocess = &mut subprocesses.get_mut(i).unwrap();

                                    let mut size_buf = vec![0u8; 1];
                                    let read_res =
                                        subprocess.reqvalue_rx.read_exact(&mut size_buf);
                                    if read_res.is_err() {
                                        break 'done;
                                    }
                                    read_res.unwrap();

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
                                            let vsd = value_to_valuesd(element_rr.clone());
                                            match (&vsd, element_rr) {
                                                (&ValueSD::Null, Value::Null) => {}
                                                (&ValueSD::Null, _) => {
                                                    self.print_error("unable to serialise value for pmap");
                                                }
                                                _ => {}
                                            }
                                            write_valuesd(&mut subprocess.value_tx, vsd);
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    self.stack.pop();
                    for i in 0..procs {
                        write_valuesd(&mut subprocesses.get_mut(i).unwrap().value_tx, ValueSD::Null);
                    }
                    for i in 0..procs {
                        let res =
                            waitpid(subprocesses.get(i).unwrap().pid, None);
                        match res {
                            /* Termination by way of a signal may have
                            * happened by this point, so ignore this
                            * error. */
                            Err(nix::errno::Errno::ECHILD) => {},
                            Err(e) => {
                                eprintln!("unable to clean up process: {}", e);
                            }
                            _ => {}
                        }
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
}

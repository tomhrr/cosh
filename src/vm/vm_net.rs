use std::cell::RefCell;
use std::io::BufWriter;
use std::net::TcpStream;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time;

use ipnetwork::IpNetwork::{V4, V6};
use netstat2::*;
use num::FromPrimitive;
use num_bigint::BigInt;
use pnet::datalink;
use sysinfo::Uid;

use crate::chunk::{Value, new_string_value,
                   BufReaderWithBuffer};
use crate::hasher::{new_hash_indexmap, new_set_indexmap};
use crate::vm::*;

impl VM {
    pub fn core_ifconfig(&mut self) -> i32 {
        let interfaces = datalink::interfaces();
        let mut lst = VecDeque::new();
        for interface in interfaces {
            let mut map = new_hash_indexmap();
            map.insert(
                "name".to_string(),
                new_string_value(interface.name)
            );
            match interface.mac {
                Some(m) => {
                    map.insert(
                        "mac".to_string(),
                        new_string_value(m.to_string())
                    );
                }
                _ => {}
            }
            let mut iplst = VecDeque::new();
            for ip in interface.ips {
                match ip {
                    V4(network) => {
                        let ipaddr = new_string_value(network.ip().to_string());
                        self.stack.push(ipaddr);
                        let res = self.core_ip();
                        if res != 1 {
                            return 0;
                        }
                        let ipaddr_obj = self.stack.pop().unwrap();

                        let netaddr =
                            format!("{}/{}",
                                    network.network().to_string(),
                                    network.prefix().to_string());
                        self.stack.push(new_string_value(netaddr));
                        let res = self.core_ip();
                        if res != 1 {
                            return 0;
                        }
                        let netaddr_obj = self.stack.pop().unwrap();

                        let mut netmap = new_hash_indexmap();
                        netmap.insert("ip".to_string(),      ipaddr_obj);
                        netmap.insert("network".to_string(), netaddr_obj);

                        iplst.push_back(Value::Hash(Rc::new(RefCell::new(netmap))));
                    }
                    V6(network) => {
                        let ipaddr = new_string_value(network.ip().to_string());
                        self.stack.push(ipaddr);
                        let res = self.core_ip();
                        if res != 1 {
                            return 0;
                        }
                        let ipaddr_obj = self.stack.pop().unwrap();

                        let netaddr =
                            format!("{}/{}",
                                    network.network().to_string(),
                                    network.prefix().to_string());
                        self.stack.push(new_string_value(netaddr));
                        let res = self.core_ip();
                        if res != 1 {
                            return 0;
                        }
                        let netaddr_obj = self.stack.pop().unwrap();

                        let mut netmap = new_hash_indexmap();
                        netmap.insert("ip".to_string(),      ipaddr_obj);
                        netmap.insert("network".to_string(), netaddr_obj);

                        iplst.push_back(Value::Hash(Rc::new(RefCell::new(netmap))));
                    }
                }
            }
            map.insert("ips".to_string(),
                       Value::List(Rc::new(RefCell::new(iplst))));

            map.insert("flags".to_string(),
                       Value::BigInt(
                           BigInt::from_u32(interface.flags.try_into().unwrap()).unwrap()
                       ));

            lst.push_back(Value::Hash(Rc::new(RefCell::new(map))));
        }
        self.stack.push(Value::List(Rc::new(RefCell::new(lst))));
        1
    }

    pub fn core_netstat(&mut self) -> i32 {
        let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
        let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;
        let sockets_info =
            get_sockets_info(af_flags, proto_flags).unwrap();

        {
            self.instantiate_sys();
            let usersopt = &mut self.users;
            let users = &mut usersopt.as_mut().unwrap();
            users.refresh_list();
        }

        let mut lst = VecDeque::new();
        for si in sockets_info {
            let mut map = new_hash_indexmap();
            match si.protocol_socket_info {
                ProtocolSocketInfo::Tcp(tcp_si) => {
                    map.insert("type".to_string(),
                               new_string_value("tcp".to_string()));

                    let local_addr = tcp_si.local_addr.to_string();
                    self.stack.push(new_string_value(local_addr));
                    let res = self.core_ip();
                    if res != 1 {
                        return 0;
                    }
                    map.insert("local_addr".to_string(),
                               self.stack.pop().unwrap());
                    map.insert("local_port".to_string(),
                               Value::Int(tcp_si.local_port.into()));

                    let remote_addr = tcp_si.remote_addr.to_string();
                    self.stack.push(new_string_value(remote_addr));
                    let res = self.core_ip();
                    if res != 1 {
                        return 0;
                    }
                    map.insert("remote_addr".to_string(),
                               self.stack.pop().unwrap());
                    map.insert("remote_port".to_string(),
                               Value::Int(tcp_si.remote_port.into()));

                    map.insert("state".to_string(),
                               new_string_value(tcp_si.state.to_string()));
                }
                ProtocolSocketInfo::Udp(udp_si) => {
                    map.insert("type".to_string(),
                               new_string_value("udp".to_string()));

                    let local_addr = udp_si.local_addr.to_string();
                    self.stack.push(new_string_value(local_addr));
                    let res = self.core_ip();
                    if res != 1 {
                        return 0;
                    }
                    map.insert("local_addr".to_string(),
                               self.stack.pop().unwrap());
                    map.insert("local_port".to_string(),
                               Value::Int(udp_si.local_port.into()));
                }
            }

            let usersopt = &mut self.users;
            let users = &mut usersopt.as_mut().unwrap();

            map.insert("inode".to_string(),
                       Value::BigInt(
                           BigInt::from_u32(si.inode.try_into().unwrap()).unwrap()
                       ));
            map.insert("uid".to_string(),
                       Value::BigInt(
                           BigInt::from_u32(si.uid.try_into().unwrap()).unwrap()
                       ));

            match users.get_user_by_id(&Uid::try_from(usize::try_from(si.uid).unwrap()).unwrap()) {
                None => {}
                Some(user) => {
                    map.insert(
                        "user".to_string(),
                        new_string_value(user.name().to_string())
                    );
                }
            };

            let mut pids = VecDeque::new();
            for pid in si.associated_pids {
                pids.push_back(Value::BigInt(BigInt::from_u32(pid).unwrap()));
            }
            map.insert("pids".to_string(),
                       Value::List(Rc::new(RefCell::new(pids))));

            lst.push_back(Value::Hash(Rc::new(RefCell::new(map))));
        }
        self.stack.push(Value::List(Rc::new(RefCell::new(lst))));
        1
    }

    pub fn core_socket(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("socket requires two arguments");
            return 0;
        }

        let port_rr  = self.stack.pop().unwrap();
        let host_rr  = self.stack.pop().unwrap();

        let port_opt = port_rr.to_int();
        let port_int;
        match port_opt {
            Some(port) => {
                port_int = port;
            }
            _ => {
                self.print_error("port number must be an integer");
                return 0;
            }
        };

	let host_str_opt: Option<&str>;
	to_str!(host_rr.clone(), host_str_opt);
        let host_str;
	match host_str_opt {
            Some(host) => {
                host_str = host;
            }
            _ => {
                self.print_error("host must be a string");
                return 0;
            }
        }

        let conn_str = format!("{}:{}", host_str, port_int);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let stream_res = TcpStream::connect(conn_str);
            let _ = tx.send(stream_res);
        });
        let stream;
        loop {
            let stream_recv_res = rx.try_recv();
            match stream_recv_res {
                Ok(Ok(stream_value)) => {
                    stream = stream_value;
                    break;
                }
                Ok(Err(e)) => {
                    let err_str = format!("unable to connect to host: {}", e);
                    self.print_error(&err_str);
                    return 0;
                }
                Err(TryRecvError::Disconnected) => {
                    let err_str = format!("unable to connect to host: disconnected");
                    self.print_error(&err_str);
                    return 0;
                }
                Err(TryRecvError::Empty) => {
                    if !self.running.load(Ordering::SeqCst) {
                        self.running.store(true, Ordering::SeqCst);
                        self.stack.clear();
                        return 0;
                    }
                    let dur = time::Duration::from_secs_f64(0.05);
                    thread::sleep(dur);
                }
            }
        };
        stream.set_nonblocking(true).unwrap();

        let tsw = Value::TcpSocketWriter(Rc::new(RefCell::new(BufWriter::new(stream.try_clone().unwrap()))));
        let tsr = Value::TcpSocketReader(Rc::new(RefCell::new(BufReaderWithBuffer::new(BufReader::new(stream)))));
        self.stack.push(tsr);
        self.stack.push(tsw);
        return 1;
    }

    pub fn core_nc(&mut self) -> i32 {
        if self.stack.len() < 3 {
            self.print_error("nc requires three arguments");
            return 0;
        }

        let res = self.core_socket();
        if res == 0 {
            return 0;
        }

        let tsw_rr   = self.stack.pop().unwrap();
        let tsr_rr   = self.stack.pop().unwrap();
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
                    _ => {
                        self.stack.push(tsw_rr.clone());
                        self.stack.push(element_rr);
                        let res = self.core_writeline();
                        if res == 0 {
                            return 0;
                        }
                    }
                }
            };
            self.stack.pop().unwrap();
        } else {
            self.stack.push(tsw_rr);
            self.stack.push(input_rr);
            let res = self.core_writeline();
            if res == 0 {
                return 0;
            }
        }

        self.stack.push(tsr_rr);
        return 1;
    }
}

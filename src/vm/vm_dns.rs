use std::cell::RefCell;
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time;

use hickory_client::client::Client;
use hickory_client::client::SyncClient;
use hickory_client::op::ResponseCode;
use hickory_client::rr::Record;
use hickory_client::rr::RecordType;
use hickory_client::udp::UdpClientConnection;

use crate::chunk::{Value, new_string_value};
use crate::vm::*;

impl VM {
    /// Add the given string to the map as a bigint, unless it can't
    /// be parsed, in which case add it as a string.
    pub fn add_bi(&mut self,
                  map: &mut IndexMap<String, Value>,
                  key: &str,
                  value: &str) {
        let bi_opt = value.parse::<num_bigint::BigInt>();
        match bi_opt {
            Ok(bi) => {
                map.insert(key.to_string(),
                           Value::BigInt(bi));
            }
            _ => {
                map.insert(key.to_string(),
                           new_string_value(value.to_string()));
            }
        }
    }

    /// Add the given string to the map as an int, unless it can't
    /// be parsed, in which case add it as a string.
    pub fn add_int(&mut self,
                   map: &mut IndexMap<String, Value>,
                   key: &str,
                   value: &str) {
        let int_opt = value.parse::<i32>();
        match int_opt {
            Ok(int) => {
                map.insert(key.to_string(),
                           Value::Int(int));
            }
            _ => {
                map.insert(key.to_string(),
                           new_string_value(value.to_string()));
            }
        }
    }

    /// Convert the DNS record into a hash.
    pub fn record_to_value(&mut self,
                           record: &Record) -> Value {
        let mut record_map = IndexMap::new();
        record_map.insert(
            "name".to_string(),
            new_string_value(record.name().to_string())
        );
        record_map.insert(
            "ttl".to_string(),
            Value::BigInt(record.ttl().try_into().unwrap())
        );
        record_map.insert(
            "class".to_string(),
            new_string_value("IN".to_string())
        );
        record_map.insert(
            "type".to_string(),
            new_string_value(record.record_type().to_string())
        );
        match record.data() {
            Some(d) => {
                record_map.insert(
                    "rdata".to_string(),
                    new_string_value(d.to_string())
                );
            }
            _ => {}
        }

        let mut sdata_map = IndexMap::new();
        match record.record_type() {
            RecordType::A => {
                sdata_map.insert(
                    "address".to_string(),
                    new_string_value(record.data().unwrap().to_string())
                );
            }
            RecordType::AAAA => {
                sdata_map.insert(
                    "address".to_string(),
                    new_string_value(record.data().unwrap().to_string())
                );
            }
            RecordType::NS => {
                sdata_map.insert(
                    "nsdname".to_string(),
                    new_string_value(record.data().unwrap().to_string())
                );
            }
            RecordType::CNAME => {
                sdata_map.insert(
                    "cname".to_string(),
                    new_string_value(record.data().unwrap().to_string())
                );
            }
            RecordType::PTR => {
                sdata_map.insert(
                    "ptrdname".to_string(),
                    new_string_value(record.data().unwrap().to_string())
                );
            }
            RecordType::TXT => {
                sdata_map.insert(
                    "txtdata".to_string(),
                    new_string_value(record.data().unwrap().to_string())
                );
            }
            RecordType::MX => {
                let rdstr = record.data().unwrap().to_string();
                let parts: Vec<&str> = rdstr.split_whitespace().collect();
                sdata_map.insert(
                    "preference".to_string(),
                    new_string_value(parts.get(0).unwrap().to_string())
                );
                sdata_map.insert(
                    "exchange".to_string(),
                    new_string_value(parts.get(1).unwrap().to_string())
                );
            }
            RecordType::SOA => {
                let rdstr = record.data().unwrap().to_string();
                let parts: Vec<&str> = rdstr.split_whitespace().collect();
                sdata_map.insert(
                    "mname".to_string(),
                    new_string_value(parts.get(0).unwrap().to_string())
                );
                sdata_map.insert(
                    "rname".to_string(),
                    new_string_value(parts.get(1).unwrap().to_string())
                );
                self.add_bi(&mut sdata_map, "serial",  parts.get(2).unwrap());
                self.add_bi(&mut sdata_map, "refresh", parts.get(3).unwrap());
                self.add_bi(&mut sdata_map, "retry",   parts.get(4).unwrap());
                self.add_bi(&mut sdata_map, "expire",  parts.get(5).unwrap());
                self.add_bi(&mut sdata_map, "minimum", parts.get(6).unwrap());
            }
            RecordType::DS => {
                let rdstr = record.data().unwrap().to_string();
                let parts: Vec<&str> = rdstr.split_whitespace().collect();
                self.add_int(&mut sdata_map, "keytag",    parts.get(0).unwrap());
                self.add_int(&mut sdata_map, "algorithm", parts.get(1).unwrap());
                self.add_int(&mut sdata_map, "digtype",   parts.get(2).unwrap());
                sdata_map.insert(
                    "digest".to_string(),
                    new_string_value(parts.get(3).unwrap().to_string())
                );
            }
            RecordType::DNSKEY => {
                let rdstr = record.data().unwrap().to_string();
                let parts: Vec<&str> = rdstr.split_whitespace().collect();
                self.add_int(&mut sdata_map, "flags",     parts.get(0).unwrap());
                self.add_int(&mut sdata_map, "protocol",  parts.get(1).unwrap());
                self.add_int(&mut sdata_map, "algorithm", parts.get(2).unwrap());
                sdata_map.insert(
                    "keybin".to_string(),
                    new_string_value(parts.get(3).unwrap().to_string())
                );
            }
            _ => {}
        }
        if sdata_map.len() > 0 {
            record_map.insert(
                "sdata".to_string(),
                Value::Hash(Rc::new(RefCell::new(sdata_map)))
            );
        }

        return Value::Hash(Rc::new(RefCell::new(record_map)));
    }

    /// Query for DNS records at a specific server.
    pub fn core_dnsat(&mut self) -> i32 {
        if self.stack.len() < 3 {
            self.print_error("dnsat requires three arguments");
            return 0;
        }

        let type_rr = self.stack.pop().unwrap();
        let query_rr = self.stack.pop().unwrap();
        let server_rr = self.stack.pop().unwrap();

        let type_opt: Option<&str>;
        to_str!(type_rr, type_opt);
        let query_opt: Option<&str>;
        to_str!(query_rr, query_opt);
        let server_opt: Option<&str>;
        to_str!(server_rr, server_opt);

        match (type_opt, query_opt, server_opt) {
            (Some(type_str), Some(query), Some(server)) => {
                let addr_opt = std::net::IpAddr::from_str(server);
                let addr_to_use =
                    match addr_opt {
                        Ok(std::net::IpAddr::V4(addr)) => {
                            SocketAddr::new(std::net::IpAddr::V4(addr), 53)
                        }
                        Ok(std::net::IpAddr::V6(addr)) => {
                            SocketAddr::new(std::net::IpAddr::V6(addr), 53)
                        }
                        Err(_) => {
                            self.print_error("unable to parse IP address");
                            return 0;
                        }
                    };

                let query_so = query.to_string();
                let type_str_so = type_str.to_string();

		let uc_type_str = type_str_so.to_uppercase();
                let record_type_opt =
                    hickory_client::rr::RecordType::from_str(&uc_type_str);
                if let Err(_) = record_type_opt {
                    self.print_error("invalid DNS record type");
                    return 0;
                }
                let record_type = record_type_opt.unwrap();

		let (tx, rx) = mpsc::channel();
		thread::spawn(move || {
		    let client = SyncClient::new(
			UdpClientConnection::new(addr_to_use).unwrap()
		    );
		    let name =
			hickory_client::rr::Name::from_ascii(query_so).unwrap();
		    let res = client.query(
			&name,
			hickory_client::rr::DNSClass::IN,
			record_type
		    );
		    let _ = tx.send(res);
		});
                let resp;
		loop {
		    let response_recv_res = rx.try_recv();
		    match response_recv_res {
			Ok(Ok(response)) => {
                            resp = response;
                            break;
			}
			Ok(Err(e)) => {
			    let err_str = format!("unable to send request: {}", e);
			    self.print_error(&err_str);
			    return 0;
			}
			Err(TryRecvError::Disconnected) => {
			    let err_str = format!("unable to send request: disconnected");
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
                }

                let mut header_map = IndexMap::new();
                header_map.insert(
                    "opcode".to_string(),
                    new_string_value(resp.op_code().to_string().to_uppercase())
                );
                let status_str =
                    match resp.response_code() {
                        ResponseCode::NoError  => "NOERROR",
                        ResponseCode::FormErr  => "FORMERR",
                        ResponseCode::ServFail => "SERVFAIL",
                        ResponseCode::NXDomain => "NXDOMAIN",
                        ResponseCode::NotImp   => "NOTIMP",
                        ResponseCode::Refused  => "REFUSED",
                        ResponseCode::YXDomain => "YXDOMAIN",
                        ResponseCode::YXRRSet  => "YXRRSET",
                        ResponseCode::NXRRSet  => "NXRRSET",
                        ResponseCode::NotAuth  => "NOTAUTH",
                        ResponseCode::NotZone  => "NOTZONE",
                        _ => "UNKNOWN"
                    };
                header_map.insert(
                    "status".to_string(),
                    new_string_value(status_str.to_string())
                );
                header_map.insert(
                    "id".to_string(),
                    Value::Int(resp.id() as i32)
                );

                let mut question_map = IndexMap::new();
                question_map.insert(
                    "name".to_string(),
                    new_string_value(query.to_string())
                );
                question_map.insert(
                    "type".to_string(),
                    new_string_value("IN".to_string())
                );
                question_map.insert(
                    "class".to_string(),
                    new_string_value(type_str.to_string())
                );

                let mut answer_lst = VecDeque::new();
                for record in resp.answers() {
                    answer_lst.push_back(self.record_to_value(&record));
                }

                let mut authority_lst = VecDeque::new();
                for record in resp.name_servers() {
                    if record.record_type() == RecordType::SOA {
                        authority_lst.push_back(self.record_to_value(&record));
                    }
                }

                let mut additional_lst = VecDeque::new();
                for record in resp.additionals() {
                    additional_lst.push_back(self.record_to_value(&record));
                }

                let mut res_map = IndexMap::new();
                res_map.insert(
                    "header".to_string(),
                    Value::Hash(Rc::new(RefCell::new(header_map)))
                );
                res_map.insert(
                    "question".to_string(),
                    Value::Hash(Rc::new(RefCell::new(question_map)))
                );
                if answer_lst.len() > 0 {
                    res_map.insert(
                        "answer".to_string(),
                        Value::List(Rc::new(RefCell::new(answer_lst)))
                    );
                }
                if authority_lst.len() > 0 {
                    res_map.insert(
                        "authority".to_string(),
                        Value::List(Rc::new(RefCell::new(authority_lst)))
                    );
                }
                if additional_lst.len() > 0 {
                    res_map.insert(
                        "additional".to_string(),
                        Value::List(Rc::new(RefCell::new(additional_lst)))
                    );
                }
                self.stack.push(Value::Hash(Rc::new(RefCell::new(res_map))));
                return 1;
            }
            _ => {
                self.print_error("dns requires two string arguments");
                return 0;
            }
        }
    }

    /// Query for DNS records at the default server.
    pub fn core_dns(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("dns requires three arguments");
            return 0;
        }

        let type_rr = self.stack.pop().unwrap();
        let query_rr = self.stack.pop().unwrap();

        let server_addr = self.dns_servers.get(0);
        if let None = server_addr {
            self.print_error("unable to find default server for dns");
            return 0;
        }
        let server_rr = new_string_value(server_addr.unwrap().to_string());

        self.stack.push(server_rr);
        self.stack.push(query_rr);
        self.stack.push(type_rr);

        return self.core_dnsat();
    }
}

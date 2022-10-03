//use std::cell::RefCell;
//use std::convert::TryFrom;
//use std::rc::Rc;
//use std::str::FromStr;
use std::net::{Ipv4Addr, Ipv6Addr};

use ipnet::{Ipv4Net, Ipv6Net};
use num_bigint::{BigInt, BigUint};
use num_traits::{FromPrimitive, ToPrimitive, Zero};

use vm::*;

fn ipv4_addr_to_int(ipv4: Ipv4Addr) -> u32 {
    let octets = ipv4.octets();
    let n1: u32 = (octets[0].to_u32().unwrap() << 24).into();
    let n2: u32 = (octets[1].to_u32().unwrap() << 16).into();
    let n3: u32 = (octets[2].to_u32().unwrap() << 8).into();
    let n4: u32 = octets[3].into();
    let n = n1 | n2 | n3 | n4;
    return n;
}

fn ipv6_addr_to_int(ipv6: Ipv6Addr) -> BigUint {
    let octets = ipv6.octets();
    let mut n1 = BigUint::from(octets[0]) << 120;
    n1 = n1 | (BigUint::from(octets[1]) << 112);
    n1 = n1 | (BigUint::from(octets[2]) << 104);
    n1 = n1 | (BigUint::from(octets[3]) << 96);
    n1 = n1 | (BigUint::from(octets[4]) << 88);
    n1 = n1 | (BigUint::from(octets[5]) << 80);
    n1 = n1 | (BigUint::from(octets[6]) << 72);
    n1 = n1 | (BigUint::from(octets[7]) << 64);
    n1 = n1 | (BigUint::from(octets[8]) << 56);
    n1 = n1 | (BigUint::from(octets[9]) << 48);
    n1 = n1 | (BigUint::from(octets[10]) << 40);
    n1 = n1 | (BigUint::from(octets[11]) << 32);
    n1 = n1 | (BigUint::from(octets[12]) << 24);
    n1 = n1 | (BigUint::from(octets[13]) << 16);
    n1 = n1 | (BigUint::from(octets[14]) << 8);
    n1 = n1 | BigUint::from(octets[15]);
    return n1;
}

fn int_to_ipv4_addr(n: u32) -> Ipv4Addr {
    let o1 = n >> 24 & 0xFF;
    let o2 = n >> 16 & 0xFF;
    let o3 = n >> 8  & 0xFF;
    let o4 = n       & 0xFF;
    let ipv4 = Ipv4Addr::new(o1.try_into().unwrap(),
                             o2.try_into().unwrap(),
                             o3.try_into().unwrap(),
                             o4.try_into().unwrap());
    return ipv4;
}

fn int_to_ipv6_addr(n: BigUint) -> Ipv6Addr {
    let mask = BigUint::from_u32(0xFFFF).unwrap();
    let o1 = (n.clone() >> 112u16 & mask.clone()).to_u16().unwrap();
    let o2 = (n.clone() >> 96u16  & mask.clone()).to_u16().unwrap();
    let o3 = (n.clone() >> 80u16  & mask.clone()).to_u16().unwrap();
    let o4 = (n.clone() >> 64u16  & mask.clone()).to_u16().unwrap();
    let o5 = (n.clone() >> 48u16  & mask.clone()).to_u16().unwrap();
    let o6 = (n.clone() >> 32u16  & mask.clone()).to_u16().unwrap();
    let o7 = (n.clone() >> 16u16  & mask.clone()).to_u16().unwrap();
    let o8 = (n.clone()           & mask.clone()).to_u16().unwrap();
    let ipv6 = Ipv6Addr::new(o1, o2, o3, o4, o5, o6, o7, o8);
    return ipv6;
}

impl VM {
    /// Parses an IP address or range and returns an IP object.
    pub fn core_ip(&mut self) -> i32 {
	if self.stack.len() < 1 {
            self.print_error("ip requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let value_opt: Option<&str>;
        to_str!(value_rr, value_opt);

        match value_opt {
            Some(s) => {
                if s.contains(".") {
                    let ipv4_res;
                    if !s.contains("/") {
                        let s2 = format!("{}/32", s);
                        ipv4_res = Ipv4Net::from_str(&s2);
                    } else {
                        ipv4_res = Ipv4Net::from_str(s);
                    }
                    match ipv4_res {
                        Ok(ipv4) => {
                            let addr = ipv4.addr();
                            let addr_int = ipv4_addr_to_int(addr);
                            let prefix_len = ipv4.prefix_len();
                            if prefix_len == 0 && addr_int != 0 {
                                self.print_error("invalid prefix length");
                                return 0;
                            }
                            if !(prefix_len == 0 && addr_int == 0) {
                                let addr_check =
                                    addr_int & (1 << (32 - prefix_len)) - 1;
                                if addr_check != 0 {
                                    self.print_error("invalid prefix length");
                                    return 0;
                                }
                            }
                            self.stack.push(Value::Ipv4(ipv4));
                            return 1;
                        }
                        Err(e) => {
			    let err_str = format!("unable to parse IP address: {}",
						  e.to_string());
			    self.print_error(&err_str);
			    return 0;
                        }
                    }
                } else {
                    eprintln!("flag1");
                    let ipv6_res;
                    if !s.contains("/") {
                        let s2 = format!("{}/128", s);
                        ipv6_res = Ipv6Net::from_str(&s2);
                        eprintln!("flag2");
                    } else {
                        ipv6_res = Ipv6Net::from_str(s);
                    }
                    match ipv6_res {
                        Ok(ipv6) => {
                        eprintln!("flag3");
                            let addr = ipv6.addr();
                            let addr_int = ipv6_addr_to_int(addr);
                            let prefix_len = ipv6.prefix_len();
                            eprintln!("{} {} {}", addr, addr_int, prefix_len);
                            if prefix_len == 0 && !addr_int.is_zero() {
                                self.print_error("invalid prefix length");
                                return 0;
                            }
                            if !(prefix_len == 0 && addr_int == BigUint::from(0u8)) {
                                let prefix_mask = (BigUint::from(1u8) << (128 - prefix_len)) - BigUint::from(1u8);
                                let addr_check: BigUint = addr_int & prefix_mask;
                                if !addr_check.is_zero() {
                                    self.print_error("invalid prefix length");
                                    return 0;
                                }
                            }
                            self.stack.push(Value::Ipv6(ipv6));
                            return 1;
                        }
                        Err(e) => {
			    let err_str = format!("unable to parse IP address: {}",
						  e.to_string());
			    self.print_error(&err_str);
			    return 0;
                        }
                    }
                }
            }
            _ => {}
        }

        return 1;
    }

    /// Converts an integer into an IP object.
    pub fn core_ip_from_int(&mut self) -> i32 {
	if self.stack.len() < 2 {
            self.print_error("ip.from-int requires two arguments");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let value_opt = value_rr.to_bigint();

        let version_rr = self.stack.pop().unwrap();
        let version_opt = version_rr.to_int();

        match (version_opt, value_opt) {
            (Some(4), Some(value)) => {
                if value > BigInt::from_u32(0xFFFFFFFF).unwrap() {
                    self.print_error("IPv4 address is outside 32-bit bound");
                    return 0;
                }
                let uvalue =
                    value.to_biguint().unwrap().to_u32().unwrap();
                let ipv4 = int_to_ipv4_addr(uvalue);
                self.stack.push(Value::Ipv4(Ipv4Net::new(ipv4, 32).unwrap()));
            }
            (Some(6), Some(value)) => {
                let uvalue = value.to_biguint().unwrap();
                let ipv6 = int_to_ipv6_addr(uvalue);
                self.stack.push(Value::Ipv6(Ipv6Net::new(ipv6, 128).unwrap()));
            }
            (Some(_), _) => {
                self.print_error("invalid IP address version");
                return 0;
            }
            _ => {
                self.print_error("invalid IP integer");
                return 0;
            }
        }

        return 1;
    }

    /// Returns the first address of an IP object.
    pub fn core_ip_addr(&mut self) -> i32 {
	if self.stack.len() < 1 {
            self.print_error("ip.addr requires one argument");
            return 0;
        }

        let ip_rr = self.stack.pop().unwrap();
        match ip_rr {
            Value::Ipv4(ipv4net) => {
                let s = format!("{}", ipv4net);
                let snp = s.chars().take_while(|&c| c != '/').collect::<String>();
                let sp = StringPair::new(snp.to_string(), None);
                let st = Value::String(Rc::new(RefCell::new(sp)));
                self.stack.push(st);
                return 1;
            }
            Value::Ipv6(ipv6net) => {
                let s = format!("{}", ipv6net);
                let snp = s.chars().take_while(|&c| c != '/').collect::<String>();
                let sp = StringPair::new(snp.to_string(), None);
                let st = Value::String(Rc::new(RefCell::new(sp)));
                self.stack.push(st);
                return 1;
            }
            _ => {
                self.print_error("expected IP object argument");
                return 0;
            }
        }
    }

    /// Returns the prefix length of an IP object.
    pub fn core_ip_len(&mut self) -> i32 {
	if self.stack.len() < 1 {
            self.print_error("ip.len requires one argument");
            return 0;
        }

        let ip_rr = self.stack.pop().unwrap();
        match ip_rr {
            Value::Ipv4(ipv4net) => {
                let st = Value::Int(ipv4net.prefix_len().into());
                self.stack.push(st);
                return 1;
            }
            Value::Ipv6(ipv6net) => {
                let st = Value::Int(ipv6net.prefix_len().into());
                self.stack.push(st);
                return 1;
            }
            _ => {
                self.print_error("expected IP object argument");
                return 0;
            }
        }
    }

    /// Returns the first address of the IP object as an integer.
    pub fn core_ip_addr_int(&mut self) -> i32 {
	if self.stack.len() < 1 {
            self.print_error("ip.addr-int requires one argument");
            return 0;
        }

        let ip_rr = self.stack.pop().unwrap();
        match ip_rr {
            Value::Ipv4(ipv4net) => {
                let ipv4addr_int =
                    ipv4_addr_to_int(ipv4net.network());
                let st = Value::BigInt(BigInt::from(ipv4addr_int));
                self.stack.push(st);
                return 1;
            }
            Value::Ipv6(ipv6net) => {
                let ipv6addr_int =
                    ipv6_addr_to_int(ipv6net.network());
                let st = Value::BigInt(BigInt::from(ipv6addr_int));
                self.stack.push(st);
                return 1;
            }
            _ => {
                self.print_error("expected IP object argument");
                return 0;
            }
        }
    }

    /// Returns the last address of the IP object.
    pub fn core_ip_last_addr(&mut self) -> i32 {
	if self.stack.len() < 1 {
            self.print_error("ip.last-addr requires one argument");
            return 0;
        }

        let ip_rr = self.stack.pop().unwrap();
        match ip_rr {
            Value::Ipv4(ipv4net) => {
                if ipv4_addr_to_int(ipv4net.network()) == 0 && ipv4net.prefix_len() == 0 {
                    let f = format!("{}", "255.255.255.255");
                    let sp = StringPair::new(f, None);
                    let st = Value::String(Rc::new(RefCell::new(sp)));
                    self.stack.push(st);
                    return 1;
                }
                let ipv4addr_int =
                    ipv4_addr_to_int(ipv4net.network()) |
                        ((1 << (32 - ipv4net.prefix_len())) - 1);
                let last = int_to_ipv4_addr(ipv4addr_int);
                let f = format!("{}", last);
                let sp = StringPair::new(f, None);
                let st = Value::String(Rc::new(RefCell::new(sp)));
                self.stack.push(st);
                return 1;
            }
            Value::Ipv6(ipv6net) => {
                let prefix_mask =
                    (BigUint::from(1u8) << (128 - ipv6net.prefix_len())) - BigUint::from(1u8);
                let ipv6addr_int =
                    ipv6_addr_to_int(ipv6net.network()) | prefix_mask;
                let last = int_to_ipv6_addr(ipv6addr_int);
                let f = format!("{}", last);
                let sp = StringPair::new(f, None);
                let st = Value::String(Rc::new(RefCell::new(sp)));
                self.stack.push(st);
            }
            _ => {
                self.print_error("expected IP object argument");
                return 0;
            }
        }

        return 1;
    }

    /// Returns the last address of the IP object as an integer.
    pub fn core_ip_last_addr_int(&mut self) -> i32 {
	if self.stack.len() < 1 {
            self.print_error("ip.last-addr-int requires one argument");
            return 0;
        }

        let ip_rr = self.stack.pop().unwrap();
        match ip_rr {
            Value::Ipv4(ipv4net) => {
                if ipv4_addr_to_int(ipv4net.network()) == 0 && ipv4net.prefix_len() == 0 {
                    let st =
                        Value::BigInt(BigInt::from_u32(0xFFFFFFFF).unwrap());
                    self.stack.push(st);
                    return 1;
                }
                let ipv4addr_int =
                    ipv4_addr_to_int(ipv4net.network()) |
                        ((1 << (32 - ipv4net.prefix_len())) - 1);
                let st =
                    Value::BigInt(BigInt::from_u32(ipv4addr_int).unwrap());
                self.stack.push(st);
                return 1;
            }
            Value::Ipv6(ipv6net) => {
                let prefix_mask =
                    (BigUint::from(1u8) << (128 - ipv6net.prefix_len())) - BigUint::from(1u8);
                let ipv6addr_int =
                    ipv6_addr_to_int(ipv6net.network()) | prefix_mask;
                let st =
                    Value::BigInt(BigInt::from(ipv6addr_int));
                self.stack.push(st);
            }
            _ => {
                self.print_error("expected IP object argument");
                return 0;
            }
        }

        return 1;
    }

    /// Returns the number of hosts covered by this IP object.
    pub fn core_ip_size(&mut self) -> i32 {
	if self.stack.len() < 1 {
            self.print_error("ip.size requires one argument");
            return 0;
        }

        let ip_rr = self.stack.pop().unwrap();
        match ip_rr {
            Value::Ipv4(ipv4net) => {
                if ipv4_addr_to_int(ipv4net.network()) == 0 && ipv4net.prefix_len() == 0 {
                    let st =
                        Value::BigInt(BigInt::from_u32(0xFFFFFFFF).unwrap());
                    self.stack.push(st);
                    return 1;
                }
                let ipv4addr_int =
                    ipv4_addr_to_int(ipv4net.network());
                let ipv4addr_last_int =
                    ipv4addr_int | ((1 << (32 - ipv4net.prefix_len())) - 1);
                let res = ipv4addr_last_int - ipv4addr_int + 1;
                let st =
                    Value::BigInt(BigInt::from_u32(res).unwrap());
                self.stack.push(st);
                return 1;
            }
            Value::Ipv6(ipv6net) => {
                let prefix_mask =
                    (BigUint::from(1u8) << (128 - ipv6net.prefix_len())) - BigUint::from(1u8);
                let ipv6addr_int =
                    ipv6_addr_to_int(ipv6net.network());
                let ipv6addr_last_int =
                    ipv6addr_int.clone() | prefix_mask;
                let res = ipv6addr_last_int - ipv6addr_int + BigUint::from(1u8);
                let st =
                    Value::BigInt(BigInt::from(res));
                self.stack.push(st);
            }
            _ => {
                self.print_error("expected IP object argument");
                return 0;
            }
        }

        return 1;
    }

    /// Returns the IP object version.
    pub fn core_ip_version(&mut self) -> i32 {
	if self.stack.len() < 1 {
            self.print_error("ip.version requires one argument");
            return 0;
        }

        let ip_rr = self.stack.pop().unwrap();
        match ip_rr {
            Value::Ipv4(_) => {
                self.stack.push(Value::Int(4));
                return 1;
            }
            Value::Ipv6(_) => {
                self.stack.push(Value::Int(6));
            }
            _ => {
                self.print_error("expected IP object argument");
                return 0;
            }
        }

        return 1;
    }

    /// Returns the IP object as a string.
    pub fn core_ip_to_string(&mut self) -> i32 {
	if self.stack.len() < 1 {
            self.print_error("ip.version requires one argument");
            return 0;
        }

        let ip_rr = self.stack.pop().unwrap();
        match ip_rr {
            Value::Ipv4(ipv4net) => {
                let prefix_len = ipv4net.prefix_len();
                if prefix_len == 32 {
                    let s = format!("{}", ipv4net);
                    let snp = s.chars().take_while(|&c| c != '/').collect::<String>();
                    let sp = StringPair::new(snp.to_string(), None);
                    let st = Value::String(Rc::new(RefCell::new(sp)));
                    self.stack.push(st);
                } else {
                    let s = format!("{}", ipv4net);
                    let sp = StringPair::new(s, None);
                    let st = Value::String(Rc::new(RefCell::new(sp)));
                    self.stack.push(st);
                }
                return 1;
            }
            Value::Ipv6(ipv6net) => {
                let prefix_len = ipv6net.prefix_len();
                if prefix_len == 128 {
                    let s = format!("{}", ipv6net.network());
                    let sp = StringPair::new(s, None);
                    let st = Value::String(Rc::new(RefCell::new(sp)));
                    self.stack.push(st);
                } else {
                    let s = format!("{}/{}", ipv6net.network(), ipv6net.prefix_len());
                    let sp = StringPair::new(s, None);
                    let st = Value::String(Rc::new(RefCell::new(sp)));
                    self.stack.push(st);
                }
                return 1;
            }
            _ => {
                self.print_error("expected IP object argument");
                return 0;
            }
        }
    }
}

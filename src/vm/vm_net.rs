use std::cell::RefCell;
use std::rc::Rc;

use ipnetwork::IpNetwork::{V4, V6};
use num::FromPrimitive;
use num_bigint::BigInt;
use pnet::datalink;

use crate::chunk::{Value, new_string_value};
use crate::vm::*;

impl VM {
    pub fn core_ifconfig(&mut self) -> i32 {
        let interfaces = datalink::interfaces();
        let mut lst = VecDeque::new();
        for interface in interfaces {
            let mut map = IndexMap::new();
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

                        let mut netmap = IndexMap::new();
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

                        let mut netmap = IndexMap::new();
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
}

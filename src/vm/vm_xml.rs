use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use indexmap::IndexMap;

use chunk::{print_error, Chunk, Value};
use vm::*;

/// Converts a roxmltree object into a value.
fn convert_from_xml(node: &roxmltree::Node) -> Value {
    let mut map = IndexMap::new();
    let tag_name = node.tag_name().name();
    map.insert(
        "key".to_string(),
        Rc::new(RefCell::new(Value::String(tag_name.to_string(), None))),
    );
    if node.is_text() {
        let text_opt = node.text();
        match text_opt {
            None => {}
            Some(s) => {
                map.insert(
                    "text".to_string(),
                    Rc::new(RefCell::new(Value::String(s.to_string(), None))),
                );
            }
        }
        return Value::Hash(map);
    }

    let mut attr_map = IndexMap::new();
    for attr in node.attributes() {
        attr_map.insert(
            attr.name().to_string(),
            Rc::new(RefCell::new(Value::String(attr.value().to_string(), None))),
        );
    }
    map.insert(
        "attributes".to_string(),
        Rc::new(RefCell::new(Value::Hash(attr_map))),
    );

    let mut child_nodes = VecDeque::new();
    for child_node in node.children() {
        let child_node_value = convert_from_xml(&child_node);
        child_nodes.push_back(Rc::new(RefCell::new(child_node_value)));
    }
    map.insert("value".to_string(),
               Rc::new(RefCell::new(Value::List(child_nodes))));
    return Value::Hash(map);
}

/// Converts a value into an XML string.
fn convert_to_xml(v: &Value) -> Option<String> {
    let mut begin_open_element = String::new();
    let attributes;
    let mut begin_close_element = String::new();
    let mut text = String::new();
    let child_nodes;
    let mut end_element = String::new();
    match &*v {
        Value::Hash(vm) => {
            let key_opt = vm.get("key");
            match key_opt {
                Some(value_rr) => {
                    let value_rrb = value_rr.borrow();
                    match &*value_rrb {
                        Value::String(s, _) => {
                            if s != "" {
                                begin_open_element = format!("<{}", s);
                                begin_close_element = ">".to_string();
                                end_element = format!("</{}>", s);
                            }
                        }
                        _ => {}
                    }
                }
                None => {}
            }

            let attributes_opt = vm.get("attributes");
            let attributes_str = match attributes_opt {
                Some(attributes_rr) => {
                    let attributes_rrb = attributes_rr.borrow();
                    match &*attributes_rrb {
                        Value::Hash(map) => {
                            let mut has_none = false;
                            let attributes_str_lst = map
                                .iter()
                                .map(|(key, value_rr)| {
                                    let value_rrb = value_rr.borrow();
                                    let value_str_pre =
                                        value_rrb.to_string();
                                    let value_str_opt =
                                        to_string_2(&value_str_pre);
                                    match value_str_opt {
                                        Some(s) => {
                                            format!("{}=\"{}\"", key, s)
                                        }
                                        None => {
                                            has_none = true;
                                            "".to_string()
                                        }
                                    }
                                })
                                .collect::<Vec<_>>();
                            if has_none {
                                return None;
                            } else {
                                attributes_str_lst.join(" ")
                            }
                        }
                        _ => "".to_string(),
                    }
                }
                _ => "".to_string(),
            };
            attributes = if attributes_str != "" {
                format!(" {}", attributes_str)
            } else {
                "".to_owned()
            };

            let value_opt = vm.get("value");
            let mut has_none = false;
            child_nodes = match value_opt {
                Some(value_rr) => {
                    let value_rrb = value_rr.borrow();
                    match &*value_rrb {
                        Value::List(lst) => lst
                            .iter()
                            .map(|lst_value_rr| {
                                let lst_value_rrb = convert_to_xml(&lst_value_rr.borrow());
                                if lst_value_rrb.is_none() {
                                    has_none = true;
                                    "".to_string()
                                } else {
                                    lst_value_rrb.unwrap()
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(""),
                        _ => "".to_string(),
                    }
                }
                _ => "".to_string(),
            };
            if has_none {
                return None;
            }

            let text_opt = vm.get("text");
            match text_opt {
                Some(value_rr) => {
                    let value_rrb = value_rr.borrow();
                    match &*value_rrb {
                        Value::String(s, _) => {
                            text = s.to_string();
                        }
                        _ => {}
                    }
                }
                _ => {}
            };
            return Some(format!("{}{}{}{}{}{}", begin_open_element, attributes, begin_close_element, text, child_nodes, end_element));
        }
        _ => Some("".to_string()),
    }
}

impl VM {
    /// Takes an XML string, converts it into a hash, and puts the
    /// result onto the stack.
    pub fn core_from_xml(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "from-xml requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let value_rrb = value_rr.borrow();
        let value_pre = value_rrb.to_string();
        let value_opt = to_string_2(&value_pre);

        match value_opt {
            Some(s) => {
                let doc_res = roxmltree::Document::parse(s);
                let doc;
                match doc_res {
                    Err(e) => {
                        let err_str =
                            format!("unable to parse XML: {}", e.to_string());
                        print_error(chunk, i, &err_str);
                        return 0;
                    }
                    Ok(d) => {
                        doc = d;
                    }
                }
                let xml_rr = Rc::new(RefCell::new(convert_from_xml(
                    &doc.root_element(),
                )));
                self.stack.push(xml_rr);
            }
            _ => {
                print_error(chunk, i, "from-xml argument must be string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a hash that is the result of calling `from-xml`, converts
    /// it into a string representation, and puts the result onto the
    /// stack.
    pub fn core_to_xml(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "to-xml requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let value_rrb = value_rr.borrow();
        let doc_opt = convert_to_xml(&value_rrb);
        if doc_opt.is_none() {
            print_error(chunk, i, "unable to convert value to XML");
            return 0;
        }
        self.stack
            .push(Rc::new(RefCell::new(Value::String(doc_opt.unwrap(), None))));
        return 1;
    }
}

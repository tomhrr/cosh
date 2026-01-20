use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::chunk::{StringTriple, Value};
use crate::hasher::new_hash_indexmap;
use crate::vm::*;

/// Converts a value into an XML string.
fn convert_to_xml(v: &Value) -> Option<String> {
    let mut begin_open_element = String::new();
    let attributes;
    let namespaces;
    let mut begin_close_element = String::new();
    let mut text = String::new();
    let child_nodes;
    let mut end_element = String::new();
    match v {
        Value::Hash(vm) => {
            let vmm = vm.borrow();
            let key_opt = vmm.get("key");
            if let Some(Value::String(st)) = key_opt {
                let s = &st.borrow().string;
                if !s.is_empty() {
                    begin_open_element = format!("<{}", s);
                    begin_close_element = ">".to_string();
                    end_element = format!("</{}>", s);
                }
            }

            let attributes_opt = vmm.get("attributes");
            let attributes_str = match attributes_opt {
                Some(Value::Hash(map)) => {
                    let mut has_none = false;
                    let attributes_str_lst = map
                        .borrow()
                        .iter()
                        .map(|(key, value_rr)| {
                            let value_str_opt: Option<&str>;
                            to_str!(value_rr, value_str_opt);

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
            };
            attributes = if !attributes_str.is_empty() {
                format!(" {}", attributes_str)
            } else {
                "".to_owned()
            };

            let namespaces_opt = vmm.get("namespaces");
            let namespaces_str = match namespaces_opt {
                Some(Value::List(lst)) => {
                    let namespaces_lst = lst
                        .borrow()
                        .iter()
                        .map(|el| match el {
                            Value::Hash(hsh) => {
                                let hb = hsh.borrow();
                                let uri_opt = hb.get("uri").unwrap();
                                let name_opt = hb.get("name").unwrap();

                                let uri_str_opt: Option<&str>;
                                to_str!(uri_opt, uri_str_opt);
                                let name_str_opt: Option<&str>;
                                to_str!(name_opt, name_str_opt);

                                match (name_str_opt, uri_str_opt) {
                                    (Some(name), Some(uri)) => {
                                        if name.eq("") {
                                            format!("xmlns=\"{}\"", uri)
                                        } else {
                                            format!("xmlns:{}=\"{}\"", name, uri)
                                        }
                                    }
                                    _ => "".to_string(),
                                }
                            }
                            _ => "".to_string(),
                        })
                        .collect::<Vec<_>>();
                    namespaces_lst.join(" ")
                }
                _ => "".to_string(),
            };
            namespaces = if !namespaces_str.is_empty() {
                format!(" {}", namespaces_str)
            } else {
                "".to_owned()
            };

            let value_opt = vmm.get("value");
            let mut has_none = false;
            child_nodes = match value_opt {
                Some(Value::List(lst)) => lst
                    .borrow()
                    .iter()
                    .map(|lst_value_rr| {
                        let lst_value_rrb = convert_to_xml(lst_value_rr);
                        match lst_value_rrb {
                            Some(lst_value) => lst_value,
                            None => {
                                has_none = true;
                                "".to_string()
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(""),
                _ => "".to_string(),
            };
            if has_none {
                return None;
            }

            if vmm.contains_key("text") {
                let t = vmm.get("text").unwrap();
                if let Value::String(ts) = t {
                    text = ts.borrow().string.clone();
                } else {
                    let s_opt = t.to_string();
                    match s_opt {
                        Some(s) => {
                            text = s;
                        }
                        None => {
                            let type_str = v.type_string();
                            text = format!("v[{}]", type_str);
                        }
                    }
                }
            }
            Some(format!(
                "{}{}{}{}{}{}{}",
                begin_open_element,
                namespaces,
                attributes,
                begin_close_element,
                text,
                child_nodes,
                end_element
            ))
        }
        _ => Some("".to_string()),
    }
}

impl VM {
    /// Converts a roxmltree object into a value.
    fn convert_from_xml(
        &mut self,
        node: &roxmltree::Node,
        param_namespaces: &HashMap<String, String>,
    ) -> Value {
        let mut map = new_hash_indexmap();

        let mut current_namespaces = param_namespaces;
        let mut changed_namespaces = false;
        for ns in node.namespaces() {
            let uri = ns.uri();
            let ns_name_opt = ns.name();
            let name = match ns_name_opt {
                Some(ns_name) => ns_name.to_string(),
                None => "".to_string(),
            };

            if let Some(prev_name) = current_namespaces.get(uri) {
                if name.eq(prev_name) {
                    continue;
                }
            }

            changed_namespaces = true;
            break;
        }

        let mut new_namespaces;
        if changed_namespaces {
            let mut node_namespaces = VecDeque::new();
            new_namespaces = current_namespaces.clone();

            for ns in node.namespaces() {
                let uri = ns.uri();
                let ns_name_opt = ns.name();
                let name = match ns_name_opt {
                    Some(ns_name) => ns_name.to_string(),
                    None => "".to_string(),
                };

                if let Some(prev_name) = current_namespaces.get(uri) {
                    if name.eq(prev_name) {
                        continue;
                    }
                }

                let mut ns_map = new_hash_indexmap();
                ns_map.insert(
                    "uri".to_string(),
                    Value::String(Rc::new(RefCell::new(StringTriple::new(
                        uri.to_string(),
                        None,
                    )))),
                );
                ns_map.insert(
                    "name".to_string(),
                    Value::String(Rc::new(RefCell::new(StringTriple::new(
                        name.to_string(),
                        None,
                    )))),
                );
                node_namespaces.push_back(Value::Hash(Rc::new(RefCell::new(ns_map))));
                new_namespaces.insert(uri.to_string(), name.to_string());
            }
            map.insert(
                "namespaces".to_string(),
                Value::List(Rc::new(RefCell::new(node_namespaces))),
            );
            current_namespaces = &new_namespaces;
        }

        let tag_name = node.tag_name();
        let tag_name_str = tag_name.name().to_string();
        let tag_name_ns = tag_name.namespace();

        let key = match tag_name_ns {
            Some(tag_ns) => {
                let ns_prefix_opt = current_namespaces.get(tag_ns);
                if ns_prefix_opt.is_none() {
                    self.print_error("invalid XML namespace");
                    return Value::Null;
                }
                let ns_prefix = ns_prefix_opt.unwrap();
                if !ns_prefix.eq("") {
                    format!("{}:{}", ns_prefix, tag_name_str)
                } else {
                    tag_name_str
                }
            }
            None => tag_name_str,
        };

        map.insert(
            "key".to_string(),
            Value::String(Rc::new(RefCell::new(StringTriple::new(key, None)))),
        );
        if node.is_text() {
            let text_opt = node.text();
            match text_opt {
                None => {}
                Some(s) => {
                    map.insert(
                        "text".to_string(),
                        Value::String(Rc::new(RefCell::new(StringTriple::new(
                            s.to_string(),
                            None,
                        )))),
                    );
                }
            }
            return Value::Hash(Rc::new(RefCell::new(map)));
        }

        let mut attr_map = new_hash_indexmap();
        for attr in node.attributes() {
            attr_map.insert(
                attr.name().to_string(),
                Value::String(Rc::new(RefCell::new(StringTriple::new(
                    attr.value().to_string(),
                    None,
                )))),
            );
        }
        map.insert(
            "attributes".to_string(),
            Value::Hash(Rc::new(RefCell::new(attr_map))),
        );

        let mut child_nodes = VecDeque::new();
        for child_node in node.children() {
            let child_node_value = self.convert_from_xml(&child_node, current_namespaces);
            child_nodes.push_back(child_node_value);
        }
        map.insert(
            "value".to_string(),
            Value::List(Rc::new(RefCell::new(child_nodes))),
        );
        Value::Hash(Rc::new(RefCell::new(map)))
    }

    /// Takes an XML string, converts it into a hash, and puts the
    /// result onto the stack.
    pub fn core_from_xml(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("from-xml requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        if !value_rr.is_generator() {
            let value_opt: Option<&str>;
            to_str!(value_rr, value_opt);

            match value_opt {
                Some(s) => {
                    let doc_res = roxmltree::Document::parse(s);
                    let doc;
                    match doc_res {
                        Err(e) => {
                            let err_str = format!("unable to parse XML: {}", e);
                            self.print_error(&err_str);
                            return 0;
                        }
                        Ok(d) => {
                            doc = d;
                        }
                    }
                    let namespaces = HashMap::new();
                    let xml_rr = self.convert_from_xml(&doc.root_element(), &namespaces);
                    self.stack.push(xml_rr);
                    1
                }
                _ => {
                    self.print_error("from-xml argument must be string or generator");
                    0
                }
            }
        } else {
            self.stack.push(value_rr);
            self.stack
                .push(Value::String(Rc::new(RefCell::new(StringTriple::new(
                    "".to_string(),
                    None,
                )))));
            let function_rr = self.string_to_callable("join").unwrap();
            let res = self.call(OpCode::Call, function_rr);
            if !res {
                return 0;
            }
            self.core_from_xml()
        }
    }

    /// Takes a hash that is the result of calling `from-xml`, converts
    /// it into a string representation, and puts the result onto the
    /// stack.
    pub fn core_to_xml(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("to-xml requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let doc_opt = convert_to_xml(&value_rr);
        if doc_opt.is_none() {
            self.print_error("unable to convert value to XML");
            return 0;
        }
        self.stack
            .push(Value::String(Rc::new(RefCell::new(StringTriple::new(
                doc_opt.unwrap(),
                None,
            )))));
        1
    }
}

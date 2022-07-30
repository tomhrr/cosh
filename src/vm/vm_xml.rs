use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use indexmap::IndexMap;

use chunk::Value;
use vm::*;


impl VM {
    /// Converts a roxmltree object into a value.
    fn convert_from_xml(&mut self, interner: &mut StringInterner, node: &roxmltree::Node) -> Value {
        let mut map = IndexMap::new();
        let tag_name = node.tag_name().name();
        map.insert(
            self.intern_string(interner, "key"),
            self.intern_string_to_value(interner, tag_name)
        );
        if node.is_text() {
            let text_opt = node.text();
            match text_opt {
                None => {}
                Some(s) => {
                    map.insert(
                        self.intern_string(interner, "text"),
                        self.intern_string_to_value(interner, s),
                    );
                }
            }
            return Value::Hash(Rc::new(RefCell::new(map)));
        }
    
        let mut attr_map = IndexMap::new();
        for attr in node.attributes() {
            attr_map.insert(
                self.intern_string(interner, attr.name()),
                self.intern_string_to_value(interner, attr.value()),
            );
        }
        map.insert(
            self.intern_string(interner, "attributes"),
            Value::Hash(Rc::new(RefCell::new(attr_map))),
        );
    
        let mut child_nodes = VecDeque::new();
        for child_node in node.children() {
            let child_node_value = self.convert_from_xml(interner, &child_node);
            child_nodes.push_back(child_node_value);
        }
        map.insert(
            self.intern_string(interner, "value"),
            Value::List(Rc::new(RefCell::new(child_nodes))),
        );
        return Value::Hash(Rc::new(RefCell::new(map)));
    }

    /// Converts a value into an XML string.
    fn convert_to_xml(&mut self, interner: &mut StringInterner, v: &Value) -> Option<String> {
        let mut begin_open_element = String::new();
        let attributes;
        let mut begin_close_element = String::new();
        let mut text = String::new();
        let child_nodes;
        let mut end_element = String::new();
        match &*v {
            Value::Hash(vm) => {
                let vmm = vm.borrow();
                let key_opt = vmm.get(&self.intern_string(interner, "key"));
                match key_opt {
                    Some(value_rr) => match value_rr {
                        Value::String(sp) => {
                            let s = self.interner_resolve(interner, *sp);
                            if s != "" {
                                begin_open_element = format!("<{}", s);
                                begin_close_element = ">".to_string();
                                end_element = format!("</{}>", s);
                            }
                        }
                        _ => {}
                    },
                    None => {}
                }
    
                let attributes_opt = vmm.get(&self.intern_string(interner, "attributes"));
                let attributes_str = match attributes_opt {
                    Some(attributes_rr) => match attributes_rr {
                        Value::Hash(map) => {
                            let mut has_none = false;
                            let attributes_str_lst = map
                                .borrow()
                                .iter()
                                .map(|(key, value_rr)| {
                                    let value_str_opt =
                                        self.intern_string_value(interner, value_rr.clone());
                                    let s1 =
                                        self.interner_resolve(interner, *key).to_string();
                                    match value_str_opt {
                                        Some(s) => {
                                            let s2 =
                                                self.interner_resolve(interner, s);
                                            format!("{}=\"{}\"", s1, s2)
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
                    },
                    _ => "".to_string(),
                };
                attributes = if attributes_str != "" {
                    format!(" {}", attributes_str)
                } else {
                    "".to_owned()
                };
    
                let value_opt = vmm.get(&self.intern_string(interner, "value"));
                let mut has_none = false;
                child_nodes = match value_opt {
                    Some(value_rr) => match value_rr {
                        Value::List(lst) => lst
                            .borrow()
                            .iter()
                            .map(|lst_value_rr| {
                                let lst_value_rrb =
                                    self.convert_to_xml(interner, &lst_value_rr);
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
                    },
                    _ => "".to_string(),
                };
                if has_none {
                    return None;
                }
    
                let text_opt = vmm.get(&self.intern_string(interner, "text"));
                match text_opt {
                    Some(value_rr) => match value_rr {
                        Value::String(sp) => {
                            text =
                                self.interner_resolve(interner, *sp).to_string();
                        }
                        _ => {}
                    },
                    _ => {}
                };
                return Some(format!(
                    "{}{}{}{}{}{}",
                    begin_open_element, attributes, begin_close_element, text, child_nodes, end_element
                ));
            }
            _ => Some("".to_string()),
        }
    }

    /// Takes an XML string, converts it into a hash, and puts the
    /// result onto the stack.
    pub fn core_from_xml(&mut self, interner: &mut StringInterner) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("from-xml requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let s = self.intern_string_value(interner, value_rr);
        if s.is_none() {
            self.print_error("from-xml argument must be string");
            return 0;
        }
        let ss = s.unwrap();
        let doc;
        let ss2 = self.interner_resolve(interner, ss).to_string();

        let doc_res = roxmltree::Document::parse(&ss2);
        match doc_res {
            Err(e) => {
                let err_str = format!("unable to parse XML: {}", e.to_string());
                self.print_error(&err_str);
                return 0;
            }
            Ok(d) => {
                doc = d;
            }
        }
        let xml_rr = self.convert_from_xml(interner, &doc.root_element());
        self.stack.push(xml_rr);

        return 1;
    }

    /// Takes a hash that is the result of calling `from-xml`, converts
    /// it into a string representation, and puts the result onto the
    /// stack.
    pub fn core_to_xml(&mut self, interner: &mut StringInterner) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("to-xml requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let doc_opt = self.convert_to_xml(interner, &value_rr);
        if doc_opt.is_none() {
            self.print_error("unable to convert value to XML");
            return 0;
        }
        let c = self.intern_string_to_value(interner, &doc_opt.unwrap());
        self.stack.push(c);
        return 1;
    }
}

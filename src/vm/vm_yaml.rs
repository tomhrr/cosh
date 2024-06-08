use std::cell::RefCell;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::rc::Rc;

use indexmap::IndexMap;
use num_bigint::ToBigInt;

use crate::chunk::Value;
use crate::vm::*;

/// Converts a serde_yaml object into a value.
fn convert_from_yaml(v: &serde_yaml::value::Value) -> Value {
    match v {
        serde_yaml::value::Value::Null => Value::Null,
        serde_yaml::value::Value::Bool(true) => Value::Bool(true),
        serde_yaml::value::Value::Bool(false) => Value::Bool(false),
        serde_yaml::value::Value::Number(n) => {
            if n.is_i64() {
                let n_uw = n.as_i64().unwrap();
                let n2_res = i32::try_from(n_uw);
                match n2_res {
                    Ok(n2) => Value::Int(n2),
                    _ => Value::BigInt(n_uw.to_bigint().unwrap()),
                }
            } else if n.is_u64() {
                let n_uw = n.as_u64().unwrap();
                let n2_res = i32::try_from(n_uw);
                match n2_res {
                    Ok(n2) => Value::Int(n2),
                    _ => Value::BigInt(n_uw.to_bigint().unwrap()),
                }
            } else {
                Value::Float(n.as_f64().unwrap())
            }
        }
        serde_yaml::value::Value::String(s) =>
            new_string_value(s.to_string()),
        serde_yaml::value::Value::Sequence(lst) => Value::List(Rc::new(RefCell::new(
            lst.iter().map(convert_from_yaml).collect::<VecDeque<_>>(),
        ))),
        serde_yaml::value::Value::Mapping(map) => Value::Hash(Rc::new(RefCell::new(
            map.iter()
                .map(|(k, v)| {
                    let k_value = convert_from_yaml(k);
                    let k_value_opt: Option<&str>;
                    to_str!(k_value, k_value_opt);
                    (k_value_opt.unwrap().to_string(), convert_from_yaml(v))
                })
                .collect::<IndexMap<_, _>>(),
        ))),
        _ => Value::Null
    }
}

/// Convert a value into a YAML string.
fn convert_to_yaml(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Byte(n) => n.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(st) => {
            format!("\"{}\"", &st.borrow().string)
        }
        Value::List(lst) => {
            let s = lst
                .borrow()
                .iter()
                .map(convert_to_yaml)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{}]", s)
        }
        Value::Hash(vm) => {
            let s = vm
                .borrow()
                .iter()
                .map(|(k, v_rr)| format!("\"{}\":{}", k, convert_to_yaml(v_rr)))
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{}}}", s)
        }
        _ => {
            let s_opt = v.to_string();
            match s_opt {
                Some(s) => {
                    format!("\"{}\"", s)
                }
                None => {
                    let type_str = v.type_string();
                    let s = format!("\"v[{}]\"", type_str);
                    s
                }
            }
        }
    }
}

impl VM {
    /// Takes a YAML string, converts it into a hash, and puts the
    /// result onto the stack.
    pub fn core_from_yaml(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("from-yaml requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        if !value_rr.is_generator() {
            let value_opt: Option<&str>;
            to_str!(value_rr, value_opt);

            match value_opt {
                Some(s) => {
                    let doc_res = serde_yaml::from_str(s);
                    let doc;
                    match doc_res {
                        Err(e) => {
                            let err_str = format!("from-yaml argument is not valid YAML: {}", e);
                            self.print_error(&err_str);
                            return 0;
                        }
                        Ok(d) => {
                            doc = d;
                        }
                    }
                    let yaml_rr = convert_from_yaml(&doc);
                    self.stack.push(yaml_rr);
                    1
                }
                _ => {
                    self.print_error("from-yaml argument must be string or generator");
                    0
                }
            }
        } else {
            self.stack.push(value_rr);
            self.stack.push(new_string_value("".to_string()));
            let function_rr = self.string_to_callable("join").unwrap();
            let res = self.call(OpCode::Call, function_rr);
            if !res {
                return 0;
            }
            self.core_from_yaml()
        }
    }

    /// Takes a hash, converts it into a YAML string representation,
    /// and puts the result onto the stack.
    pub fn core_to_yaml(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("to-yaml requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        self.stack.push(new_string_value(convert_to_yaml(&value_rr)));
        1
    }
}

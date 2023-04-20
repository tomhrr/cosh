use std::cell::RefCell;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::rc::Rc;

use indexmap::IndexMap;
use num_bigint::ToBigInt;

use crate::chunk::{StringTriple, Value};
use crate::vm::*;

/// Converts a serde_json object into a value.
fn convert_from_json(v: &serde_json::value::Value) -> Value {
    match v {
        serde_json::value::Value::Null => Value::Null,
        serde_json::value::Value::Bool(true) => Value::Bool(true),
        serde_json::value::Value::Bool(false) => Value::Bool(false),
        serde_json::value::Value::Number(n) => {
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
        serde_json::value::Value::String(s) => Value::String(Rc::new(RefCell::new(
            StringTriple::new(s.to_string(), None),
        ))),
        serde_json::value::Value::Array(lst) => Value::List(Rc::new(RefCell::new(
            lst.iter().map(convert_from_json).collect::<VecDeque<_>>(),
        ))),
        serde_json::value::Value::Object(map) => Value::Hash(Rc::new(RefCell::new(
            map.iter()
                .map(|(k, v)| (k.to_string(), convert_from_json(v)))
                .collect::<IndexMap<_, _>>(),
        ))),
    }
}

/// Convert a value into a JSON string.
fn convert_to_json(v: &Value) -> String {
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
                .map(convert_to_json)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{}]", s)
        }
        Value::Hash(vm) => {
            let s = vm
                .borrow()
                .iter()
                .map(|(k, v_rr)| format!("\"{}\":{}", k, convert_to_json(v_rr)))
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{}}}", s)
        }
        _ => "".to_string(),
    }
}

impl VM {
    /// Takes a JSON string, converts it into a hash, and puts the
    /// result onto the stack.
    pub fn core_from_json(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("from-json requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        if !value_rr.is_generator() {
            let value_opt: Option<&str>;
            to_str!(value_rr, value_opt);

            match value_opt {
                Some(s) => {
                    let doc_res = serde_json::from_str(s);
                    let doc;
                    match doc_res {
                        Err(e) => {
                            let err_str = format!("from-json argument is not valid JSON: {}", e);
                            self.print_error(&err_str);
                            return 0;
                        }
                        Ok(d) => {
                            doc = d;
                        }
                    }
                    let json_rr = convert_from_json(&doc);
                    self.stack.push(json_rr);
                    1
                }
                _ => {
                    self.print_error("from-json argument must be string or generator");
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
            self.core_from_json()
        }
    }

    /// Takes a hash, converts it into a JSON string representation,
    /// and puts the result onto the stack.
    pub fn core_to_json(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("to-json requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        self.stack
            .push(Value::String(Rc::new(RefCell::new(StringTriple::new(
                convert_to_json(&value_rr),
                None,
            )))));

        1
    }
}

use std::cell::RefCell;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::rc::Rc;

use indexmap::IndexMap;
use num_bigint::ToBigInt;

use chunk::{print_error, Chunk, StringPair, Value};
use vm::*;

/// Converts a serde_json object into a value.
fn convert_from_json(v: &serde_json::value::Value) -> Value {
    match &*v {
        serde_json::value::Value::Null => Value::Null,
        serde_json::value::Value::Bool(true) => Value::Int(1),
        serde_json::value::Value::Bool(false) => Value::Int(0),
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
        serde_json::value::Value::String(s) => {
            eprintln!("string {}", s);
            Value::String(Rc::new(RefCell::new(StringPair::new(s.to_string(), None))))
        }
        serde_json::value::Value::Array(lst) => Value::List(Rc::new(RefCell::new(
            lst.iter()
                .map(|v| convert_from_json(v))
                .collect::<VecDeque<_>>(),
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
    match &*v {
        Value::Null => "null".to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(sp) => {
            format!("\"{}\"", &sp.borrow().s)
        }
        Value::List(lst) => {
            let s = lst
                .borrow()
                .iter()
                .map(|v_rr| convert_to_json(&v_rr))
                .collect::<Vec<_>>()
                .join(",");
            format!("[{}]", s)
        }
        Value::Hash(vm) => {
            let s = vm
                .borrow()
                .iter()
                .map(|(k, v_rr)| format!("\"{}\":{}", k, convert_to_json(&v_rr)))
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
    pub fn core_from_json(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "from-json requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let value_s;
        let value_b;
        let value_str;
        let value_bk: Option<String>;
        let value_opt: Option<&str> = match value_rr {
            Value::String(sp) => {
                value_s = sp;
                value_b = value_s.borrow();
                Some(&value_b.s)
            }
            _ => {
                value_bk = value_rr.to_string();
                match value_bk {
                    Some(s) => {
                        value_str = s;
                        Some(&value_str)
                    }
                    _ => None,
                }
            }
        };

        match value_opt {
            Some(s) => {
                let doc_res = serde_json::from_str(s);
                let doc;
                match doc_res {
                    Err(e) => {
                        let err_str = format!("unable to parse JSON: {}", e.to_string());
                        print_error(chunk, i, &err_str);
                        return 0;
                    }
                    Ok(d) => {
                        doc = d;
                    }
                }
                let json_rr = convert_from_json(&doc);
                self.stack.push(json_rr);
            }
            _ => {
                print_error(chunk, i, "from-json argument must be string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a hash, converts it into a JSON string representation,
    /// and puts the result onto the stack.
    pub fn core_to_json(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "to-json requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        self.stack
            .push(Value::String(Rc::new(RefCell::new(StringPair::new(
                convert_to_json(&value_rr),
                None,
            )))));

        return 1;
    }
}

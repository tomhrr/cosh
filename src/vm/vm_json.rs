use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use indexmap::IndexMap;

use chunk::{print_error, Chunk, Value};
use vm::*;

/// Converts a serde_json object into a value.
fn convert_from_json(v: &serde_json::value::Value) -> Value {
    match &*v {
        serde_json::value::Value::Null => Value::Null,
        serde_json::value::Value::Bool(true) => Value::Int(1),
        serde_json::value::Value::Bool(false) => Value::Int(0),
        serde_json::value::Value::Number(n) => {
            if n.is_i64() {
                Value::Int(n.as_i64().unwrap() as i32)
            } else if n.is_u64() {
                Value::Int(n.as_u64().unwrap() as i32)
            } else {
                Value::Float(n.as_f64().unwrap())
            }
        }
        serde_json::value::Value::String(s) => {
            Value::String(s.to_string(), None)
        }
        serde_json::value::Value::Array(lst) => Value::List(
            lst.iter()
                .map(|v| Rc::new(RefCell::new(convert_from_json(v))))
                .collect::<VecDeque<_>>(),
        ),
        serde_json::value::Value::Object(map) => Value::Hash(
            map.iter()
                .map(|(k, v)| {
                    (k.to_string(), Rc::new(RefCell::new(convert_from_json(v))))
                })
                .collect::<IndexMap<_, _>>(),
        ),
    }
}

/// Convert a value into a JSON string.
fn convert_to_json(v: &Value) -> String {
    match &*v {
        Value::Null => "null".to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s, _) => {
            format!("\"{}\"", s)
        }
        Value::List(lst) => {
            let s = lst
                .iter()
                .map(|v_rr| convert_to_json(&v_rr.borrow()))
                .collect::<Vec<_>>()
                .join(",");
            format!("[{}]", s)
        }
        Value::Hash(vm) => {
            let s = vm
                .iter()
                .map(|(k, v_rr)| {
                    format!("\"{}\":{}", k, convert_to_json(&v_rr.borrow()))
                })
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
        let value_rrb = value_rr.borrow();
        let value_pre = value_rrb.to_string();
        let value_opt = to_string_2(&value_pre);

        match value_opt {
            Some(s) => {
                let doc_res = serde_json::from_str(s);
                let doc;
                match doc_res {
                    Err(e) => {
                        let err_str =
                            format!("unable to parse JSON: {}", e.to_string());
                        print_error(chunk, i, &err_str);
                        return 0;
                    }
                    Ok(d) => {
                        doc = d;
                    }
                }
                let json_rr = Rc::new(RefCell::new(convert_from_json(&doc)));
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
        let value_rrb = value_rr.borrow();
        self.stack.push(Rc::new(RefCell::new(Value::String(
            convert_to_json(&value_rrb),
            None,
        ))));

        return 1;
    }
}
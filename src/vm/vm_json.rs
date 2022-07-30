use std::cell::RefCell;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::rc::Rc;

use indexmap::IndexMap;
use num_bigint::ToBigInt;

use chunk::Value;
use vm::*;

impl VM {
    /// Converts a serde_json object into a value.
    fn convert_from_json(&mut self, interner: &mut StringInterner, v: &serde_json::value::Value) -> Value {
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
                Value::String(self.intern_string(interner, &s.to_string()))
            }
            serde_json::value::Value::Array(lst) => Value::List(Rc::new(RefCell::new(
                lst.iter()
                    .map(|v| self.convert_from_json(interner, v))
                    .collect::<VecDeque<_>>(),
            ))),
            serde_json::value::Value::Object(map) => Value::Hash(Rc::new(RefCell::new(
                map.iter()
                    .map(|(k, v)| (self.intern_string(interner, &k.to_string()),
                    self.convert_from_json(interner, v)))
                    .collect::<IndexMap<_, _>>(),
            ))),
        }
    }
    
    /// Convert a value into a JSON string.
    fn convert_to_json(&mut self, interner: &mut StringInterner, v: &Value) -> String {
        match &*v {
            Value::Null => "null".to_string(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(sp) => {
                format!("\"{}\"", self.interner_resolve(interner, *sp))
            }
            Value::List(lst) => {
                let s = lst
                    .borrow()
                    .iter()
                    .map(|v_rr| self.convert_to_json(interner, &v_rr))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("[{}]", s)
            }
            Value::Hash(vm) => {
                let s = vm
                    .borrow()
                    .iter()
                    .map(|(k, v_rr)| format!("\"{}\":{}",
                        self.interner_resolve(interner, *k).to_string(),
                        self.convert_to_json(interner, &v_rr)))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{{{}}}", s)
            }
            _ => "".to_string(),
        }
    }

    /// Takes a JSON string, converts it into a hash, and puts the
    /// result onto the stack.
    pub fn core_from_json(&mut self, interner: &mut StringInterner) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("from-json requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let value_opt = self.intern_string_value(interner, value_rr);

        match value_opt {
            Some(ss) => {
                let s = self.interner_resolve(interner, ss);
                let doc_res = serde_json::from_str(s);
                let doc;
                match doc_res {
                    Err(e) => {
                        let err_str = format!("unable to parse JSON: {}", e.to_string());
                        self.print_error(&err_str);
                        return 0;
                    }
                    Ok(d) => {
                        doc = d;
                    }
                }
                let json_rr = self.convert_from_json(interner, &doc);
                self.stack.push(json_rr);
            }
            _ => {
                self.print_error("from-json argument must be string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a hash, converts it into a JSON string representation,
    /// and puts the result onto the stack.
    pub fn core_to_json(&mut self, interner: &mut StringInterner) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("to-json requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let sj = self.convert_to_json(interner, &value_rr);
        let c = self.intern_string_to_value(interner, &sj);
        self.stack.push(c);

        return 1;
    }
}

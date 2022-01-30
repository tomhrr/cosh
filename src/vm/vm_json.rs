use std::cell::RefCell;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::rc::Rc;

use indexmap::IndexMap;
use num_bigint::ToBigInt;

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
                let n_uw = n.as_i64().unwrap();
                let n2_res = i32::try_from(n_uw);
                match n2_res {
                    Ok(n2) => {
                        Value::Int(n2)
                    }
                    _ => {
                        Value::BigInt(n_uw.to_bigint().unwrap())
                    }
                }
            } else if n.is_u64() {
                let n_uw = n.as_u64().unwrap();
                let n2_res = i32::try_from(n_uw);
                match n2_res {
                    Ok(n2) => {
                        Value::Int(n2)
                    }
                    _ => {
                        Value::BigInt(n_uw.to_bigint().unwrap())
                    }
                }
            } else {
                Value::Float(n.as_f64().unwrap())
            }
        }
        serde_json::value::Value::String(s) => {
            eprintln!("string {}", s);
            Value::String(s.to_string(), None)
        }
        serde_json::value::Value::Array(lst) => Value::List(
            lst.iter()
                .map(|v| {
                    let c = convert_from_json(v);
                    match c {
                        Value::Null     => RValue::Raw(c),
                        Value::Int(_)   => RValue::Raw(c),
                        Value::Float(_) => RValue::Raw(c),
                        _               => RValue::Ref(Rc::new(RefCell::new(c)))
                    }})
                .collect::<VecDeque<_>>(),
        ),
        serde_json::value::Value::Object(map) => Value::Hash(
            map.iter()
                .map(|(k, v)| {
                    (k.to_string(), RValue::Ref(Rc::new(RefCell::new(convert_from_json(v)))))
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
                .map(|v_rr| {
		    let mut v_rm;
		    let v_rrb = match v_rr {
			RValue::Raw(ref v) => v,
			RValue::Ref(ref v_rc) => {
			    v_rm = v_rc.borrow();
			    &*v_rm
			}
		    };
                    convert_to_json(v_rrb)
                }) 
                .collect::<Vec<_>>()
                .join(",");
            format!("[{}]", s)
        }
        Value::Hash(vm) => {
            let s = vm
                .iter()
                .map(|(k, v_rr)| {
		    let mut v_rm;
		    let v_rrb = match v_rr {
			RValue::Raw(ref v) => v,
			RValue::Ref(ref v_rc) => {
			    v_rm = v_rc.borrow();
			    &*v_rm
			}
		    };
                    format!("\"{}\":{}", k, convert_to_json(v_rrb))
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
        let mut value_rm;
        let value_rrb = match value_rr {
            RValue::Raw(ref v) => v,
            RValue::Ref(ref v_rc) => {
                value_rm = v_rc.borrow();
                &*value_rm
            }
        };
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
                let json_rr = RValue::Ref(Rc::new(RefCell::new(convert_from_json(&doc))));
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
        let mut value_rm;
        let value_rrb = match value_rr {
            RValue::Raw(ref v) => v,
            RValue::Ref(ref v_rc) => {
                value_rm = v_rc.borrow();
                &*value_rm
            }
        };
        self.stack.push(RValue::Ref(Rc::new(RefCell::new(Value::String(
            convert_to_json(&value_rrb),
            None,
        )))));

        return 1;
    }
}

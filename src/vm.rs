use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::convert::TryInto;
use std::io::BufRead;
use std::ops::Index;
use std::ops::IndexMut;
use std::rc::Rc;
use std::str;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use indexmap::IndexMap;
use lazy_static::lazy_static;
use sysinfo::{System, SystemExt};

use chunk::{
    print_error, AnonymousFunction, CFPair, Chunk, GeneratorObject, StringPair, Value, ValueSD,
};
use compiler::Compiler;
use opcode::{to_opcode, OpCode};

mod vm_arithmetic;
mod vm_basics;
mod vm_command;
mod vm_hash;
mod vm_io;
mod vm_json;
mod vm_list;
mod vm_print;
mod vm_regex;
mod vm_string;
mod vm_system;
mod vm_xml;

/// For dealing with the EndList opcode, which also supports ending a
/// hash.
pub enum ListType {
    List,
    Hash,
}

/// See Value.to_string.  This function takes the result of a call to
/// that function, and returns a &str.
pub fn to_string_2<'a>(v: &'a (Option<&str>, Option<String>)) -> Option<&'a str> {
    match v {
        (Some(s), _) => Some(s),
        (_, Some(s)) => Some(&s),
        _ => None,
    }
}

/// For running compiled bytecode.
pub struct VM {
    // Whether to print debug information to standard error while
    // running.
    debug: bool,
    // The stack.
    stack: Vec<Value>,
    // Whether the stack should be printed after interpretation has
    // finished.
    print_stack: bool,
    // The local variable stack.
    local_var_stack: Rc<RefCell<Vec<Value>>>,
    // A System object, for getting process information.
    sys: System,
}

lazy_static! {
    static ref SIMPLE_FORMS: HashMap<&'static str, fn(&mut VM, &Chunk, usize) -> i32> = {
        let mut map = HashMap::new();
        map.insert("+", VM::opcode_add as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert(
            "-",
            VM::opcode_subtract as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert(
            "*",
            VM::opcode_multiply as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert("/", VM::opcode_divide as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("=", VM::opcode_eq as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert(">", VM::opcode_gt as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("<", VM::opcode_lt as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert(
            "print",
            VM::opcode_print as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert("drop", VM::opcode_drop as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert(
            "clear",
            VM::opcode_clear as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert("dup", VM::opcode_dup as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("over", VM::opcode_over as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("swap", VM::opcode_swap as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("rot", VM::opcode_rot as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert(
            "depth",
            VM::opcode_depth as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert(
            "is-null",
            VM::opcode_isnull as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert(
            "is-list",
            VM::opcode_islist as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert(
            "is-callable",
            VM::opcode_iscallable as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert(
            "is-shiftable",
            VM::opcode_isshiftable as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert("open", VM::opcode_open as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert(
            "readline",
            VM::opcode_readline as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert(
            "println",
            VM::core_println as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert("rm", VM::core_rm as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert(
            "writeline",
            VM::core_writeline as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert("close", VM::core_close as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert(
            "opendir",
            VM::core_opendir as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert(
            "readdir",
            VM::core_readdir as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert("cp", VM::core_cp as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("mv", VM::core_mv as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("cd", VM::core_cd as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("pwd", VM::core_pwd as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("touch", VM::core_touch as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("stat", VM::core_stat as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("ps", VM::core_ps as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("kill", VM::core_kill as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("m", VM::core_m as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("s", VM::core_s as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("c", VM::core_c as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("nth", VM::core_nth as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("nth!", VM::core_nth_em as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert(
            "append",
            VM::core_append as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert("push", VM::opcode_push as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert(
            "unshift",
            VM::core_unshift as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert("pop", VM::opcode_pop as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("len", VM::core_len as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert(
            "is-dir",
            VM::core_is_dir as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert("split", VM::core_split as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("at", VM::core_at as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("at!", VM::core_at_em as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("keys", VM::core_keys as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert(
            "values",
            VM::core_values as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert("each", VM::core_each as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert(
            "from-json",
            VM::core_from_json as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert(
            "to-json",
            VM::core_to_json as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert(
            "from-xml",
            VM::core_from_xml as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert(
            "to-xml",
            VM::core_to_xml as fn(&mut VM, &Chunk, usize) -> i32,
        );
        map.insert("str", VM::opcode_str as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("int", VM::opcode_int as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("flt", VM::opcode_flt as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("rand", VM::opcode_rand as fn(&mut VM, &Chunk, usize) -> i32);
        map
    };
    static ref SHIFT_FORMS: HashMap<
        &'static str,
        fn(
            &mut VM,
            &mut Vec<RefCell<HashMap<String, Value>>>,
            &mut RefCell<HashMap<String, Chunk>>,
            &mut Vec<Rc<RefCell<Vec<Value>>>>,
            &Chunk,
            usize,
            (u32, u32),
            Arc<AtomicBool>,
        ) -> i32,
    > = {
        let mut map = HashMap::new();
        map.insert(
            "shift",
            VM::opcode_shift
                as fn(
                    &mut VM,
                    &mut Vec<RefCell<HashMap<String, Value>>>,
                    &mut RefCell<HashMap<String, Chunk>>,
                    &mut Vec<Rc<RefCell<Vec<Value>>>>,
                    &Chunk,
                    usize,
                    (u32, u32),
                    Arc<AtomicBool>,
                ) -> i32,
        );
        map.insert(
            "gnth",
            VM::core_gnth
                as fn(
                    &mut VM,
                    &mut Vec<RefCell<HashMap<String, Value>>>,
                    &mut RefCell<HashMap<String, Chunk>>,
                    &mut Vec<Rc<RefCell<Vec<Value>>>>,
                    &Chunk,
                    usize,
                    (u32, u32),
                    Arc<AtomicBool>,
                ) -> i32,
        );
        map.insert(
            "|",
            VM::core_pipe
                as fn(
                    &mut VM,
                    &mut Vec<RefCell<HashMap<String, Value>>>,
                    &mut RefCell<HashMap<String, Chunk>>,
                    &mut Vec<Rc<RefCell<Vec<Value>>>>,
                    &Chunk,
                    usize,
                    (u32, u32),
                    Arc<AtomicBool>,
                ) -> i32,
        );
        map.insert(
            "shift-all",
            VM::core_shift_all
                as fn(
                    &mut VM,
                    &mut Vec<RefCell<HashMap<String, Value>>>,
                    &mut RefCell<HashMap<String, Chunk>>,
                    &mut Vec<Rc<RefCell<Vec<Value>>>>,
                    &Chunk,
                    usize,
                    (u32, u32),
                    Arc<AtomicBool>,
                ) -> i32,
        );
        map.insert(
            "join",
            VM::core_join
                as fn(
                    &mut VM,
                    &mut Vec<RefCell<HashMap<String, Value>>>,
                    &mut RefCell<HashMap<String, Chunk>>,
                    &mut Vec<Rc<RefCell<Vec<Value>>>>,
                    &Chunk,
                    usize,
                    (u32, u32),
                    Arc<AtomicBool>,
                ) -> i32,
        );
        map
    };
    static ref SIMPLE_OPS: Vec<Option<fn(&mut VM, &Chunk, usize) -> i32>> = {
        let mut vec = vec![None; 255];
        vec[OpCode::Add as usize] = Some(VM::opcode_add as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Subtract as usize] =
            Some(VM::opcode_subtract as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Multiply as usize] =
            Some(VM::opcode_multiply as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Divide as usize] = Some(VM::opcode_divide as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Eq as usize] = Some(VM::opcode_eq as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Gt as usize] = Some(VM::opcode_gt as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Lt as usize] = Some(VM::opcode_lt as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Print as usize] = Some(VM::opcode_print as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Drop as usize] = Some(VM::opcode_drop as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Clear as usize] = Some(VM::opcode_clear as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Dup as usize] = Some(VM::opcode_dup as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Over as usize] = Some(VM::opcode_over as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Swap as usize] = Some(VM::opcode_swap as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Rot as usize] = Some(VM::opcode_rot as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Depth as usize] = Some(VM::opcode_depth as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::IsNull as usize] = Some(VM::opcode_isnull as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::DupIsNull as usize] =
            Some(VM::opcode_dupisnull as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::IsList as usize] = Some(VM::opcode_islist as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::IsCallable as usize] =
            Some(VM::opcode_iscallable as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::IsShiftable as usize] =
            Some(VM::opcode_isshiftable as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Open as usize] = Some(VM::opcode_open as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Readline as usize] =
            Some(VM::opcode_readline as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Str as usize] = Some(VM::opcode_str as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Int as usize] = Some(VM::opcode_int as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Flt as usize] = Some(VM::opcode_flt as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Rand as usize] = Some(VM::opcode_rand as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Push as usize] = Some(VM::opcode_push as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Pop as usize] = Some(VM::opcode_pop as fn(&mut VM, &Chunk, usize) -> i32);
        vec
    };
}

impl VM {
    pub fn new(print_stack: bool, debug: bool) -> VM {
        VM {
            debug: debug,
            stack: Vec::new(),
            local_var_stack: Rc::new(RefCell::new(Vec::new())),
            print_stack: print_stack,
            sys: System::new(),
        }
    }

    pub fn call_named_function<'a>(
        &mut self,
        scopes: &mut Vec<RefCell<HashMap<String, Value>>>,
        global_functions: &mut RefCell<HashMap<String, Chunk>>,
        call_stack_chunks: &Vec<&Chunk>,
        chunk: &'a Chunk,
        chunk_values: Rc<RefCell<HashMap<String, Value>>>,
        chunk_functions: Rc<RefCell<Vec<CFPair>>>,
        i: usize,
        gen_global_vars: Option<Rc<RefCell<HashMap<String, Value>>>>,
        gen_local_vars_stack: Option<Rc<RefCell<Vec<Value>>>>,
        prev_local_vars_stacks: &mut Vec<Rc<RefCell<Vec<Value>>>>,
        line_col: (u32, u32),
        running: Arc<AtomicBool>,
        is_value_function: bool,
        plvs_index: u32,
        call_chunk_rc: Rc<RefCell<Chunk>>,
    ) -> bool {
        let call_chunk = call_chunk_rc.borrow();
        let mut new_call_stack_chunks = call_stack_chunks.clone();
        new_call_stack_chunks.push(chunk);
        if call_chunk.is_generator {
            let mut gen_args = Vec::new();
            let req_arg_count = call_chunk.req_arg_count;
            if self.stack.len() < req_arg_count.try_into().unwrap() {
                let err_str = format!(
                    "generator requires {} argument{}",
                    req_arg_count,
                    if req_arg_count > 1 { "s" } else { "" }
                );
                print_error(chunk, i, &err_str);
                return false;
            }
            let mut arg_count = call_chunk.arg_count;
            if arg_count != 0 {
                while arg_count > 0 && self.stack.len() > 0 {
                    gen_args.push(self.stack.pop().unwrap());
                    arg_count = arg_count - 1;
                }
            }
            if gen_args.len() == 0 {
                gen_args.push(Value::Null);
            }
            let mut gen_call_stack_chunks = Vec::new();
            for i in new_call_stack_chunks.iter() {
                gen_call_stack_chunks.push((*i).clone());
            }
            let gen_rr = Value::Generator(Rc::new(RefCell::new(GeneratorObject::new(
                Rc::new(RefCell::new(HashMap::new())),
                Rc::new(RefCell::new(Vec::new())),
                0,
                call_chunk.clone(),
                Rc::new(RefCell::new(gen_call_stack_chunks)),
                gen_args,
                Rc::new(RefCell::new(HashMap::new())),
            ))));
            self.stack.push(gen_rr);
        } else {
            if call_chunk.has_vars {
                scopes.push(RefCell::new(HashMap::new()));
            }

            if is_value_function {
                self.local_var_stack =
                    (*(prev_local_vars_stacks.get(plvs_index as usize).unwrap())).clone();
            } else if call_chunk.nested {
                self.local_var_stack = (*(prev_local_vars_stacks.last().unwrap())).clone();
            }

            let res = self.run(
                scopes,
                global_functions,
                &new_call_stack_chunks,
                &call_chunk,
                chunk_values.clone(),
                chunk_functions,
                0,
                gen_global_vars.clone(),
                gen_local_vars_stack.clone(),
                prev_local_vars_stacks,
                line_col,
                running.clone(),
            );

            if is_value_function {
                prev_local_vars_stacks[plvs_index as usize] = self.local_var_stack.clone();
            } else if call_chunk.nested {
                let plvs_len = prev_local_vars_stacks.len();
                prev_local_vars_stacks[plvs_len - 1] = self.local_var_stack.clone();
            }

            if res == 0 {
                return false;
            }
        }
        return true;
    }

    pub fn call_string<'a>(
        &mut self,
        scopes: &mut Vec<RefCell<HashMap<String, Value>>>,
        global_functions: &mut RefCell<HashMap<String, Chunk>>,
        call_stack_chunks: &Vec<&Chunk>,
        chunk: &'a Chunk,
        chunk_values: Rc<RefCell<HashMap<String, Value>>>,
        chunk_functions: Rc<RefCell<Vec<CFPair>>>,
        i: usize,
        gen_global_vars: Option<Rc<RefCell<HashMap<String, Value>>>>,
        gen_local_vars_stack: Option<Rc<RefCell<Vec<Value>>>>,
        prev_local_vars_stacks: &mut Vec<Rc<RefCell<Vec<Value>>>>,
        line_col: (u32, u32),
        running: Arc<AtomicBool>,
        is_value_function: bool,
        plvs_index: u32,
        is_implicit: bool,
        s: &str,
    ) -> bool {
        let sf_fn_opt = SIMPLE_FORMS.get(s);
        if !sf_fn_opt.is_none() {
            let sf_fn = sf_fn_opt.unwrap();
            let n = sf_fn(self, chunk, i);
            if n == 0 {
                return false;
            }
            return true;
        }

        let shift_fn_opt = SHIFT_FORMS.get(s);
        if !shift_fn_opt.is_none() {
            let shift_fn = shift_fn_opt.unwrap();
            let n = shift_fn(
                self,
                scopes,
                global_functions,
                prev_local_vars_stacks,
                chunk,
                i,
                line_col,
                running,
            );
            if n == 0 {
                return false;
            }
            return true;
        }

        if s == "toggle-mode" {
            self.print_stack = !self.print_stack;
        } else if s == ".s" {
            self.print_stack(chunk, i, scopes, global_functions, running, true);
        } else if s == "exc" {
            if self.stack.len() < 1 {
                print_error(chunk, i, "exc requires one argument");
                return false;
            }
            let fn_rr = self.stack.pop().unwrap();
            let backup_rr = fn_rr.clone();
            let fn_s;
            let fn_b;
            let fn_str;
            let fn_bk: Option<String>;
            let fn_opt: Option<&str> = match fn_rr {
                Value::String(sp) => {
                    fn_s = sp;
                    fn_b = fn_s.borrow();
                    Some(&fn_b.s)
                }
                _ => {
                    fn_bk = fn_rr.to_string();
                    match fn_bk {
                        Some(s) => {
                            fn_str = s;
                            Some(&fn_str)
                        }
                        _ => None,
                    }
                }
            };

            let mut pushed = false;
            match fn_opt {
                Some(s) => {
                    let sf_fn_opt = SIMPLE_FORMS.get(&s as &str);
                    if !sf_fn_opt.is_none() {
                        let sf_fn = sf_fn_opt.unwrap();
                        let nv = Value::CoreFunction(*sf_fn);
                        self.stack.push(nv);
                        pushed = true;
                    } else {
                        let shift_fn_opt = SHIFT_FORMS.get(&s as &str);
                        if !shift_fn_opt.is_none() {
                            let shift_fn = shift_fn_opt.unwrap();
                            let nv = Value::ShiftFunction(*shift_fn);
                            self.stack.push(nv);
                            pushed = true;
                        } else {
                            let mut new_call_stack_chunks = call_stack_chunks.clone();
                            new_call_stack_chunks.push(chunk);

                            let call_stack_function;
                            let global_function;
                            let mut call_chunk_opt = None;

                            for sf in new_call_stack_chunks.iter().rev() {
                                if sf.functions.borrow().contains_key(s) {
                                    call_stack_function = sf.functions.borrow();
                                    call_chunk_opt = Some(call_stack_function.get(s).unwrap());
                                    break;
                                }
                            }
                            if call_chunk_opt.is_none() && global_functions.borrow().contains_key(s)
                            {
                                global_function = global_functions.borrow().get(s).unwrap().clone();
                                call_chunk_opt = Some(&global_function);
                            }
                            match call_chunk_opt {
                                Some(call_chunk) => {
                                    let nv = Value::NamedFunction(Rc::new(RefCell::new(
                                        call_chunk.clone(),
                                    )));
                                    self.stack.push(nv);
                                    pushed = true;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
            if !pushed {
                self.stack.push(backup_rr);
            }
        } else if s == "import" {
            if self.stack.len() < 1 {
                print_error(chunk, i, "import requires one argument");
                return false;
            }

            let lib_rr = self.stack.pop().unwrap();
            let lib_str_s;
            let lib_str_b;
            let lib_str_str;
            let lib_str_bk: Option<String>;
            let lib_str_opt: Option<&str> = match lib_rr {
                Value::String(sp) => {
                    lib_str_s = sp;
                    lib_str_b = lib_str_s.borrow();
                    Some(&lib_str_b.s)
                }
                _ => {
                    lib_str_bk = lib_rr.to_string();
                    match lib_str_bk {
                        Some(s) => {
                            lib_str_str = s;
                            Some(&lib_str_str)
                        }
                        _ => None,
                    }
                }
            };

            match lib_str_opt {
                Some(s) => {
                    let mut compiler = Compiler::new(false);
                    let import_chunk_opt = compiler.deserialise(s);
                    match import_chunk_opt {
                        Some(import_chunk) => {
                            let mut global_functions_b = global_functions.borrow_mut();
                            for (k, v) in import_chunk.functions.borrow().iter() {
                                global_functions_b.insert(k.clone(), v.clone());
                            }
                        }
                        None => {
                            return false;
                        }
                    }
                }
                _ => {
                    print_error(chunk, i, "import argument must be a string");
                    return false;
                }
            }
        } else {
            let mut new_call_stack_chunks = call_stack_chunks.clone();
            new_call_stack_chunks.push(chunk);

            let call_stack_function;
            let global_function;
            let mut call_chunk_opt = None;

            for sf in new_call_stack_chunks.iter().rev() {
                if sf.functions.borrow().contains_key(s) {
                    call_stack_function = sf.functions.borrow();
                    call_chunk_opt = Some(call_stack_function.get(s).unwrap());
                    break;
                }
            }
            if call_chunk_opt.is_none() && global_functions.borrow().contains_key(s) {
                global_function = global_functions.borrow().get(s).unwrap().clone();
                call_chunk_opt = Some(&global_function);
            }
            match call_chunk_opt {
                None => {
                    if is_implicit {
                        let value_rr = Value::String(Rc::new(RefCell::new(StringPair::new(
                            s.to_string(),
                            None,
                        ))));
                        self.stack.push(value_rr);
                    } else {
                        print_error(chunk, i, "function not found");
                        return false;
                    }
                }
                Some(call_chunk) => {
                    let call_chunk_rc = Rc::new(RefCell::new(call_chunk.clone()));
                    return self.call_named_function(
                        scopes,
                        global_functions,
                        call_stack_chunks,
                        chunk,
                        chunk_values.clone(),
                        chunk_functions,
                        0,
                        gen_global_vars.clone(),
                        gen_local_vars_stack.clone(),
                        prev_local_vars_stacks,
                        line_col,
                        running.clone(),
                        is_value_function,
                        plvs_index,
                        call_chunk_rc,
                    );
                }
            }
        }

        return true;
    }

    /// Takes the set of scopes, the global functions, the call stack
    /// chunks, the current chunk, the values for the current chunk,
    /// the instruction index, the opcode for the call that is being
    /// executed, the value for the function being called, the global
    /// variables for the current generator (if applicable), the local
    /// variables for the current generator (if applicable), the
    /// previous local variable stacks, the current line and column
    /// number, and the running flag as its arguments.  Calls the
    /// function (per the value for the function that is being
    /// called).
    pub fn call<'a>(
        &mut self,
        scopes: &mut Vec<RefCell<HashMap<String, Value>>>,
        global_functions: &mut RefCell<HashMap<String, Chunk>>,
        call_stack_chunks: &Vec<&Chunk>,
        chunk: &'a Chunk,
        chunk_values: Rc<RefCell<HashMap<String, Value>>>,
        chunk_functions: Rc<RefCell<Vec<CFPair>>>,
        i: usize,
        call_opcode: OpCode,
        mut function_rr: Option<Value>,
        function_str: Option<&str>,
        function_str_index: i32,
        gen_global_vars: Option<Rc<RefCell<HashMap<String, Value>>>>,
        gen_local_vars_stack: Option<Rc<RefCell<Vec<Value>>>>,
        prev_local_vars_stacks: &mut Vec<Rc<RefCell<Vec<Value>>>>,
        line_col: (u32, u32),
        running: Arc<AtomicBool>,
    ) -> bool {
        if self.debug {
            eprintln!("Chunk functions: {:?}", chunk_functions);
        }
        // Determine whether the function has been called implicitly.
        let is_implicit;
        match call_opcode {
            OpCode::CallImplicit => {
                is_implicit = true;
            }
            OpCode::Call => {
                is_implicit = false;
            }
            OpCode::CallConstant => {
                is_implicit = false;
            }
            _ => {
                eprintln!("unexpected opcode!");
                std::process::abort();
            }
        }

        // If the value being called is a function value, then confirm
        // that the correct local variable stack is still available,
        // and store that value so the right stack is used later on.
        let mut is_value_function = false;
        let mut plvs_index = 0;
        let mut value_function_str = "".to_owned();
        {
            match function_rr {
                Some(Value::Function(ref vf)) => {
                    //s, vf_plvs_index, vf_plvs_ptr) => {
                    let af = vf.borrow();
                    let s = &af.f;
                    let vf_plvs_index = af.local_var_stack_index;
                    let vf_plvs_ptr = af.stack_id;

                    is_value_function = true;
                    plvs_index = vf_plvs_index;
                    value_function_str = s.clone();
                    if (plvs_index + 1) > (prev_local_vars_stacks.len() as u32) {
                        print_error(chunk, i, "cannot call function, as stack has gone away");
                        return false;
                    }
                    let plvs_ptr =
                        prev_local_vars_stacks[plvs_index as usize].as_ptr() as *const _ as u64;
                    if vf_plvs_ptr != plvs_ptr {
                        print_error(chunk, i, "cannot call function, as stack has gone away");
                        return false;
                    }
                }
                _ => {}
            }
        }
        if is_value_function {
            function_rr = Some(Value::String(Rc::new(RefCell::new(StringPair::new(
                value_function_str.to_string(),
                None,
            )))));
        }

        match function_str {
            Some(s) => {
                if function_str_index > -1 {
                    if self.debug {
                        eprintln!("function str is {:?}", s);
                        eprintln!("function str index is {:?}", function_str_index);
                    }
                    /* todo: the two lookups here may be affecting
                     * performance. */
                    let not_present;
                    {
                        let cfb = chunk_functions.borrow();
                        let cv = cfb.get(function_str_index as usize);
                        match cv {
                            Some(CFPair {
                                ffn: Value::Null,
                                cfs: _,
                            })
                            | None => {
                                not_present = true;
                            }
                            _ => {
                                not_present = false;
                            }
                        }
                    }
                    if not_present {
                        let sf_fn_opt = SIMPLE_FORMS.get(&s as &str);
                        if !sf_fn_opt.is_none() {
                            let sf_fn = sf_fn_opt.unwrap();
                            let nv = Value::CoreFunction(*sf_fn);
                            chunk_functions
                                .borrow_mut()
                                .resize(function_str_index as usize, CFPair::new(Value::Null));
                            let cfpair = CFPair::new(nv);
                            chunk_functions
                                .borrow_mut()
                                .insert(function_str_index as usize, cfpair);
                            if self.debug {
                                eprintln!("function str {:?} found, inserted as core function", s);
                            }
                        } else {
                            let shift_fn_opt = SHIFT_FORMS.get(&s as &str);
                            if !shift_fn_opt.is_none() {
                                let shift_fn = shift_fn_opt.unwrap();
                                let nv = Value::ShiftFunction(*shift_fn);
                                chunk_functions
                                    .borrow_mut()
                                    .resize(function_str_index as usize, CFPair::new(Value::Null));
                                let cfpair = CFPair::new(nv);
                                chunk_functions
                                    .borrow_mut()
                                    .insert(function_str_index as usize, cfpair);
                                if self.debug {
                                    eprintln!(
                                        "function str {:?} found, inserted as shift function",
                                        s
                                    );
                                }
                            } else {
                                let mut new_call_stack_chunks = call_stack_chunks.clone();
                                new_call_stack_chunks.push(chunk);

                                let call_stack_function;
                                let global_function;
                                let mut call_chunk_opt = None;

                                for sf in new_call_stack_chunks.iter().rev() {
                                    if sf.functions.borrow().contains_key(s) {
                                        call_stack_function = sf.functions.borrow();
                                        call_chunk_opt = Some(call_stack_function.get(s).unwrap());
                                        break;
                                    }
                                }
                                if call_chunk_opt.is_none()
                                    && global_functions.borrow().contains_key(s)
                                {
                                    global_function =
                                        global_functions.borrow().get(s).unwrap().clone();
                                    call_chunk_opt = Some(&global_function);
                                }
                                match call_chunk_opt {
                                    Some(call_chunk) => {
                                        let call_chunk_rc =
                                            Rc::new(RefCell::new(call_chunk.clone()));
                                        let nv = Value::NamedFunction(call_chunk_rc.clone());
                                        chunk_functions.borrow_mut().resize(
                                            function_str_index as usize,
                                            CFPair::new(Value::Null),
                                        );
                                        let cfpair = CFPair::new(nv);
                                        chunk_functions
                                            .borrow_mut()
                                            .insert(function_str_index as usize, cfpair);
                                        if self.debug {
                                            eprintln!("function str {:?} found, inserted as named function", s);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    {
                        let cfb = chunk_functions.borrow();
                        let cv = cfb.get(function_str_index as usize);
                        match cv {
                            Some(CFPair {
                                ffn: Value::CoreFunction(cf),
                                cfs: _,
                            }) => {
                                if self.debug {
                                    eprintln!("function str {:?} matches core function", s);
                                }
                                let n = cf(self, chunk, i);
                                if n == 0 {
                                    return false;
                                }
                                return true;
                            }
                            Some(CFPair {
                                ffn: Value::ShiftFunction(sf),
                                cfs: _,
                            }) => {
                                if self.debug {
                                    eprintln!("function str {:?} matches shift function", s);
                                }
                                let n = sf(
                                    self,
                                    scopes,
                                    global_functions,
                                    prev_local_vars_stacks,
                                    chunk,
                                    i,
                                    line_col,
                                    running,
                                );
                                if n == 0 {
                                    return false;
                                }
                                return true;
                            }
                            Some(CFPair {
                                ffn: Value::NamedFunction(call_chunk_rc),
                                cfs,
                            }) => {
                                if self.debug {
                                    eprintln!("function str {:?} matches named function", s);
                                }
                                return self.call_named_function(
                                    scopes,
                                    global_functions,
                                    call_stack_chunks,
                                    chunk,
                                    chunk_values.clone(),
                                    cfs.clone(),
                                    0,
                                    gen_global_vars.clone(),
                                    gen_local_vars_stack.clone(),
                                    prev_local_vars_stacks,
                                    line_col,
                                    running.clone(),
                                    is_value_function,
                                    plvs_index,
                                    call_chunk_rc.clone(),
                                );
                            }
                            Some(CFPair {
                                ffn: Value::Null,
                                cfs: _,
                            }) => {
                                if self.debug {
                                    eprintln!("function str {:?} not cached", s);
                                }
                            }
                            Some(s) => {
                                if self.debug {
                                    eprintln!("unexpected cached function!");
                                    eprintln!("{:?}", s);
                                }
                                std::process::abort();
                            }
                            None => {
                                if self.debug {
                                    eprintln!("function str {:?} not cached", s);
                                }
                            }
                        };
                    }
                }
                if self.debug {
                    eprintln!("instantiating new chunk functions for string {:?}", s);
                }
                return self.call_string(
                    scopes,
                    global_functions,
                    call_stack_chunks,
                    chunk,
                    chunk_values.clone(),
                    Rc::new(RefCell::new(Vec::new())),
                    0,
                    gen_global_vars.clone(),
                    gen_local_vars_stack.clone(),
                    prev_local_vars_stacks,
                    line_col,
                    running.clone(),
                    is_value_function,
                    plvs_index,
                    is_implicit,
                    s,
                );
            }
            _ => {}
        }

        let frr = function_rr.unwrap();

        match frr {
            Value::Command(s) => {
                let i2 = self.core_command(&s.borrow(), chunk, i);
                if i2 == 0 {
                    return false;
                }
            }
            Value::CommandUncaptured(s) => {
                let i2 = self.core_command_uncaptured(&s.borrow(), chunk, i);
                if i2 == 0 {
                    return false;
                }
            }
            Value::CoreFunction(cf) => {
                let n = cf(self, chunk, i);
                if n == 0 {
                    return false;
                }
                return true;
            }
            Value::ShiftFunction(sf) => {
                let n = sf(
                    self,
                    scopes,
                    global_functions,
                    prev_local_vars_stacks,
                    chunk,
                    i,
                    line_col,
                    running,
                );
                if n == 0 {
                    return false;
                }
                return true;
            }
            Value::NamedFunction(call_chunk_rc) => {
                if self.debug {
                    eprintln!("instantiating new chunk functions for NamedFunction");
                }
                return self.call_named_function(
                    scopes,
                    global_functions,
                    call_stack_chunks,
                    chunk,
                    chunk_values.clone(),
                    Rc::new(RefCell::new(Vec::new())),
                    0,
                    gen_global_vars.clone(),
                    gen_local_vars_stack.clone(),
                    prev_local_vars_stacks,
                    line_col,
                    running.clone(),
                    is_value_function,
                    plvs_index,
                    call_chunk_rc,
                );
            }
            Value::String(sp) => {
                let s = &sp.borrow().s;
                if self.debug {
                    eprintln!("instantiating new chunk functions for string: {}", s);
                }
                return self.call_string(
                    scopes,
                    global_functions,
                    call_stack_chunks,
                    chunk,
                    chunk_values.clone(),
                    Rc::new(RefCell::new(Vec::new())),
                    0,
                    gen_global_vars.clone(),
                    gen_local_vars_stack.clone(),
                    prev_local_vars_stacks,
                    line_col,
                    running.clone(),
                    is_value_function,
                    plvs_index,
                    is_implicit,
                    &s,
                );
            }
            _ => {
                if is_implicit {
                    self.stack.push(frr.clone());
                } else {
                    print_error(chunk, i, "function not found");
                    return false;
                }
            }
        }

        return true;
    }

    /// Takes the set of scopes, the global functions, the call stack
    /// chunks, the current chunk, the values for the current chunk,
    /// the instruction index, the global variables for the current
    /// generator (if applicable), the local variables for the current
    /// generator (if applicable), the previous local variable stacks,
    /// the current line and column number, and the running flag as
    /// its arguments.  Runs the code from the chunk, beginning at the
    /// specified instruction index.
    pub fn run<'a>(
        &mut self,
        scopes: &mut Vec<RefCell<HashMap<String, Value>>>,
        global_functions: &mut RefCell<HashMap<String, Chunk>>,
        call_stack_chunks: &Vec<&Chunk>,
        chunk: &'a Chunk,
        chunk_values: Rc<RefCell<HashMap<String, Value>>>,
        chunk_functions: Rc<RefCell<Vec<CFPair>>>,
        index: usize,
        mut gen_global_vars: Option<Rc<RefCell<HashMap<String, Value>>>>,
        mut gen_local_vars_stack: Option<Rc<RefCell<Vec<Value>>>>,
        prev_local_vars_stacks: &mut Vec<Rc<RefCell<Vec<Value>>>>,
        line_col: (u32, u32),
        running: Arc<AtomicBool>,
    ) -> usize {
        let mut i = index;
        let data = chunk.data.borrow();

        let mut list_index_opt = None;
        let mut list_indexes = Vec::new();
        let mut list_types = Vec::new();

        while i < data.len() {
            if !running.load(Ordering::SeqCst) {
                running.store(true, Ordering::SeqCst);
                self.stack.clear();
                return 0;
            }
            let op = to_opcode(data[i]);
            if self.debug {
                eprintln!(">  Opcode: {:?}", op);
                eprintln!(" > Stack:  {:?}", self.stack);
                eprintln!(" > Index:  {:?}", i);
            }
            let op_fn_opt = SIMPLE_OPS[op as usize];
            if !op_fn_opt.is_none() {
                let op_fn = op_fn_opt.unwrap();
                let res = op_fn(self, chunk, i);
                if res == 0 {
                    return 0;
                } else {
                    i = i + 1;
                    continue;
                }
            }
            match op {
                OpCode::AddConstant => {
                    i = i + 1;
                    let i_upper = data[i];
                    i = i + 1;
                    let i_lower = data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let mut done = false;
                    if chunk.has_constant_int(i2 as i32) {
                        let n = chunk.get_constant_int(i2 as i32);
                        let len = self.stack.len();
                        let v1_rr = self.stack.get_mut(len - 1).unwrap();
                        match v1_rr {
                            Value::Int(ref mut n1) => {
                                *n1 = *n1 + n;
                                done = true;
                            }
                            _ => {}
                        };
                    }
                    if !done {
                        let op_fn_opt = SIMPLE_OPS[OpCode::Add as usize];
                        self.stack.push(chunk.get_constant(i2 as i32));
                        let op_fn = op_fn_opt.unwrap();
                        let res = op_fn(self, chunk, i);
                        if res == 0 {
                            return 0;
                        } else {
                            i = i + 1;
                            continue;
                        }
                    }
                }
                OpCode::SubtractConstant => {
                    i = i + 1;
                    let i_upper = data[i];
                    i = i + 1;
                    let i_lower = data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let mut done = false;
                    if chunk.has_constant_int(i2 as i32) {
                        let n = chunk.get_constant_int(i2 as i32);
                        let len = self.stack.len();
                        let v1_rr = self.stack.get_mut(len - 1).unwrap();
                        match v1_rr {
                            Value::Int(ref mut n1) => {
                                *n1 = *n1 - n;
                                done = true;
                            }
                            _ => {}
                        };
                    }
                    if !done {
                        let op_fn_opt = SIMPLE_OPS[OpCode::Subtract as usize];
                        self.stack.push(chunk.get_constant(i2 as i32));
                        let op_fn = op_fn_opt.unwrap();
                        let res = op_fn(self, chunk, i);
                        if res == 0 {
                            return 0;
                        } else {
                            i = i + 1;
                            continue;
                        }
                    }
                }
                OpCode::MultiplyConstant => {
                    i = i + 1;
                    let i_upper = data[i];
                    i = i + 1;
                    let i_lower = data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let mut done = false;
                    if chunk.has_constant_int(i2 as i32) {
                        let n = chunk.get_constant_int(i2 as i32);
                        let len = self.stack.len();
                        let v1_rr = self.stack.get_mut(len - 1).unwrap();
                        match v1_rr {
                            Value::Int(ref mut n1) => {
                                *n1 = *n1 * n;
                                done = true;
                            }
                            _ => {}
                        };
                    }
                    if !done {
                        let op_fn_opt = SIMPLE_OPS[OpCode::Multiply as usize];
                        self.stack.push(chunk.get_constant(i2 as i32));
                        let op_fn = op_fn_opt.unwrap();
                        let res = op_fn(self, chunk, i);
                        if res == 0 {
                            return 0;
                        } else {
                            i = i + 1;
                            continue;
                        }
                    }
                }
                OpCode::DivideConstant => {
                    i = i + 1;
                    let i_upper = data[i];
                    i = i + 1;
                    let i_lower = data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let mut done = false;
                    if chunk.has_constant_int(i2 as i32) {
                        let n = chunk.get_constant_int(i2 as i32);
                        let len = self.stack.len();
                        let v1_rr = self.stack.get_mut(len - 1).unwrap();
                        match v1_rr {
                            Value::Int(ref mut n1) => {
                                *n1 = *n1 / n;
                                done = true;
                            }
                            _ => {}
                        };
                    }
                    if !done {
                        let op_fn_opt = SIMPLE_OPS[OpCode::Divide as usize];
                        self.stack.push(chunk.get_constant(i2 as i32));
                        let op_fn = op_fn_opt.unwrap();
                        let res = op_fn(self, chunk, i);
                        if res == 0 {
                            return 0;
                        } else {
                            i = i + 1;
                            continue;
                        }
                    }
                }
                OpCode::EqConstant => {
                    i = i + 1;
                    let i_upper = data[i];
                    i = i + 1;
                    let i_lower = data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let n = chunk.get_constant_int(i2 as i32);

                    let len = self.stack.len();
                    let v1_rr = self.stack.get_mut(len - 1).unwrap();
                    match v1_rr {
                        Value::Int(ref mut n1) => {
                            if *n1 == n {
                                *n1 = 1;
                            } else {
                                *n1 = 0;
                            }
                        }
                        _ => {}
                    };
                }
                OpCode::StartList => {
                    match list_index_opt {
                        Some(list_index) => {
                            list_indexes.push(list_index);
                        }
                        _ => {}
                    }
                    list_index_opt = Some(self.stack.len());
                    list_types.push(ListType::List);
                }
                OpCode::StartHash => {
                    match list_index_opt {
                        Some(list_index) => {
                            list_indexes.push(list_index);
                        }
                        _ => {}
                    }
                    list_index_opt = Some(self.stack.len());
                    list_types.push(ListType::Hash);
                }
                OpCode::EndList => match list_index_opt {
                    Some(list_index) => {
                        let list_type = list_types.pop().unwrap();
                        match list_type {
                            ListType::List => {
                                let mut lst = VecDeque::new();
                                while self.stack.len() > list_index {
                                    lst.push_front(self.stack.pop().unwrap());
                                }
                                if list_indexes.len() > 0 {
                                    list_index_opt = Some(list_indexes.pop().unwrap());
                                } else {
                                    list_index_opt = None;
                                }
                                self.stack.push(Value::List(Rc::new(RefCell::new(lst))));
                            }
                            ListType::Hash => {
                                let mut map = IndexMap::new();
                                while self.stack.len() > list_index {
                                    let value_rr = self.stack.pop().unwrap();
                                    let key_rr = self.stack.pop().unwrap();
                                    let key_str_s;
                                    let key_str_b;
                                    let key_str_str;
                                    let key_str_bk: Option<String>;
                                    let key_str_opt: Option<&str> = match key_rr {
                                        Value::String(sp) => {
                                            key_str_s = sp;
                                            key_str_b = key_str_s.borrow();
                                            Some(&key_str_b.s)
                                        }
                                        _ => {
                                            key_str_bk = key_rr.to_string();
                                            match key_str_bk {
                                                Some(s) => {
                                                    key_str_str = s;
                                                    Some(&key_str_str)
                                                }
                                                _ => None,
                                            }
                                        }
                                    };
                                    map.insert(key_str_opt.unwrap().to_string(), value_rr);
                                }
                                if list_indexes.len() > 0 {
                                    list_index_opt = Some(list_indexes.pop().unwrap());
                                } else {
                                    list_index_opt = None;
                                }
                                self.stack.push(Value::Hash(Rc::new(RefCell::new(map))));
                            }
                        }
                    }
                    None => {
                        print_error(chunk, i, "no start list found");
                        return 0;
                    }
                },
                OpCode::Function => {
                    // todo: The logic here is awkward, and needs
                    // reviewing.
                    i = i + 1;
                    let i_upper = data[i];
                    i = i + 1;
                    let i_lower = data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let value_rr = chunk.get_constant(i2 as i32);
                    let mut copy = false;
                    let mut fn_name = "".to_owned();
                    // The index is the length, because when the
                    // function is called, you want it to go to the
                    // current local_var_stack (which will have been
                    // pushed onto plvs if this function has been
                    // called).
                    let plvs_index = prev_local_vars_stacks.len();
                    let plvs_ptr = self.local_var_stack.as_ptr() as *const _ as u64;
                    {
                        match value_rr {
                            Value::String(ref sp) => {
                                let s = &sp.borrow().s;
                                match chunk_values.borrow().get(s) {
                                    Some(cv_value_rr) => match cv_value_rr {
                                        Value::String(_) => {
                                            self.stack.push(Value::Function(Rc::new(
                                                RefCell::new(AnonymousFunction::new(
                                                    s.to_string(),
                                                    plvs_index as u32,
                                                    plvs_ptr,
                                                )),
                                            )));
                                        }
                                        _ => {
                                            eprintln!("unexpected function value!");
                                            std::process::abort();
                                        }
                                    },
                                    _ => {
                                        fn_name = s.clone();
                                        copy = true;
                                    }
                                }
                            }
                            _ => {
                                eprintln!("unexpected function value!");
                                std::process::abort();
                            }
                        }
                    }
                    if copy {
                        chunk_values
                            .borrow_mut()
                            .insert(fn_name.clone().to_string(), value_rr);
                        let cv_value_rr = chunk_values.borrow().get(&fn_name).unwrap().clone();
                        match cv_value_rr {
                            Value::String(sp) => {
                                self.stack.push(Value::Function(Rc::new(RefCell::new(
                                    AnonymousFunction::new(
                                        sp.borrow().s.to_string(),
                                        plvs_index as u32,
                                        plvs_ptr,
                                    ),
                                ))));
                            }
                            _ => {
                                eprintln!("unexpected function value!");
                                std::process::abort();
                            }
                        }
                    }
                }
                OpCode::Constant => {
                    i = i + 1;
                    let i_upper = data[i];
                    i = i + 1;
                    let i_lower = data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let value_rr = chunk.get_constant(i2 as i32);
                    let mut copy = false;
                    let mut name = "".to_owned();
                    {
                        match value_rr {
                            Value::String(ref sp) => {
                                match chunk_values.borrow().get(&sp.borrow().s) {
                                    Some(cv_value_rr) => {
                                        self.stack.push(cv_value_rr.clone());
                                    }
                                    _ => {
                                        name = sp.borrow().s.clone();
                                        copy = true;
                                    }
                                }
                            }
                            _ => {
                                self.stack.push(value_rr.clone());
                            }
                        }
                    }
                    if copy {
                        chunk_values
                            .borrow_mut()
                            .insert(name.clone().to_string(), value_rr);
                        self.stack
                            .push(chunk_values.borrow().get(&name).unwrap().clone());
                    }
                }
                OpCode::CallConstant => {
                    prev_local_vars_stacks.push(self.local_var_stack.clone());
                    self.local_var_stack = Rc::new(RefCell::new(vec![]));

                    i = i + 1;
                    let i_upper = data[i];
                    i = i + 1;
                    let i_lower = data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);

                    let (mut line, mut col) = line_col;
                    if line == 0 && col == 0 {
                        let point = chunk.get_point(i);
                        match point {
                            Some((point_line, point_col)) => {
                                line = point_line;
                                col = point_col;
                            }
                            _ => {
                                line = 1;
                                col = 1;
                            }
                        }
                    }

                    let value_sd = &chunk.constants[i2 as usize];
                    match value_sd {
                        ValueSD::String(ref sp) => {
                            let res = self.call(
                                scopes,
                                global_functions,
                                call_stack_chunks,
                                chunk,
                                chunk_values.clone(),
                                chunk_functions.clone(),
                                i,
                                op,
                                None,
                                Some(sp),
                                (i2 as u32).try_into().unwrap(),
                                None,
                                None,
                                prev_local_vars_stacks,
                                (line, col),
                                running.clone(),
                            );

                            self.local_var_stack = prev_local_vars_stacks.pop().unwrap();

                            if !res {
                                return 0;
                            }
                        }
                        _ => {
                            eprintln!("expected string for callconstant!");
                            std::process::abort();
                        }
                    };
                }
                OpCode::Call | OpCode::CallImplicit => {
                    if self.stack.len() < 1 {
                        print_error(chunk, i, "call requires one argument");
                        return 0;
                    }

                    prev_local_vars_stacks.push(self.local_var_stack.clone());
                    self.local_var_stack = Rc::new(RefCell::new(vec![]));

                    let function_rr = self.stack.pop().unwrap();

                    let (mut line, mut col) = line_col;
                    if line == 0 && col == 0 {
                        let point = chunk.get_point(i);
                        match point {
                            Some((point_line, point_col)) => {
                                line = point_line;
                                col = point_col;
                            }
                            _ => {
                                line = 1;
                                col = 1;
                            }
                        }
                    }

                    let res = self.call(
                        scopes,
                        global_functions,
                        call_stack_chunks,
                        chunk,
                        chunk_values.clone(),
                        chunk_functions.clone(),
                        i,
                        op,
                        Some(function_rr),
                        None,
                        -1,
                        None,
                        None,
                        prev_local_vars_stacks,
                        (line, col),
                        running.clone(),
                    );

                    self.local_var_stack = prev_local_vars_stacks.pop().unwrap();

                    if !res {
                        return 0;
                    }
                }
                OpCode::SetLocalVar => {
                    if self.stack.len() < 1 {
                        print_error(chunk, i, "! requires one argument");
                        return 0;
                    }

                    i = i + 1;
                    let var_index: u8 = data[i].try_into().unwrap();
                    let value_rr = self.stack.pop().unwrap();

                    match gen_local_vars_stack {
                        Some(ref mut glvs) => {
                            if var_index == (glvs.borrow().len() as u8) {
                                glvs.borrow_mut().push(value_rr);
                            } else {
                                glvs.borrow_mut()[var_index as usize] = value_rr;
                            }
                        }
                        _ => {
                            if var_index == (self.local_var_stack.borrow().len() as u8) {
                                self.local_var_stack.borrow_mut().push(value_rr);
                            } else {
                                let lvs_b = &mut self.local_var_stack.borrow_mut();
                                let existing_value_rr_ptr = lvs_b.index_mut(var_index as usize);
                                *existing_value_rr_ptr = value_rr;
                            }
                        }
                    }
                }
                OpCode::GetLocalVar => {
                    i = i + 1;
                    let var_index: u8 = data[i].try_into().unwrap();

                    match gen_local_vars_stack {
                        Some(ref mut glvs) => {
                            let value_rr = glvs.borrow()[var_index as usize].clone();
                            self.stack.push(value_rr);
                        }
                        _ => {
                            let value_rr = self
                                .local_var_stack
                                .borrow()
                                .index(var_index as usize)
                                .clone();
                            self.stack.push(value_rr);
                        }
                    }
                }
                OpCode::PopLocalVar => match gen_local_vars_stack {
                    Some(ref mut glvs) => {
                        glvs.borrow_mut().pop();
                    }
                    _ => {
                        self.local_var_stack.borrow_mut().pop();
                    }
                },
                OpCode::Var => {
                    if self.stack.len() < 1 {
                        print_error(chunk, i, "var requires one argument");
                        return 0;
                    }

                    let var_name;
                    {
                        let var_name_rr = self.stack.pop().unwrap();
                        match var_name_rr {
                            Value::String(sp) => {
                                var_name = sp.borrow().s.clone().to_string();
                            }
                            _ => {
                                print_error(chunk, i, "variable name must be a string");
                                return 0;
                            }
                        }
                    }

                    match gen_global_vars {
                        Some(ref mut ggv) => {
                            ggv.borrow_mut().insert(var_name.to_string(), Value::Int(0));
                        }
                        _ => {
                            scopes
                                .last()
                                .unwrap()
                                .borrow_mut()
                                .insert(var_name.to_string(), Value::Int(0));
                        }
                    }
                }
                OpCode::SetVar => {
                    if self.stack.len() < 2 {
                        print_error(chunk, i, "! requires two arguments");
                        return 0;
                    }

                    let var_name_rr = self.stack.pop().unwrap();
                    let value_rr = self.stack.pop().unwrap();

                    match var_name_rr {
                        Value::String(sp) => {
                            let mut done = false;
                            let s = &sp.borrow().s;

                            match gen_global_vars {
                                Some(ref mut ggv) => {
                                    if ggv.borrow().contains_key(s) {
                                        ggv.borrow_mut().insert(s.clone(), value_rr.clone());
                                        done = true;
                                    }
                                }
                                _ => {}
                            }

                            if !done {
                                for scope in scopes.iter().rev() {
                                    if scope.borrow().contains_key(s) {
                                        scope.borrow_mut().insert(s.to_string(), value_rr.clone());
                                        done = true;
                                        break;
                                    }
                                }
                            }

                            if !done {
                                print_error(chunk, i, "could not find variable");
                                return 0;
                            }
                        }
                        _ => {
                            print_error(chunk, i, "variable name must be a string");
                            return 0;
                        }
                    }
                }
                OpCode::GetVar => {
                    if self.stack.len() < 1 {
                        print_error(chunk, i, "@ requires one argument");
                        return 0;
                    }

                    let var_name_rr = self.stack.pop().unwrap();
                    match var_name_rr {
                        Value::String(sp) => {
                            let mut done = false;
                            let s = &sp.borrow().s;

                            match gen_global_vars {
                                Some(ref mut ggv) => {
                                    if ggv.borrow().contains_key(s) {
                                        self.stack.push(ggv.borrow().get(s).unwrap().clone());
                                        done = true;
                                    }
                                }
                                _ => {}
                            }

                            if !done {
                                for scope in scopes.iter().rev() {
                                    if scope.borrow().contains_key(s) {
                                        self.stack.push(scope.borrow().get(s).unwrap().clone());
                                        done = true;
                                        break;
                                    }
                                }
                            }
                            if !done {
                                print_error(chunk, i, "could not find variable");
                                return 0;
                            }
                        }
                        _ => {
                            print_error(chunk, i, "variable name must be a string");
                            return 0;
                        }
                    }
                }
                OpCode::JumpNe => {
                    if self.stack.len() < 1 {
                        print_error(chunk, i, "jumpne requires one argument");
                        return 0;
                    }

                    let value_rr = self.stack.pop().unwrap();

                    i = i + 1;
                    let i1: usize = data[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = data[i].try_into().unwrap();
                    let jmp_len: usize = (i1 << 8) | i2;

                    match value_rr {
                        Value::String(sp) => {
                            if sp.borrow().s == "" {
                                i = i + jmp_len;
                            }
                        }
                        Value::Int(0) => {
                            i = i + jmp_len;
                        }
                        Value::Float(nf) => {
                            if nf == 0.0 {
                                i = i + jmp_len;
                            }
                        }
                        _ => {}
                    }
                }
                OpCode::JumpNeR => {
                    if self.stack.len() < 1 {
                        print_error(chunk, i, "jumpner requires one argument");
                        return 0;
                    }

                    let value_rr = self.stack.pop().unwrap();

                    i = i + 1;
                    let i1: usize = data[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = data[i].try_into().unwrap();
                    let jmp_len: usize = (i1 << 8) | i2;

                    match value_rr {
                        Value::String(sp) => {
                            if sp.borrow().s == "" {
                                i = i - jmp_len;
                            }
                        }
                        Value::Int(0) => {
                            i = i - jmp_len;
                        }
                        Value::Float(nf) => {
                            if nf == 0.0 {
                                i = i - jmp_len;
                            }
                        }
                        _ => {}
                    }
                }
                OpCode::JumpNeREqC => {
                    if self.stack.len() < 1 {
                        print_error(chunk, i, "jumpnereqc requires one argument");
                        return 0;
                    }

                    i = i + 1;
                    let i1: usize = data[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = data[i].try_into().unwrap();
                    let jmp_len: usize = (i1 << 8) | i2;

                    i = i + 1;
                    let i_upper = data[i];
                    i = i + 1;
                    let i_lower = data[i];
                    let i3 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let value_rr = self.stack.last().unwrap();
                    if chunk.has_constant_int(i3 as i32) {
                        let cmp_rr = chunk.get_constant_int(i3 as i32);

                        match &*value_rr {
                            Value::Int(n2) => {
                                if cmp_rr != *n2 {
                                    i = i - jmp_len;
                                };
                            }
                            _ => {
                                eprintln!("unexpected jumpnereqc value!");
                                std::process::abort();
                            }
                        }
                    } else {
                        eprintln!("unexpected jumpnereqc constant!");
                        std::process::abort();
                    }
                }
                OpCode::Shift => {
                    let i2 = self.opcode_shift(
                        scopes,
                        global_functions,
                        prev_local_vars_stacks,
                        chunk,
                        i,
                        line_col,
                        running.clone(),
                    );
                    if i2 == 0 {
                        return 0;
                    }
                }
                OpCode::Yield => {
                    match gen_global_vars {
                        None => {
                            eprintln!("yield without generator");
                        }
                        _ => {}
                    }
                    return i + 1;
                }
                OpCode::Jump => {
                    i = i + 1;
                    let i1: usize = data[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = data[i].try_into().unwrap();
                    let jmp_len: usize = (i1 << 8) | i2;
                    i = i + jmp_len;
                }
                OpCode::JumpR => {
                    i = i + 1;
                    let i1: usize = data[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = data[i].try_into().unwrap();
                    let jmp_len: usize = (i1 << 8) | i2;
                    i = i - jmp_len;
                }
                OpCode::Error => {
                    if self.stack.len() < 1 {
                        print_error(chunk, i, "error requires one argument");
                        return 0;
                    }

                    let (mut line, mut col) = line_col;
                    if line == 0 && col == 0 {
                        let point = chunk.get_point(i);
                        match point {
                            Some((point_line, point_col)) => {
                                line = point_line;
                                col = point_col;
                            }
                            _ => {
                                line = 1;
                                col = 1;
                            }
                        }
                    }

                    let error_rr = self.stack.pop().unwrap();

                    let error_str_s;
                    let error_str_b;
                    let error_str_str;
                    let error_str_bk: Option<String>;
                    let error_str_opt: Option<&str> = match error_rr {
                        Value::String(sp) => {
                            error_str_s = sp;
                            error_str_b = error_str_s.borrow();
                            Some(&error_str_b.s)
                        }
                        _ => {
                            error_str_bk = error_rr.to_string();
                            match error_str_bk {
                                Some(s) => {
                                    error_str_str = s;
                                    Some(&error_str_str)
                                }
                                _ => None,
                            }
                        }
                    };

                    match error_str_opt {
                        Some(s) => {
                            let err_str = format!("{}:{}: {}", line, col, s);
                            eprintln!("{}", err_str);
                            return 0;
                        }
                        None => {
                            let err_str = format!("{}:{}: {}", line, col, "(unknown error)");
                            eprintln!("{}", err_str);
                            return 0;
                        }
                    }
                }
                OpCode::EndFn => {
                    if !chunk.is_generator && chunk.has_vars {
                        scopes.pop();
                    }
                    return i + 1;
                }
                OpCode::Return => {
                    if !chunk.is_generator && chunk.has_vars {
                        scopes.pop();
                    }
                    return i + 1;
                }
                _ => {
                    eprintln!("unknown opcode in bytecode! {:?}", op);
                    std::process::abort();
                }
            }
            i = i + 1;
        }

        match list_index_opt {
            Some(_) => {
                print_error(chunk, i, "unterminated list start");
                return 0;
            }
            _ => {}
        }

        if self.print_stack {
            self.print_stack(chunk, i, scopes, global_functions, running, false);
            self.stack.clear();
        }

        return i + 1;
    }

    /// Takes the global functions, the global variables, the file to
    /// read the program code from, and the running flag as its
    /// arguments.  Compiles the program code and executes it,
    /// returning the chunk (if compiled successfully), the updated
    /// set of global variables, and the updated set of global
    /// functions.
    pub fn interpret(
        &mut self,
        global_functions: HashMap<String, Chunk>,
        variables: HashMap<String, Value>,
        fh: &mut Box<dyn BufRead>,
        running: Arc<AtomicBool>,
        name: &str,
    ) -> (
        Option<Chunk>,
        HashMap<String, Value>,
        Vec<RefCell<HashMap<String, Chunk>>>,
    ) {
        let mut compiler = Compiler::new(self.debug);
        let chunk_opt = compiler.compile(fh, name);
        match chunk_opt {
            None => return (None, HashMap::new(), Vec::new()),
            _ => {}
        }
        let chunk = chunk_opt.unwrap();
        let mut global_functions_rr = RefCell::new(global_functions);
        let call_stack_chunks = vec![];
        let mut scopes = vec![RefCell::new(variables)];
        let chunk_values = Rc::new(RefCell::new(HashMap::new()));
        let chunk_functions = Rc::new(RefCell::new(Vec::new()));
        let mut prev_local_vars_stacks = vec![];

        self.run(
            &mut scopes,
            &mut global_functions_rr,
            &call_stack_chunks,
            &chunk,
            chunk_values,
            chunk_functions,
            0,
            None,
            None,
            &mut prev_local_vars_stacks,
            (0, 0),
            running.clone(),
        );
        if self.print_stack {
            self.stack.clear();
        }
        let updated_variables = match scopes.first() {
            Some(scope) => scope.borrow().clone(),
            _ => HashMap::new(),
        };
        return (Some(chunk), updated_variables, vec![global_functions_rr]);
    }
}

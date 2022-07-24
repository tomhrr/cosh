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
    print_error, Chunk, GeneratorObject, StringPair, Value, ValueSD,
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
    // The scopes.
    scopes: Vec<Rc<RefCell<HashMap<String, Value>>>>,
    // The global functions.
    global_functions: HashMap<String, Rc<RefCell<Chunk>>>,
    // The call stack chunks.
    pub call_stack_chunks: Vec<Rc<RefCell<Chunk>>>,
    // A flag for interrupting execution.
    pub running: Arc<AtomicBool>,
    // A System object, for getting process information.
    sys: System,
}

lazy_static! {
    static ref SIMPLE_FORMS: HashMap<&'static str, fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32> = {
        let mut map = HashMap::new();
        map.insert("+", VM::opcode_add as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert(
            "-",
            VM::opcode_subtract as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert(
            "*",
            VM::opcode_multiply as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert("/", VM::opcode_divide as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("=", VM::opcode_eq as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert(">", VM::opcode_gt as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("<", VM::opcode_lt as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert(
            "print",
            VM::opcode_print as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert("drop", VM::opcode_drop as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert(
            "clear",
            VM::opcode_clear as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert("dup", VM::opcode_dup as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("over", VM::opcode_over as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("swap", VM::opcode_swap as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("rot", VM::opcode_rot as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert(
            "depth",
            VM::opcode_depth as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert(
            "is-null",
            VM::opcode_isnull as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert(
            "is-list",
            VM::opcode_islist as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert(
            "is-callable",
            VM::opcode_iscallable as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert(
            "is-shiftable",
            VM::opcode_isshiftable as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert("open", VM::opcode_open as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert(
            "readline",
            VM::opcode_readline as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert(
            "println",
            VM::core_println as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert("rm", VM::core_rm as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert(
            "writeline",
            VM::core_writeline as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert("close", VM::core_close as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert(
            "opendir",
            VM::core_opendir as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert(
            "readdir",
            VM::core_readdir as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert("cp", VM::core_cp as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("mv", VM::core_mv as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("cd", VM::core_cd as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("pwd", VM::core_pwd as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("touch", VM::core_touch as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("stat", VM::core_stat as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("ps", VM::core_ps as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("kill", VM::core_kill as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("m", VM::core_m as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("s", VM::core_s as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("c", VM::core_c as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("nth", VM::core_nth as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("nth!", VM::core_nth_em as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert(
            "append",
            VM::core_append as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert("push", VM::opcode_push as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert(
            "unshift",
            VM::core_unshift as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert("pop", VM::opcode_pop as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("len", VM::core_len as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert(
            "is-dir",
            VM::core_is_dir as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert("split", VM::core_split as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("splitr", VM::core_splitr as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("at", VM::core_at as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("at!", VM::core_at_em as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("keys", VM::core_keys as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert(
            "values",
            VM::core_values as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert("each", VM::core_each as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert(
            "from-json",
            VM::core_from_json as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert(
            "to-json",
            VM::core_to_json as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert(
            "from-xml",
            VM::core_from_xml as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert(
            "to-xml",
            VM::core_to_xml as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32,
        );
        map.insert("str", VM::opcode_str as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("int", VM::opcode_int as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("flt", VM::opcode_flt as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map.insert("rand", VM::opcode_rand as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        map
    };
    static ref SHIFT_FORMS: HashMap<
        &'static str,
        fn(
            &mut VM,
            Rc<RefCell<Chunk>>,
            usize,
            (u32, u32),
        ) -> i32,
    > = {
        let mut map = HashMap::new();
        map.insert(
            "shift",
            VM::opcode_shift
                as fn(
                    &mut VM,
                    Rc<RefCell<Chunk>>,
                    usize,
                    (u32, u32),
                ) -> i32,
        );
        map.insert(
            "gnth",
            VM::core_gnth
                as fn(
                    &mut VM,
                    Rc<RefCell<Chunk>>,
                    usize,
                    (u32, u32),
                ) -> i32,
        );
        map.insert(
            "|",
            VM::core_pipe
                as fn(
                    &mut VM,
                    Rc<RefCell<Chunk>>,
                    usize,
                    (u32, u32),
                ) -> i32,
        );
        map.insert(
            "shift-all",
            VM::core_shift_all
                as fn(
                    &mut VM,
                    Rc<RefCell<Chunk>>,
                    usize,
                    (u32, u32),
                ) -> i32,
        );
        map.insert(
            "join",
            VM::core_join
                as fn(
                    &mut VM,
                    Rc<RefCell<Chunk>>,
                    usize,
                    (u32, u32),
                ) -> i32,
        );
        map
    };
    static ref SIMPLE_OPS: Vec<Option<fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32>> = {
        let mut vec = vec![None; 255];
        vec[OpCode::Add as usize] = Some(VM::opcode_add as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Subtract as usize] =
            Some(VM::opcode_subtract as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Multiply as usize] =
            Some(VM::opcode_multiply as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Divide as usize] = Some(VM::opcode_divide as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Eq as usize] = Some(VM::opcode_eq as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Gt as usize] = Some(VM::opcode_gt as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Lt as usize] = Some(VM::opcode_lt as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Print as usize] = Some(VM::opcode_print as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Drop as usize] = Some(VM::opcode_drop as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Clear as usize] = Some(VM::opcode_clear as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Dup as usize] = Some(VM::opcode_dup as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Over as usize] = Some(VM::opcode_over as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Swap as usize] = Some(VM::opcode_swap as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Rot as usize] = Some(VM::opcode_rot as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Depth as usize] = Some(VM::opcode_depth as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::IsNull as usize] = Some(VM::opcode_isnull as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::DupIsNull as usize] =
            Some(VM::opcode_dupisnull as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::IsList as usize] = Some(VM::opcode_islist as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::IsCallable as usize] =
            Some(VM::opcode_iscallable as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::IsShiftable as usize] =
            Some(VM::opcode_isshiftable as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Open as usize] = Some(VM::opcode_open as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Readline as usize] =
            Some(VM::opcode_readline as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Str as usize] = Some(VM::opcode_str as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Int as usize] = Some(VM::opcode_int as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Flt as usize] = Some(VM::opcode_flt as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Rand as usize] = Some(VM::opcode_rand as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Push as usize] = Some(VM::opcode_push as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Pop as usize] = Some(VM::opcode_pop as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::ToggleMode as usize] = Some(VM::opcode_togglemode as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::PrintStack as usize] = Some(VM::opcode_printstack as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::ToFunction as usize] = Some(VM::opcode_tofunction as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
        vec[OpCode::Import as usize] = Some(VM::opcode_import as fn(&mut VM, Rc<RefCell<Chunk>>, usize) -> i32);
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
            scopes: vec![Rc::new(RefCell::new(HashMap::new()))],
            global_functions: HashMap::new(),
            call_stack_chunks: Vec::new(),
            running: Arc::new(AtomicBool::new(true)),
            sys: System::new(),
        }
    }

    #[allow(unused_variables)]
    pub fn opcode_togglemode(
        &mut self,
        chunk: Rc<RefCell<Chunk>>,
        i: usize
    ) -> i32 {
        self.print_stack = !self.print_stack;
        return 1;
    }

    pub fn opcode_printstack(
        &mut self,
        chunk: Rc<RefCell<Chunk>>,
        i: usize
    ) -> i32 {
        self.print_stack(chunk, i, true);
        return 1;
    }

    pub fn opcode_tofunction(
        &mut self,
        chunk: Rc<RefCell<Chunk>>,
        i: usize
    ) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "to-function requires one argument");
            return 0;
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
                let sv = self.string_to_callable(
                    chunk, s
                );
                match sv {
                    Some(v) => {
                        self.stack.push(v);
                        pushed = true;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        if !pushed {
            self.stack.push(backup_rr);
        }
        return 1;
    }

    pub fn opcode_import(
        &mut self,
        chunk: Rc<RefCell<Chunk>>,
        i: usize
    ) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "import requires one argument");
            return 0;
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
                        for (k, v) in import_chunk.functions.iter() {
                            self.global_functions.insert(k.clone(), v.clone());
                        }
                    }
                    None => {
                        return 0;
                    }
                }
            }
            _ => {
                print_error(chunk, i, "import argument must be a string");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a wrapped value as its single argument, and returns a
    /// wrapped value for the stringified representation of the argument.
    pub fn to_string_value(value_rr: Value) -> Option<Value> {
        let is_string;
        {
            match value_rr {
                Value::String(_) => {
                    is_string = true;
                }
                _ => {
                    is_string = false;
                }
            }
        }
        if is_string {
            return Some(value_rr);
        } else {
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
                Some(s) => Some(Value::String(Rc::new(RefCell::new(StringPair::new(
                    s.to_string(),
                    None,
                ))))),
                _ => None,
            }
        }
    }

    pub fn call_named_function<'a>(
        &mut self,
        chunk: Rc<RefCell<Chunk>>,
        i: usize,
        line_col: (u32, u32),
        plvs_stack: Option<Rc<RefCell<Vec<Value>>>>,
        call_chunk: Rc<RefCell<Chunk>>,
    ) -> bool {
        self.call_stack_chunks.push(chunk.clone());
        if call_chunk.borrow().is_generator {
            let mut gen_args = Vec::new();
            let req_arg_count = call_chunk.borrow().req_arg_count;
            if self.stack.len() < req_arg_count.try_into().unwrap() {
                let err_str = format!(
                    "generator requires {} argument{}",
                    req_arg_count,
                    if req_arg_count > 1 { "s" } else { "" }
                );
                print_error(chunk.clone(), i, &err_str);
                return false;
            }
            let mut arg_count = call_chunk.borrow().arg_count;
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
            for i in self.call_stack_chunks.iter() {
                gen_call_stack_chunks.push((*i).clone());
            }
            let gen_rr = Value::Generator(Rc::new(RefCell::new(GeneratorObject::new(
                Rc::new(RefCell::new(HashMap::new())),
                Rc::new(RefCell::new(Vec::new())),
                0,
                call_chunk,
                gen_call_stack_chunks,
                gen_args
            ))));
            self.stack.push(gen_rr);
        } else {
            if call_chunk.borrow().has_vars {
                self.scopes.push(Rc::new(RefCell::new(HashMap::new())));
            }

            let mut prev_stack = None;
            let has_plvs_stack = !plvs_stack.is_none();
            if has_plvs_stack {
                self.local_var_stack = plvs_stack.unwrap();
            } else if !call_chunk.borrow().nested {
                prev_stack = Some(self.local_var_stack.clone());
                self.local_var_stack = Rc::new(RefCell::new(vec![]));
            }

            let res = self.run(
                call_chunk.clone(),
                0,
                line_col
            );

            if !has_plvs_stack && !call_chunk.borrow().nested {
                self.local_var_stack = prev_stack.unwrap();
            }

            self.call_stack_chunks.pop();

            if res == 0 {
                return false;
            }
        }
        return true;
    }

    pub fn string_to_callable(
        &mut self,
        chunk: Rc<RefCell<Chunk>>,
        s: &str
    ) -> Option<Value> {
        let sf_fn_opt = SIMPLE_FORMS.get(s);
        if !sf_fn_opt.is_none() {
            let sf_fn = sf_fn_opt.unwrap();
            let nv = Value::CoreFunction(*sf_fn);
            return Some(nv);
        }

        let shift_fn_opt = SHIFT_FORMS.get(&s as &str);
        if !shift_fn_opt.is_none() {
            let shift_fn = shift_fn_opt.unwrap();
            let nv = Value::ShiftFunction(*shift_fn);
            return Some(nv);
        }

        self.call_stack_chunks.push(chunk);

        let global_function;
        let mut call_chunk_opt = None;

        for sf in self.call_stack_chunks.iter().rev() {
            let sfb = sf.borrow();
            if sfb.functions.contains_key(s) {
                let call_chunk = sfb.functions.get(s).unwrap();
                call_chunk_opt = Some(call_chunk.clone());
                break;
            }
        }
        if call_chunk_opt.is_none() && self.global_functions.contains_key(s) {
            global_function = self.global_functions.get(s).unwrap().clone();
            call_chunk_opt = Some(global_function.clone());
        }
        match call_chunk_opt {
            Some(call_chunk) => {
                let nv = Value::NamedFunction(
                    call_chunk.clone()
                );
                self.call_stack_chunks.pop();
                return Some(nv);
            }
            _ => {}
        }

        self.call_stack_chunks.pop();

        return None;
    }

    pub fn call_string(
        &mut self,
        chunk: Rc<RefCell<Chunk>>,
        i: usize,
        line_col: (u32, u32),
        plvs_stack: Option<Rc<RefCell<Vec<Value>>>>,
        is_implicit: bool,
        s: &str,
    ) -> bool {
        let sv = self.string_to_callable(
            chunk.clone(), s
        );
        match sv {
            Some(Value::CoreFunction(sf_fn)) => {
                let n = sf_fn(self, chunk, i);
                if n == 0 {
                    return false;
                }
                return true;
            }
            Some(Value::ShiftFunction(shift_fn)) => {
                let n = shift_fn(
                    self,
                    chunk,
                    i,
                    line_col
                );
                if n == 0 {
                    return false;
                }
                return true;
            }
            Some(Value::NamedFunction(named_fn)) => {
                self.call_stack_chunks.push(chunk.clone());
                let res = self.call_named_function(
                    chunk.clone(),
                    0,
                    line_col,
                    plvs_stack,
                    named_fn.clone(),
                );
                self.call_stack_chunks.pop();
                return res;
            }
            _ => {}
        }

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
        chunk: Rc<RefCell<Chunk>>,
        i: usize,
        call_opcode: OpCode,
        function_rr: Option<Value>,
        function_str: Option<&str>,
        function_str_index: i32,
        line_col: (u32, u32),
    ) -> bool {
        // Determine whether the function has been called implicitly.
        let is_implicit;
        match call_opcode {
            OpCode::CallImplicit | OpCode::CallImplicitConstant => {
                is_implicit = true;
            }
            OpCode::Call | OpCode::CallConstant => {
                is_implicit = false;
            }
            _ => {
                eprintln!("unexpected opcode!");
                std::process::abort();
            }
        }

        let mut cv = Value::Null;
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
                        let cfb = &chunk.borrow().constant_values;
                        let cv = cfb.get(function_str_index as usize);
                        match cv {
                            Some(Value::Null) | None => {
                                not_present = true;
                            }
                            _ => {
                                not_present = false;
                            }
                        }
                    }
                    if not_present {
                        let sv = self.string_to_callable(
                            chunk.clone(), s
                        );
                        match sv {
                            Some(v) => {
                                chunk
                                    .borrow_mut()
                                    .constant_values
                                    .resize(function_str_index as usize, Value::Null);
                                chunk
                                    .borrow_mut()
                                    .constant_values
                                    .insert(function_str_index as usize, v);
                            }
                            _ => {}
                        }
                    }

                    cv = chunk.borrow().get_constant_value(function_str_index);
                }
                match cv {
                    Value::Null => {
                        return self.call_string(
                            chunk,
                            0,
                            line_col,
                            None,
                            is_implicit,
                            s,
                        );
                    }
                    _ => {}
                }
            }
            _ => {
                cv = function_rr.unwrap();
            }
        }

        match cv {
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
                    chunk,
                    i,
                    line_col,
                );
                if n == 0 {
                    return false;
                }
                return true;
            }
            Value::NamedFunction(call_chunk_rc) => {
                return self.call_named_function(
                    chunk,
                    0,
                    line_col,
                    None,
                    call_chunk_rc,
                );
            }
            Value::AnonymousFunction(call_chunk_rc, lvs) => {
                return self.call_named_function(
                    chunk.clone(),
                    0,
                    line_col,
                    Some(lvs),
                    call_chunk_rc.clone(),
                );
            }
            Value::String(sp) => {
                let s = &sp.borrow().s;
                if self.debug {
                    eprintln!("instantiating new chunk functions for string: {}", s);
                }
                return self.call_string(
                    chunk,
                    0,
                    line_col,
                    None,
                    is_implicit,
                    &s,
                );
            }
            _ => {
                if is_implicit {
                    self.stack.push(cv.clone());
                } else {
                    print_error(chunk, i, "function not found");
                    return false;
                }
            }
        }

        return true;
    }


    /// Takes the global functions, the call stack chunks, the current
    /// chunk, the values for the current chunk, the instruction
    /// index, the global variables for the current generator (if
    /// applicable), the local variables for the current generator (if
    /// applicable), the previous local variable stacks, and the
    /// current line and column number as its arguments.  Runs the
    /// code from the chunk, beginning at the specified instruction
    /// index.
    pub fn run<'a>(
        &mut self,
        chunk: Rc<RefCell<Chunk>>,
        index: usize,
        line_col: (u32, u32),
    ) -> usize {
        let mut i = index;

        let mut list_count = 0;
        let mut list_indexes = Vec::new();
        let mut list_types = Vec::new();

        while i < chunk.borrow().data.len() {
            if !self.running.load(Ordering::SeqCst) {
                self.running.store(true, Ordering::SeqCst);
                self.stack.clear();
                return 0;
            }
            let op = to_opcode(chunk.borrow().data[i]);
            if self.debug {
                eprintln!(">  Opcode: {:?}", op);
                eprintln!(" > Stack:  {:?}", self.stack);
                eprintln!(" > Index:  {:?}", i);
            }
            let op_fn_opt = SIMPLE_OPS[op as usize];
            if !op_fn_opt.is_none() {
                let op_fn = op_fn_opt.unwrap();
                let res = op_fn(self, chunk.clone(), i);
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
                    let i_upper = chunk.borrow().data[i];
                    i = i + 1;
                    let i_lower = chunk.borrow().data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let mut done = false;
                    if chunk.borrow().has_constant_int(i2 as i32) {
                        let n = chunk.borrow().get_constant_int(i2 as i32);
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
                        self.stack.push(chunk.borrow().get_constant(i2 as i32));
                        let op_fn = op_fn_opt.unwrap();
                        let res = op_fn(self, chunk.clone(), i);
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
                    let i_upper = chunk.borrow().data[i];
                    i = i + 1;
                    let i_lower = chunk.borrow().data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let mut done = false;
                    if chunk.borrow().has_constant_int(i2 as i32) {
                        let n = chunk.borrow().get_constant_int(i2 as i32);
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
                        self.stack.push(chunk.borrow().get_constant(i2 as i32));
                        let op_fn = op_fn_opt.unwrap();
                        let res = op_fn(self, chunk.clone(), i);
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
                    let i_upper = chunk.borrow().data[i];
                    i = i + 1;
                    let i_lower = chunk.borrow().data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let mut done = false;
                    if chunk.borrow().has_constant_int(i2 as i32) {
                        let n = chunk.borrow().get_constant_int(i2 as i32);
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
                        self.stack.push(chunk.borrow().get_constant(i2 as i32));
                        let op_fn = op_fn_opt.unwrap();
                        let res = op_fn(self, chunk.clone(), i);
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
                    let i_upper = chunk.borrow().data[i];
                    i = i + 1;
                    let i_lower = chunk.borrow().data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let mut done = false;
                    if chunk.borrow().has_constant_int(i2 as i32) {
                        let n = chunk.borrow().get_constant_int(i2 as i32);
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
                        self.stack.push(chunk.borrow().get_constant(i2 as i32));
                        let op_fn = op_fn_opt.unwrap();
                        let res = op_fn(self, chunk.clone(), i);
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
                    let i_upper = chunk.borrow().data[i];
                    i = i + 1;
                    let i_lower = chunk.borrow().data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let n = chunk.borrow().get_constant_int(i2 as i32);

                    let len = self.stack.len();
                    let v1_rr = self.stack.get_mut(len - 1).unwrap();
                    let mut done = false;
                    match v1_rr {
                        Value::Int(ref mut n1) => {
                            if *n1 == n {
                                *n1 = 1;
                            } else {
                                *n1 = 0;
                            }
                            done = true;
                        }
                        _ => {}
                    };
                    if !done {
                        let op_fn_opt = SIMPLE_OPS[OpCode::Eq as usize];
                        self.stack.push(chunk.borrow().get_constant(i2 as i32));
                        let op_fn = op_fn_opt.unwrap();
                        let res = op_fn(self, chunk.clone(), i);
                        if res == 0 {
                            return 0;
                        }
                    }
                }
                OpCode::StartList => {
                    list_indexes.push(self.stack.len());
                    list_types.push(ListType::List);
                    list_count = list_count + 1;
                }
                OpCode::StartHash => {
                    list_indexes.push(self.stack.len());
                    list_types.push(ListType::Hash);
                    list_count = list_count + 1;
                }
                OpCode::EndList => {
                    if list_count == 0 {
                        print_error(chunk, i, "no start list found");
                        return 0;
                    }
                    let list_index = list_indexes.pop().unwrap();
                    let list_type = list_types.pop().unwrap();
                    list_count = list_count - 1;

                    match list_type {
                        ListType::List => {
                            let mut lst = VecDeque::new();
                            while self.stack.len() > list_index {
                                lst.push_front(self.stack.pop().unwrap());
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
                            self.stack.push(Value::Hash(Rc::new(RefCell::new(map))));
                        }
                    }
                },
                OpCode::Function => {
                    // todo: The logic here is awkward, and needs
                    // reviewing.
                    i = i + 1;
                    let i_upper = chunk.borrow().data[i];
                    i = i + 1;
                    let i_lower = chunk.borrow().data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let value_rr = chunk.borrow().get_constant(i2 as i32);
                    let mut copy = false;

                    match value_rr {
                        Value::String(ref sp) => {
                            let s = &sp.borrow().s;
                            let cfb = &chunk.borrow().constant_values;
                            match cfb.get(i2 as usize) {
                                Some(Value::String(_)) => {
                                    self.stack.push(
                                        Value::AnonymousFunction(
                                            chunk.borrow().functions.get(s).unwrap().clone(),
                                            self.local_var_stack.clone()
                                        )
                                    )
                                }
                                Some(_) => {
                                    eprintln!("unexpected function value!");
                                    std::process::abort();
                                }
                                _ => {
                                    copy = true;
                                }
                            }
                        }
                        _ => {
                            eprintln!("unexpected function value!");
                            std::process::abort();
                        }
                    }
                    if copy {
                        chunk
                            .borrow_mut()
                            .constant_values
                            .resize(i2 as usize, Value::Null);
                        chunk
                            .borrow_mut()
                            .constant_values
                            .insert(i2 as usize, value_rr);
                        let cfb = &chunk.borrow().constant_values;
                        let cv_value_rr = cfb.get(i2 as usize).unwrap().clone();
                        match cv_value_rr {
                            Value::String(ref sp) => {
                                self.stack.push(
                                    Value::AnonymousFunction(
                                        chunk.borrow().functions.get(&sp.borrow().s.to_string()).unwrap().clone(),
                                        self.local_var_stack.clone()
                                    )
                                )
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
                    let i_upper = chunk.borrow().data[i];
                    i = i + 1;
                    let i_lower = chunk.borrow().data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let mut inst = false;

                    {
                        let cfb = &chunk.borrow().constant_values;
                        let iv = cfb.get(i2 as usize);
                        if self.debug {
                            eprintln!("CFP: {:?}", iv);
                        }
                        match iv {
                            Some(Value::Null) => {
                                inst = true;
                            }
                            Some(_) => {
                                let value_rr = iv.unwrap().clone();
                                if self.debug {
                                    eprintln!("got cached value: {:?}", value_rr);
                                }
                                self.stack.push(value_rr);
                            }
                            _ => {
                                inst = true;
                            }
                        }
                    }
                    if inst {
                        let value_rr = chunk.borrow().get_constant(i2 as i32);
                        chunk
                            .borrow_mut()
                            .constant_values
                            .resize(i2 as usize, Value::Null);
                        chunk
                            .borrow_mut()
                            .constant_values
                            .insert(i2 as usize, value_rr.clone());
                        self.stack.push(value_rr.clone());
                    }
                }
                OpCode::CallConstant | OpCode::CallImplicitConstant => {
                    i = i + 1;
                    let i_upper = chunk.borrow().data[i];
                    i = i + 1;
                    let i_lower = chunk.borrow().data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);

                    let (mut line, mut col) = line_col;
                    if line == 0 && col == 0 {
                        let point = chunk.borrow().get_point(i);
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

                    let value_sd = chunk.borrow().constants[i2 as usize].clone();
                    match value_sd {
                        ValueSD::String(ref sp) => {
                            let res = self.call(
                                chunk.clone(),
                                i,
                                op,
                                None,
                                Some(sp),
                                (i2 as u32).try_into().unwrap(),
                                (line, col),
                            );

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

                    let function_rr = self.stack.pop().unwrap();

                    let (mut line, mut col) = line_col;
                    if line == 0 && col == 0 {
                        let point = chunk.borrow().get_point(i);
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
                        chunk.clone(),
                        i,
                        op,
                        Some(function_rr),
                        None,
                        -1,
                        (line, col),
                    );

                    if !res {
                        return 0;
                    }
                }
                OpCode::GLVCall => {
                    i = i + 1;
                    let var_index: u8 = chunk.borrow().data[i].try_into().unwrap();

                    let function_rr = self
                        .local_var_stack
                        .borrow()
                        .index(var_index as usize)
                        .clone();

                    let (mut line, mut col) = line_col;
                    if line == 0 && col == 0 {
                        let point = chunk.borrow().get_point(i);
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
                        chunk.clone(),
                        i,
                        OpCode::Call,
                        Some(function_rr),
                        None,
                        -1,
                        (line, col),
                    );

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
                    let var_index: u8 = chunk.borrow().data[i].try_into().unwrap();
                    let value_rr = self.stack.pop().unwrap();

                    if var_index == (self.local_var_stack.borrow().len() as u8) {
                        self.local_var_stack.borrow_mut().push(value_rr);
                    } else {
                        let lvs_b = &mut self.local_var_stack.borrow_mut();
                        let existing_value_rr_ptr = lvs_b.index_mut(var_index as usize);
                        *existing_value_rr_ptr = value_rr;
                    }
                }
                OpCode::GetLocalVar => {
                    i = i + 1;
                    let var_index: u8 = chunk.borrow().data[i].try_into().unwrap();

                    let value_rr = self
                        .local_var_stack
                        .borrow()
                        .index(var_index as usize)
                        .clone();
                    self.stack.push(value_rr);
                }
                OpCode::GLVShift => {
                    i = i + 1;
                    let var_index: u8 = chunk.borrow().data[i].try_into().unwrap();

                    let mut pt = self.local_var_stack
                        .borrow().index(var_index as
                        usize).clone();
                    let i2 = self.opcode_shift_inner(
                        chunk.clone(),
                        i,
                        line_col,
                        &mut pt
                    );
                    if i2 == 0 {
                        return 0;
                    }
                }
                OpCode::PopLocalVar => {
                    self.local_var_stack.borrow_mut().pop();
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

                    self.scopes
                        .last_mut()
                        .unwrap()
                        .borrow_mut()
                        .insert(var_name.to_string(), Value::Int(0));
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

                            for scope in self.scopes.iter_mut().rev() {
                                if scope.borrow().contains_key(s) {
                                    scope.borrow_mut().insert(s.to_string(), value_rr.clone());
                                    done = true;
                                    break;
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

                            for scope in self.scopes.iter().rev() {
                                if scope.borrow().contains_key(s) {
                                    self.stack.push(scope.borrow().get(s).unwrap().clone());
                                    done = true;
                                    break;
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
                    let i1: usize = chunk.borrow().data[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = chunk.borrow().data[i].try_into().unwrap();
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
                    let i1: usize = chunk.borrow().data[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = chunk.borrow().data[i].try_into().unwrap();
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
                    let i1: usize = chunk.borrow().data[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = chunk.borrow().data[i].try_into().unwrap();
                    let jmp_len: usize = (i1 << 8) | i2;

                    i = i + 1;
                    let i_upper = chunk.borrow().data[i];
                    i = i + 1;
                    let i_lower = chunk.borrow().data[i];
                    let i3 = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let value_rr = self.stack.last().unwrap();
                    if chunk.borrow().has_constant_int(i3 as i32) {
                        let cmp_rr = chunk.borrow().get_constant_int(i3 as i32);

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
                        chunk.clone(),
                        i,
                        line_col,
                    );
                    if i2 == 0 {
                        return 0;
                    }
                }
                OpCode::Yield => {
                    if !chunk.borrow().is_generator {
                        eprintln!("error: yield without generator");
                        return 0;
                    }
                    return i + 1;
                }
                OpCode::Jump => {
                    i = i + 1;
                    let i1: usize = chunk.borrow().data[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = chunk.borrow().data[i].try_into().unwrap();
                    let jmp_len: usize = (i1 << 8) | i2;
                    i = i + jmp_len;
                }
                OpCode::JumpR => {
                    i = i + 1;
                    let i1: usize = chunk.borrow().data[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = chunk.borrow().data[i].try_into().unwrap();
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
                        let point = chunk.borrow().get_point(i);
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
                    if !chunk.borrow().is_generator && chunk.borrow().has_vars {
                        self.scopes.pop();
                    }
                    return i + 1;
                }
                OpCode::Return => {
                    if !chunk.borrow().is_generator && chunk.borrow().has_vars {
                        self.scopes.pop();
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

        if list_count > 0 {
            print_error(chunk.clone(), i, "unterminated list start");
            return 0;
        }

        if self.print_stack {
            self.print_stack(chunk.clone(), i, false);
            self.stack.clear();
        }

        return i + 1;
    }

    /// Takes the global functions and the file to read the program
    /// code from as its arguments.  Compiles the program code and
    /// executes it, returning the chunk (if compiled successfully).
    pub fn interpret(
        &mut self,
        global_functions: &HashMap<String, Rc<RefCell<Chunk>>>,
        fh: &mut Box<dyn BufRead>,
        name: &str,
    ) -> Option<Rc<RefCell<Chunk>>> {
	for (k, v) in global_functions.iter() {
	    self.global_functions.insert(k.clone(), v.clone());
	}

        let mut compiler = Compiler::new(self.debug);
        let chunk_opt = compiler.compile(fh, name);
        match chunk_opt {
            None => return None,
            _ => {}
        }
        let chunk = Rc::new(RefCell::new(chunk_opt.unwrap()));

        self.run(
            chunk.clone(),
            0,
            (0, 0),
        );
        if self.print_stack {
            self.stack.clear();
        }
        return Some(chunk);
    }
}

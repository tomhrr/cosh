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

use chunk::{print_error, Chunk, Value};
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
pub fn to_string_2<'a>(
    v: &'a (Option<&str>, Option<String>),
) -> Option<&'a str> {
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
    stack: Vec<Rc<RefCell<Value>>>,
    // Whether the stack should be printed after interpretation has
    // finished.
    print_stack: bool,
    // The local variable stack.
    local_var_stack: Rc<RefCell<Vec<Rc<RefCell<Value>>>>>,
    // A System object, for getting process information.
    sys: System,
}

lazy_static! {
    static ref SIMPLE_FORMS: HashMap<&'static str, fn(&mut VM, &Chunk, usize) -> i32> = {
        let mut map = HashMap::new();
        map.insert("+", VM::opcode_add as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("-", VM::opcode_subtract as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("*", VM::opcode_multiply as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("/", VM::opcode_divide as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("=", VM::opcode_eq as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert(">", VM::opcode_gt as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("<", VM::opcode_lt as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("print", VM::opcode_print as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("drop", VM::opcode_drop as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("clear", VM::opcode_clear as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("dup", VM::opcode_dup as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("over", VM::opcode_over as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("swap", VM::opcode_swap as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("rot", VM::opcode_rot as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("depth", VM::opcode_depth as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("is-null", VM::opcode_isnull as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("is-list", VM::opcode_islist as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("is-callable", VM::opcode_iscallable as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("is-shiftable", VM::opcode_isshiftable as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("open", VM::opcode_open as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("readline", VM::opcode_readline as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("println", VM::core_println as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("rm", VM::core_rm as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("writeline", VM::core_writeline as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("close", VM::core_close as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("opendir", VM::core_opendir as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("readdir", VM::core_readdir as fn(&mut VM, &Chunk, usize) -> i32);
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
        map.insert("append", VM::core_append as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("push", VM::core_push as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("unshift", VM::core_unshift as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("pop", VM::core_pop as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("len", VM::core_len as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("is-dir", VM::core_is_dir as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("split", VM::core_split as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("at", VM::core_at as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("at!", VM::core_at_em as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("keys", VM::core_keys as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("values", VM::core_values as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("each", VM::core_each as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("from-json", VM::core_from_json as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("to-json", VM::core_to_json as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("from-xml", VM::core_from_xml as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("to-xml", VM::core_to_xml as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("str", VM::opcode_str as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("int", VM::opcode_int as fn(&mut VM, &Chunk, usize) -> i32);
        map.insert("flt", VM::opcode_flt as fn(&mut VM, &Chunk, usize) -> i32);
        map
    };

    static ref SHIFT_FORMS: HashMap<&'static str, fn(&mut VM, &mut Vec<RefCell<HashMap<String, Rc<RefCell<Value>>>>>, &mut RefCell<HashMap<String, Chunk>>, &mut Vec<Rc<RefCell<Vec<Rc<RefCell<Value>>>>>>, &Chunk, usize, (u32, u32), Arc<AtomicBool>) -> i32> = {
        let mut map = HashMap::new();
        map.insert("shift", VM::opcode_shift as fn(&mut VM, &mut Vec<RefCell<HashMap<String, Rc<RefCell<Value>>>>>, &mut RefCell<HashMap<String, Chunk>>, &mut Vec<Rc<RefCell<Vec<Rc<RefCell<Value>>>>>>, &Chunk, usize, (u32, u32), Arc<AtomicBool>) -> i32);
        map.insert("gnth", VM::core_gnth as fn(&mut VM, &mut Vec<RefCell<HashMap<String, Rc<RefCell<Value>>>>>, &mut RefCell<HashMap<String, Chunk>>, &mut Vec<Rc<RefCell<Vec<Rc<RefCell<Value>>>>>>, &Chunk, usize, (u32, u32), Arc<AtomicBool>) -> i32);
        map.insert("|", VM::core_pipe as fn(&mut VM, &mut Vec<RefCell<HashMap<String, Rc<RefCell<Value>>>>>, &mut RefCell<HashMap<String, Chunk>>, &mut Vec<Rc<RefCell<Vec<Rc<RefCell<Value>>>>>>, &Chunk, usize, (u32, u32), Arc<AtomicBool>) -> i32);
        map.insert("shift-all", VM::core_shift_all as fn(&mut VM, &mut Vec<RefCell<HashMap<String, Rc<RefCell<Value>>>>>, &mut RefCell<HashMap<String, Chunk>>, &mut Vec<Rc<RefCell<Vec<Rc<RefCell<Value>>>>>>, &Chunk, usize, (u32, u32), Arc<AtomicBool>) -> i32);
        map.insert("join", VM::core_join as fn(&mut VM, &mut Vec<RefCell<HashMap<String, Rc<RefCell<Value>>>>>, &mut RefCell<HashMap<String, Chunk>>, &mut Vec<Rc<RefCell<Vec<Rc<RefCell<Value>>>>>>, &Chunk, usize, (u32, u32), Arc<AtomicBool>) -> i32);
        map
    };

    static ref SIMPLE_OPS: Vec<Option<fn(&mut VM, &Chunk, usize) -> i32>> = {
        let mut vec = vec![None; 255];
        vec[OpCode::Add as usize] = Some(VM::opcode_add as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Subtract as usize] = Some(VM::opcode_subtract as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Multiply as usize] = Some(VM::opcode_multiply as fn(&mut VM, &Chunk, usize) -> i32);
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
        vec[OpCode::IsList as usize] = Some(VM::opcode_islist as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::IsCallable as usize] = Some(VM::opcode_iscallable as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::IsShiftable as usize] = Some(VM::opcode_isshiftable as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Open as usize] = Some(VM::opcode_open as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Readline as usize] = Some(VM::opcode_readline as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Str as usize] = Some(VM::opcode_str as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Int as usize] = Some(VM::opcode_int as fn(&mut VM, &Chunk, usize) -> i32);
        vec[OpCode::Flt as usize] = Some(VM::opcode_flt as fn(&mut VM, &Chunk, usize) -> i32);
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
        scopes: &mut Vec<RefCell<HashMap<String, Rc<RefCell<Value>>>>>,
        global_functions: &mut RefCell<HashMap<String, Chunk>>,
        call_stack_chunks: &Vec<&Chunk>, chunk: &'a Chunk,
        chunk_values: &mut HashMap<String, Rc<RefCell<Value>>>, i: usize,
        call_opcode: OpCode, mut function_rr: Rc<RefCell<Value>>,
        gen_global_vars: Option<&mut HashMap<String, Rc<RefCell<Value>>>>,
        gen_local_vars_stack: Option<&mut Vec<Rc<RefCell<Value>>>>,
        prev_local_vars_stacks: &mut Vec<Rc<RefCell<Vec<Rc<RefCell<Value>>>>>>,
        line_col: (u32, u32), running: Arc<AtomicBool>,
    ) -> bool {
        // Determine whether the function has been called implicitly.
        let is_implicit;
        match call_opcode {
            OpCode::CallImplicit => {
                is_implicit = true;
            }
            OpCode::Call => {
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
            let function_rrb = function_rr.borrow();
            match &*function_rrb {
                Value::Function(s, vf_plvs_index, vf_plvs_ptr) => {
                    is_value_function = true;
                    plvs_index = *vf_plvs_index;
                    value_function_str = s.clone();
                    if (plvs_index + 1) > (prev_local_vars_stacks.len() as u32) {
                        print_error(
                            chunk,
                            i,
                            "cannot call function, as stack has gone away",
                        );
                        return false;
                    }
                    let plvs_ptr = prev_local_vars_stacks[plvs_index as usize].as_ptr()
                        as *const _ as u64;
                    if *vf_plvs_ptr != plvs_ptr {
                        print_error(
                            chunk,
                            i,
                            "cannot call function, as stack has gone away",
                        );
                        return false;
                    }
                }
                _ => {}
            }
        }
        if is_value_function {
            function_rr = Rc::new(RefCell::new(Value::String(
                value_function_str.to_string(),
                None,
            )));
        }

        let function = function_rr.borrow();
        match &*function {
            Value::Command(s) => {
                let i2 = self.core_command(&s, chunk, i);
                if i2 == 0 {
                    return false;
                }
            }
            Value::CommandUncaptured(s) => {
                let i2 = self.core_command_uncaptured(&s, chunk, i);
                if i2 == 0 {
                    return false;
                }
            }
            Value::String(s, _) => {
                let sf_fn_opt = SIMPLE_FORMS.get(&s as &str);
                if !sf_fn_opt.is_none() {
                    let sf_fn = sf_fn_opt.unwrap();
                    let n = sf_fn(self, chunk, i);
                    if n == 0 {
                        return false;
                    }
                    return true;
                }

                let shift_fn_opt = SHIFT_FORMS.get(&s as &str);
                if !shift_fn_opt.is_none() {
                    let shift_fn = shift_fn_opt.unwrap();
                    let n = shift_fn(self, scopes, global_functions, prev_local_vars_stacks, chunk, i, line_col, running);
                    if n == 0 {
                        return false;
                    }
                    return true;
                }

                if s == "toggle-mode" {
                    self.print_stack = !self.print_stack;
                } else if s == ".s" {
                    self.print_stack(
                        chunk,
                        i,
                        scopes,
                        global_functions,
                        running,
                        true,
                    );
                } else if s == "import" {
                    if self.stack.len() < 1 {
                        print_error(chunk, i, "import requires one argument");
                        return false;
                    }

                    let lib_rr = self.stack.pop().unwrap();
                    let lib_rrb = lib_rr.borrow();
                    let lib_str_pre = lib_rrb.to_string();
                    let lib_str_opt = to_string_2(&lib_str_pre);
                    match lib_str_opt {
                        Some(s) => {
                            let mut compiler = Compiler::new(false);
                            let import_chunk_opt = compiler.deserialise(s);
                            match import_chunk_opt {
                                Some(import_chunk) => {
                                    let mut global_functions_b =
                                        global_functions.borrow_mut();
                                    for (k, v) in
                                        import_chunk.functions.borrow().iter()
                                    {
                                        global_functions_b.insert(k.clone(), v.clone());
                                    }
                                }
                                None => {
                                    return false;
                                }
                            }
                        }
                        _ => {
                            print_error(
                                chunk,
                                i,
                                "import argument must be a string",
                            );
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
                        global_function = global_functions
                            .borrow()
                            .get(s)
                            .unwrap()
                            .clone();
                        call_chunk_opt = Some(&global_function);
                    }
                    match call_chunk_opt {
                        None => {
                            if is_implicit {
                                let value_rr = Rc::new(RefCell::new(Value::String(
                                    s.to_string(),
                                    None,
                                )));
                                self.stack.push(value_rr);
                            } else {
                                print_error(chunk, i, "function not found");
                                return false;
                            }
                        }
                        Some(call_chunk) => {
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
                                    gen_args.push(Rc::new(RefCell::new(Value::Null)));
                                }
                                let mut gen_call_stack_chunks = Vec::new();
                                for i in new_call_stack_chunks.iter() {
                                    gen_call_stack_chunks.push((*i).clone());
                                }
                                let gen_rr =
                                    Rc::new(RefCell::new(Value::Generator(
                                        HashMap::new(),
                                        Vec::new(),
                                        0,
                                        call_chunk.clone(),
                                        gen_call_stack_chunks,
                                        gen_args,
                                        HashMap::new(),
                                    )));
                                self.stack.push(gen_rr);
                            } else {
                                if call_chunk.has_vars {
                                    scopes.push(RefCell::new(HashMap::new()));
                                }

                                if is_value_function {
                                    self.local_var_stack =
                                        (*(prev_local_vars_stacks
                                            .get(plvs_index as usize)
                                            .unwrap()))
                                        .clone();
                                } else if call_chunk.nested {
                                    self.local_var_stack =
                                        (*(prev_local_vars_stacks
                                            .last()
                                            .unwrap()))
                                        .clone();
                                }

                                let res = self.run(
                                    scopes,
                                    global_functions,
                                    &new_call_stack_chunks,
                                    &call_chunk,
                                    chunk_values,
                                    0,
                                    gen_global_vars,
                                    gen_local_vars_stack,
                                    prev_local_vars_stacks,
                                    line_col,
                                    running,
                                );

                                if is_value_function {
                                    prev_local_vars_stacks[plvs_index as usize] =
                                        self.local_var_stack.clone();
                                } else if call_chunk.nested {
                                    let plvs_len = prev_local_vars_stacks.len();
                                    prev_local_vars_stacks[plvs_len - 1] =
                                        self.local_var_stack.clone();
                                }

                                if res == 0 {
                                    return false;
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                if is_implicit {
                    self.stack.push(function_rr.clone());
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
        scopes: &mut Vec<RefCell<HashMap<String, Rc<RefCell<Value>>>>>,
        global_functions: &mut RefCell<HashMap<String, Chunk>>,
        call_stack_chunks: &Vec<&Chunk>, chunk: &'a Chunk,
        chunk_values: &mut HashMap<String, Rc<RefCell<Value>>>, index: usize,
        mut gen_global_vars: Option<&mut HashMap<String, Rc<RefCell<Value>>>>,
        mut gen_local_vars_stack: Option<&mut Vec<Rc<RefCell<Value>>>>,
        prev_local_vars_stacks: &mut Vec<Rc<RefCell<Vec<Rc<RefCell<Value>>>>>>,
        line_col: (u32, u32), running: Arc<AtomicBool>,
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
                                    list_index_opt =
                                        Some(list_indexes.pop().unwrap());
                                } else {
                                    list_index_opt = None;
                                }
                                self.stack.push(Rc::new(RefCell::new(
                                    Value::List(lst),
                                )));
                            }
                            ListType::Hash => {
                                let mut map = IndexMap::new();
                                while self.stack.len() > list_index {
                                    let value_rr = self.stack.pop().unwrap();
                                    let key_rr = self.stack.pop().unwrap();
                                    let key_rrb = key_rr.borrow();
                                    let key_str_pre = key_rrb.to_string();
                                    let key_str = to_string_2(&key_str_pre).unwrap().to_string();
                                    map.insert(key_str, value_rr);
                                }
                                if list_indexes.len() > 0 {
                                    list_index_opt =
                                        Some(list_indexes.pop().unwrap());
                                } else {
                                    list_index_opt = None;
                                }
                                self.stack.push(Rc::new(RefCell::new(
                                    Value::Hash(map),
                                )));
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
                    let i2 = (((i_upper as u16) << 8) & 0xFF00)
                        | ((i_lower & 0xFF) as u16);
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
                        let value_rrb = value_rr.borrow();
                        match &*value_rrb {
                            Value::String(s, _) => {
                                match chunk_values.get(s) {
                                    Some(cv_value_rr) => {
                                        let cv_value_rrb = cv_value_rr.borrow();
                                        match &*cv_value_rrb {
                                            Value::String(cv_s, _) => {
                                                self.stack.push(Rc::new(
                                                    RefCell::new(
                                                        Value::Function(
                                                            cv_s.clone(),
                                                            plvs_index as u32,
                                                            plvs_ptr,
                                                        ),
                                                    ),
                                                ));
                                            }
                                            _ => {
                                                eprintln!("unexpected function value!");
                                                std::process::abort();
                                            }
                                        }
                                    }
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
                        chunk_values.insert(fn_name.clone().to_string(), value_rr);
                        let cv_value_rr = chunk_values.get(&fn_name).unwrap().clone();
                        let cv_value_rrb = cv_value_rr.borrow();
                        match &*cv_value_rrb {
                            Value::String(s, _) => {
                                self.stack.push(Rc::new(RefCell::new(
                                    Value::Function(
                                        s.clone(),
                                        plvs_index as u32,
                                        plvs_ptr,
                                    ),
                                )));
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
                    let i2 = (((i_upper as u16) << 8) & 0xFF00)
                        | ((i_lower & 0xFF) as u16);
                    let value_rr = chunk.get_constant(i2 as i32);
                    let mut copy = false;
                    let mut name = "".to_owned();
                    {
                        let value_rrb = value_rr.borrow();
                        match &*value_rrb {
                            Value::String(s, _) => {
                                match chunk_values.get(s) {
                                    Some(cv_value_rr) => {
                                        self.stack.push(cv_value_rr.clone());
                                    }
                                    _ => {
                                        name = s.clone();
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
                        chunk_values.insert(name.clone().to_string(), value_rr);
                        self.stack
                            .push(chunk_values.get(&name).unwrap().clone());
                    }
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
                        chunk_values,
                        i,
                        op,
                        function_rr,
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
                            if var_index == (glvs.len() as u8) {
                                glvs.push(value_rr);
                            } else {
                                glvs[var_index as usize] = value_rr;
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
                            let value_rr = glvs[var_index as usize].clone();
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
                        glvs.pop();
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
                        let var_name_rrb = var_name_rr.borrow();
                        match &*var_name_rrb {
                            Value::String(s, _) => {
                                var_name = s.clone().to_string();
                            }
                            _ => {
                                print_error(
                                    chunk,
                                    i,
                                    "variable name must be a string",
                                );
                                return 0;
                            }
                        }
                    }

                    match gen_global_vars {
                        Some(ref mut ggv) => {
                            ggv.insert(
                                var_name.to_string(),
                                Rc::new(RefCell::new(Value::Int(0))),
                            );
                        }
                        _ => {
                            scopes.last().unwrap().borrow_mut().insert(
                                var_name.to_string(),
                                Rc::new(RefCell::new(Value::Int(0))),
                            );
                        }
                    }
                }
                OpCode::SetVar => {
                    if self.stack.len() < 2 {
                        print_error(chunk, i, "! requires two arguments");
                        return 0;
                    }

                    let var_name_rr = self.stack.pop().unwrap();
                    let var_name_rrb = var_name_rr.borrow();
                    let value_rr = self.stack.pop().unwrap();

                    match &*var_name_rrb {
                        Value::String(s, _) => {
                            let mut done = false;

                            match gen_global_vars {
                                Some(ref mut ggv) => {
                                    if ggv.contains_key(s) {
                                        ggv.insert(s.clone(), value_rr.clone());
                                        done = true;
                                    }
                                }
                                _ => {}
                            }

                            if !done {
                                for scope in scopes.iter().rev() {
                                    if scope.borrow().contains_key(s) {
                                        scope.borrow_mut().insert(
                                            s.to_string(),
                                            value_rr.clone(),
                                        );
                                        done = true;
                                        break;
                                    }
                                }
                            }

                            if !done {
                                print_error(
                                    chunk,
                                    i,
                                    "could not find variable",
                                );
                                return 0;
                            }
                        }
                        _ => {
                            print_error(
                                chunk,
                                i,
                                "variable name must be a string",
                            );
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
                    let var_name_rrb = var_name_rr.borrow();
                    match &*var_name_rrb {
                        Value::String(s, _) => {
                            let mut done = false;

                            match gen_global_vars {
                                Some(ref mut ggv) => {
                                    if ggv.contains_key(s) {
                                        self.stack
                                            .push(ggv.get(s).unwrap().clone());
                                        done = true;
                                    }
                                }
                                _ => {}
                            }

                            if !done {
                                for scope in scopes.iter().rev() {
                                    if scope.borrow().contains_key(s) {
                                        self.stack.push(
                                            scope
                                                .borrow()
                                                .get(s)
                                                .unwrap()
                                                .clone(),
                                        );
                                        done = true;
                                        break;
                                    }
                                }
                            }
                            if !done {
                                print_error(
                                    chunk,
                                    i,
                                    "could not find variable",
                                );
                                return 0;
                            }
                        }
                        _ => {
                            print_error(
                                chunk,
                                i,
                                "variable name must be a string",
                            );
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
                    let value_rrb = value_rr.borrow();

                    i = i + 1;
                    let i1: usize = data[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = data[i].try_into().unwrap();
                    let jmp_len: usize = (i1 << 8) | i2;

                    match &*value_rrb {
                        Value::String(s, _) => {
                            if s == "" {
                                i = i + jmp_len;
                            }
                        }
                        Value::Int(0) => {
                            i = i + jmp_len;
                        }
                        Value::Float(nf) => {
                            if *nf == 0.0 {
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
                    let value_rrb = value_rr.borrow();

                    i = i + 1;
                    let i1: usize = data[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = data[i].try_into().unwrap();
                    let jmp_len: usize = (i1 << 8) | i2;

                    match &*value_rrb {
                        Value::String(s, _) => {
                            if s == "" {
                                i = i - jmp_len;
                            }
                        }
                        Value::Int(0) => {
                            i = i - jmp_len;
                        }
                        Value::Float(nf) => {
                            if *nf == 0.0 {
                                i = i - jmp_len;
                            }
                        }
                        _ => {}
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
                    let error_rrb = error_rr.borrow();
                    let error_str_pre = error_rrb.to_string();
                    let error_str_opt = to_string_2(&error_str_pre);
                    match error_str_opt {
                        Some(s) => {
                            let err_str = format!("{}:{}: {}", line, col, s);
                            eprintln!("{}", err_str);
                            return 0;
                        }
                        None => {
                            let err_str = format!(
                                "{}:{}: {}",
                                line, col, "(unknown error)"
                            );
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
            self.print_stack(
                chunk,
                i,
                scopes,
                global_functions,
                running,
                false,
            );
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
        &mut self, global_functions: HashMap<String, Chunk>,
        variables: HashMap<String, Rc<RefCell<Value>>>,
        fh: &mut Box<dyn BufRead>, running: Arc<AtomicBool>,
    ) -> (
        Option<Chunk>,
        HashMap<String, Rc<RefCell<Value>>>,
        Vec<RefCell<HashMap<String, Chunk>>>,
    ) {
        let mut compiler = Compiler::new(self.debug);
        let chunk_opt = compiler.compile(fh, "(main)");
        match chunk_opt {
            None => return (None, HashMap::new(), Vec::new()),
            _ => {}
        }
        let chunk = chunk_opt.unwrap();
        let mut global_functions_rr = RefCell::new(global_functions);
        let call_stack_chunks = vec![];
        let mut scopes = vec![RefCell::new(variables)];
        let mut chunk_values = HashMap::new();
        let mut prev_local_vars_stacks = vec![];

        self.run(
            &mut scopes,
            &mut global_functions_rr,
            &call_stack_chunks,
            &chunk,
            &mut chunk_values,
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

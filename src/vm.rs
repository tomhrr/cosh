use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::convert::TryInto;
use std::io::BufRead;
use std::io::BufReader;
use std::ops::Index;
use std::ops::IndexMut;
use std::rc::Rc;
use std::str;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use indexmap::IndexMap;
use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use sysinfo::{System, SystemExt};

use chunk::{print_error, Chunk, GeneratorObject, StringTriple, Value, ValueSD};
use compiler::Compiler;
use opcode::{to_opcode, OpCode};
use rl::RLHelper;

mod vm_arithmetic;
mod vm_basics;
mod vm_command;
mod vm_datetime;
mod vm_digest;
mod vm_env;
mod vm_hash;
mod vm_io;
mod vm_ip;
mod vm_json;
mod vm_list;
mod vm_print;
mod vm_regex;
mod vm_sort;
mod vm_string;
mod vm_system;
mod vm_xml;

/// For dealing with the EndList opcode, which also supports ending a
/// hash.
pub enum ListType {
    List,
    Hash,
    Set,
}

/// For running compiled bytecode.
pub struct VM {
    /// Whether to print debug information to standard error while
    /// running.
    debug: bool,
    /// The stack.
    stack: Vec<Value>,
    /// The last stack, i.e. the (possibly partially) reified state of
    /// the stack as at the conclusion of the last call.
    last_stack: Vec<Value>,
    /// The current chunk.
    chunk: Rc<RefCell<Chunk>>,
    /// The instruction index for the current chunk.
    i: usize,
    /// Whether the stack should be printed after interpretation has
    /// finished.
    print_stack: bool,
    /// Whether the stack is currently being printed.
    printing_stack: bool,
    /// The local variable stack.
    local_var_stack: Rc<RefCell<Vec<Value>>>,
    /// The scopes.
    scopes: Vec<Rc<RefCell<HashMap<String, Value>>>>,
    /// The global functions.
    global_functions: HashMap<String, Rc<RefCell<Chunk>>>,
    /// The call stack chunks.
    pub call_stack_chunks: Vec<(Rc<RefCell<Chunk>>, usize)>,
    /// A flag for interrupting execution.
    pub running: Arc<AtomicBool>,
    /// A lookup for regexes, to save regenerating them.
    pub regexes: HashMap<String, (Rc<Regex>, bool)>,
    /// The readline object, for fetching history.
    pub readline: Option<Rc<RefCell<rustyline::Editor<RLHelper>>>>,
    /// A System object, for getting process information.
    sys: System,
    /// The local time zone.
    local_tz: chrono_tz::Tz,
    /// The UTC timezone.
    utc_tz: chrono_tz::Tz,
}

lazy_static! {
    /// A map from form name to the internal function supporting that
    /// name.
    pub static ref SIMPLE_FORMS: HashMap<&'static str, fn(&mut VM) -> i32> = {
        let mut map = HashMap::new();
        map.insert("+", VM::opcode_add as fn(&mut VM) -> i32);
        map.insert("-", VM::opcode_subtract as fn(&mut VM) -> i32);
        map.insert("*", VM::opcode_multiply as fn(&mut VM) -> i32);
        map.insert("<<", VM::core_lsft as fn(&mut VM) -> i32);
        map.insert(">>", VM::core_rsft as fn(&mut VM) -> i32);
        map.insert("^", VM::core_xor as fn(&mut VM) -> i32);
        map.insert("||", VM::core_or as fn(&mut VM) -> i32);
        map.insert("&", VM::core_and as fn(&mut VM) -> i32);
        map.insert("/", VM::opcode_divide as fn(&mut VM) -> i32);
        map.insert("%", VM::opcode_remainder as fn(&mut VM) -> i32);
        map.insert("<=>", VM::opcode_cmp as fn(&mut VM) -> i32);
        map.insert("=", VM::opcode_eq as fn(&mut VM) -> i32);
        map.insert(">", VM::opcode_gt as fn(&mut VM) -> i32);
        map.insert("<", VM::opcode_lt as fn(&mut VM) -> i32);
        map.insert("print", VM::opcode_print as fn(&mut VM) -> i32);
        map.insert("drop", VM::opcode_drop as fn(&mut VM) -> i32);
        map.insert("clear", VM::opcode_clear as fn(&mut VM) -> i32);
        map.insert("dup", VM::opcode_dup as fn(&mut VM) -> i32);
        map.insert("over", VM::opcode_over as fn(&mut VM) -> i32);
        map.insert("swap", VM::opcode_swap as fn(&mut VM) -> i32);
        map.insert("rot", VM::opcode_rot as fn(&mut VM) -> i32);
        map.insert("depth", VM::opcode_depth as fn(&mut VM) -> i32);
        map.insert("is-null", VM::opcode_isnull as fn(&mut VM) -> i32);
        map.insert("is-list", VM::opcode_islist as fn(&mut VM) -> i32);
        map.insert("is-callable", VM::opcode_iscallable as fn(&mut VM) -> i32);
        map.insert("is-shiftable", VM::opcode_isshiftable as fn(&mut VM) -> i32);
        map.insert("open", VM::opcode_open as fn(&mut VM) -> i32);
        map.insert("tempfile", VM::opcode_tempfile as fn(&mut VM) -> i32);
        map.insert("tempdir", VM::opcode_tempdir as fn(&mut VM) -> i32);
        map.insert("readline", VM::opcode_readline as fn(&mut VM) -> i32);
        map.insert("read", VM::opcode_read as fn(&mut VM) -> i32);
        map.insert("println", VM::core_println as fn(&mut VM) -> i32);
        map.insert("rm", VM::core_rm as fn(&mut VM) -> i32);
        map.insert("writeline", VM::core_writeline as fn(&mut VM) -> i32);
        map.insert("write", VM::core_write as fn(&mut VM) -> i32);
        map.insert("close", VM::core_close as fn(&mut VM) -> i32);
        map.insert("opendir", VM::core_opendir as fn(&mut VM) -> i32);
        map.insert("readdir", VM::core_readdir as fn(&mut VM) -> i32);
        map.insert("cp", VM::core_cp as fn(&mut VM) -> i32);
        map.insert("mv", VM::core_mv as fn(&mut VM) -> i32);
        map.insert("rename", VM::core_rename as fn(&mut VM) -> i32);
        map.insert("cd", VM::core_cd as fn(&mut VM) -> i32);
        map.insert("pwd", VM::core_pwd as fn(&mut VM) -> i32);
        map.insert("touch", VM::core_touch as fn(&mut VM) -> i32);
        map.insert("stat", VM::core_stat as fn(&mut VM) -> i32);
        map.insert("lstat", VM::core_lstat as fn(&mut VM) -> i32);
        map.insert("ps", VM::core_ps as fn(&mut VM) -> i32);
        map.insert("kill", VM::core_kill as fn(&mut VM) -> i32);
        map.insert("m", VM::core_m as fn(&mut VM) -> i32);
        map.insert("s", VM::core_s as fn(&mut VM) -> i32);
        map.insert("c", VM::core_c as fn(&mut VM) -> i32);
        map.insert("++", VM::core_append as fn(&mut VM) -> i32);
        map.insert("push", VM::opcode_push as fn(&mut VM) -> i32);
        map.insert("unshift", VM::core_unshift as fn(&mut VM) -> i32);
        map.insert("pop", VM::opcode_pop as fn(&mut VM) -> i32);
        map.insert("len", VM::core_len as fn(&mut VM) -> i32);
        map.insert("empty", VM::core_empty as fn(&mut VM) -> i32);
        map.insert("is-dir", VM::core_is_dir as fn(&mut VM) -> i32);
        map.insert("split", VM::core_split as fn(&mut VM) -> i32);
        map.insert("splitr", VM::core_splitr as fn(&mut VM) -> i32);
        map.insert("get", VM::core_get as fn(&mut VM) -> i32);
        map.insert("set", VM::core_set as fn(&mut VM) -> i32);
        map.insert("keys", VM::core_keys as fn(&mut VM) -> i32);
        map.insert("values", VM::core_values as fn(&mut VM) -> i32);
        map.insert("each", VM::core_each as fn(&mut VM) -> i32);
        map.insert("from-json", VM::core_from_json as fn(&mut VM) -> i32);
        map.insert("to-json", VM::core_to_json as fn(&mut VM) -> i32);
        map.insert("from-xml", VM::core_from_xml as fn(&mut VM) -> i32);
        map.insert("to-xml", VM::core_to_xml as fn(&mut VM) -> i32);
        map.insert("bool", VM::opcode_bool as fn(&mut VM) -> i32);
        map.insert("byte", VM::opcode_byte as fn(&mut VM) -> i32);
        map.insert("str", VM::opcode_str as fn(&mut VM) -> i32);
        map.insert("int", VM::opcode_int as fn(&mut VM) -> i32);
        map.insert("float", VM::opcode_flt as fn(&mut VM) -> i32);
        map.insert("rand", VM::opcode_rand as fn(&mut VM) -> i32);
        map.insert("shift", VM::opcode_shift as fn(&mut VM) -> i32);
        map.insert("join", VM::core_join as fn(&mut VM) -> i32);
        map.insert("|", VM::core_pipe as fn(&mut VM) -> i32);
        map.insert("clone", VM::opcode_clone as fn(&mut VM) -> i32);
        map.insert("now", VM::core_now as fn(&mut VM) -> i32);
        map.insert("lcnow", VM::core_lcnow as fn(&mut VM) -> i32);
        map.insert("strftime", VM::core_strftime as fn(&mut VM) -> i32);
        map.insert("to-epoch", VM::core_to_epoch as fn(&mut VM) -> i32);
        map.insert("from-epoch", VM::core_from_epoch as fn(&mut VM) -> i32);
        map.insert("set-tz", VM::core_set_tz as fn(&mut VM) -> i32);
        map.insert("+time", VM::core_addtime as fn(&mut VM) -> i32);
        map.insert("-time", VM::core_subtime as fn(&mut VM) -> i32);
        map.insert("strptime", VM::core_strptime as fn(&mut VM) -> i32);
        map.insert("strptimez", VM::core_strptimez as fn(&mut VM) -> i32);
        map.insert("ip", VM::core_ip as fn(&mut VM) -> i32);
        map.insert("ip.from-int", VM::core_ip_from_int as fn(&mut VM) -> i32);
        map.insert("ip.addr", VM::core_ip_addr as fn(&mut VM) -> i32);
        map.insert("ip.len", VM::core_ip_len as fn(&mut VM) -> i32);
        map.insert("ip.addr-int", VM::core_ip_addr_int as fn(&mut VM) -> i32);
        map.insert("ip.last-addr", VM::core_ip_last_addr as fn(&mut VM) -> i32);
        map.insert(
            "ip.last-addr-int",
            VM::core_ip_last_addr_int as fn(&mut VM) -> i32,
        );
        map.insert("ip.size", VM::core_ip_size as fn(&mut VM) -> i32);
        map.insert("ip.version", VM::core_ip_version as fn(&mut VM) -> i32);
        map.insert("ip.prefixes", VM::core_ip_prefixes as fn(&mut VM) -> i32);
        map.insert("ips", VM::core_ips as fn(&mut VM) -> i32);
        map.insert("union", VM::core_union as fn(&mut VM) -> i32);
        map.insert("isect", VM::core_isect as fn(&mut VM) -> i32);
        map.insert("diff", VM::core_diff as fn(&mut VM) -> i32);
        map.insert("symdiff", VM::core_symdiff as fn(&mut VM) -> i32);
        map.insert("is-bool", VM::opcode_is_bool as fn(&mut VM) -> i32);
        map.insert("is-int", VM::opcode_is_int as fn(&mut VM) -> i32);
        map.insert("is-bigint", VM::opcode_is_bigint as fn(&mut VM) -> i32);
        map.insert("is-str", VM::opcode_is_str as fn(&mut VM) -> i32);
        map.insert("is-float", VM::opcode_is_flt as fn(&mut VM) -> i32);
        map.insert("is-set", VM::opcode_is_set as fn(&mut VM) -> i32);
        map.insert("is-hash", VM::opcode_is_hash as fn(&mut VM) -> i32);
        map.insert("bigint", VM::opcode_bigint as fn(&mut VM) -> i32);
        map.insert("chr", VM::core_chr as fn(&mut VM) -> i32);
        map.insert("ord", VM::core_ord as fn(&mut VM) -> i32);
        map.insert("hex", VM::core_hex as fn(&mut VM) -> i32);
        map.insert("oct", VM::core_oct as fn(&mut VM) -> i32);
        map.insert("lc", VM::core_lc as fn(&mut VM) -> i32);
        map.insert("lcfirst", VM::core_lcfirst as fn(&mut VM) -> i32);
        map.insert("uc", VM::core_uc as fn(&mut VM) -> i32);
        map.insert("ucfirst", VM::core_ucfirst as fn(&mut VM) -> i32);
        map.insert("reverse", VM::core_reverse as fn(&mut VM) -> i32);
        map.insert("sqrt", VM::core_sqrt as fn(&mut VM) -> i32);
        map.insert("**", VM::core_exp as fn(&mut VM) -> i32);
        map.insert("abs", VM::core_abs as fn(&mut VM) -> i32);
        map.insert("delete", VM::core_delete as fn(&mut VM) -> i32);
        map.insert("exists", VM::core_exists as fn(&mut VM) -> i32);
        map.insert("chmod", VM::core_chmod as fn(&mut VM) -> i32);
        map.insert("chown", VM::core_chown as fn(&mut VM) -> i32);
        map.insert("mkdir", VM::core_mkdir as fn(&mut VM) -> i32);
        map.insert("rmdir", VM::core_rmdir as fn(&mut VM) -> i32);
        map.insert("link", VM::core_link as fn(&mut VM) -> i32);
        map.insert("sleep", VM::core_sleep as fn(&mut VM) -> i32);
        map.insert("env", VM::core_env as fn(&mut VM) -> i32);
        map.insert("getenv", VM::core_getenv as fn(&mut VM) -> i32);
        map.insert("setenv", VM::core_setenv as fn(&mut VM) -> i32);
        map.insert("md5", VM::core_md5 as fn(&mut VM) -> i32);
        map.insert("sha1", VM::core_sha1 as fn(&mut VM) -> i32);
        map.insert("sha256", VM::core_sha256 as fn(&mut VM) -> i32);
        map.insert("sha512", VM::core_sha512 as fn(&mut VM) -> i32);
        map.insert("sort", VM::core_sort as fn(&mut VM) -> i32);
        map.insert("sortp", VM::core_sortp as fn(&mut VM) -> i32);
        map.insert("fmt", VM::core_fmt as fn(&mut VM) -> i32);
        map.insert("exec", VM::core_exec as fn(&mut VM) -> i32);
        map.insert("cmd", VM::core_cmd as fn(&mut VM) -> i32);
        map.insert("history", VM::core_history as fn(&mut VM) -> i32);
        map.insert("last", VM::core_last as fn(&mut VM) -> i32);
        map.insert("r", VM::core_reify as fn(&mut VM) -> i32);
        map
    };

    /// A set containing the function names that are defined in
    /// lib/rt.ch (i.e. that are core functions, but are not
    /// implemented in the compiler proper).  This is so that
    /// bin/cosh.rs can distinguish these function names from
    /// user-defined function names in its autocomplete logic.
    pub static ref LIB_FORMS: HashSet<&'static str> = {
        let mut set = HashSet::new();
        set.insert("2over");
        set.insert("lsh");
        set.insert("2rot");
        set.insert("lshr");
        set.insert("<=");
        set.insert("lsr");
        set.insert("product");
        set.insert(">=");
        set.insert("map");
        set.insert("range");
        set.insert("all");
        set.insert("map-generator");
        set.insert("and");
        set.insert("map-list");
        set.insert("shuffle");
        set.insert("any");
        set.insert("max");
        set.insert("chomp");
        set.insert("grep");
        set.insert("min");
        set.insert("grep-generator");
        set.insert("nip");
        set.insert("f<");
        set.insert("grep-list");
        set.insert("no-upwards");
        set.insert("sum");
        set.insert("flatten");
        set.insert("f>");
        set.insert("none");
        set.insert("take");
        set.insert("first");
        set.insert("not");
        set.insert("take-all");
        set.insert("foldl");
        set.insert("is-list-or-set");
        set.insert("notall");
        set.insert("uniq");
        set.insert("for");
        set.insert("ls");
        set.insert("or");
        set.insert("after");
        set.insert("before");
        set.insert("pairwise");
        set.insert("slide");
        set.insert("id");
        set
    };

    /// A vector mapping from opcode to the function implementing that
    /// opcode.
    static ref SIMPLE_OPS: Vec<Option<fn(&mut VM) -> i32>> = {
        let mut vec = vec![None; 255];
        vec[OpCode::Add as usize] = Some(VM::opcode_add as fn(&mut VM) -> i32);
        vec[OpCode::Subtract as usize] = Some(VM::opcode_subtract as fn(&mut VM) -> i32);
        vec[OpCode::Multiply as usize] = Some(VM::opcode_multiply as fn(&mut VM) -> i32);
        vec[OpCode::Divide as usize] = Some(VM::opcode_divide as fn(&mut VM) -> i32);
        vec[OpCode::Remainder as usize] = Some(VM::opcode_remainder as fn(&mut VM) -> i32);
        vec[OpCode::Cmp as usize] = Some(VM::opcode_cmp as fn(&mut VM) -> i32);
        vec[OpCode::Eq as usize] = Some(VM::opcode_eq as fn(&mut VM) -> i32);
        vec[OpCode::Gt as usize] = Some(VM::opcode_gt as fn(&mut VM) -> i32);
        vec[OpCode::Lt as usize] = Some(VM::opcode_lt as fn(&mut VM) -> i32);
        vec[OpCode::Print as usize] = Some(VM::opcode_print as fn(&mut VM) -> i32);
        vec[OpCode::Drop as usize] = Some(VM::opcode_drop as fn(&mut VM) -> i32);
        vec[OpCode::Clear as usize] = Some(VM::opcode_clear as fn(&mut VM) -> i32);
        vec[OpCode::Dup as usize] = Some(VM::opcode_dup as fn(&mut VM) -> i32);
        vec[OpCode::Over as usize] = Some(VM::opcode_over as fn(&mut VM) -> i32);
        vec[OpCode::Swap as usize] = Some(VM::opcode_swap as fn(&mut VM) -> i32);
        vec[OpCode::Rot as usize] = Some(VM::opcode_rot as fn(&mut VM) -> i32);
        vec[OpCode::Depth as usize] = Some(VM::opcode_depth as fn(&mut VM) -> i32);
        vec[OpCode::IsNull as usize] = Some(VM::opcode_isnull as fn(&mut VM) -> i32);
        vec[OpCode::DupIsNull as usize] = Some(VM::opcode_dupisnull as fn(&mut VM) -> i32);
        vec[OpCode::IsList as usize] = Some(VM::opcode_islist as fn(&mut VM) -> i32);
        vec[OpCode::IsCallable as usize] = Some(VM::opcode_iscallable as fn(&mut VM) -> i32);
        vec[OpCode::IsShiftable as usize] = Some(VM::opcode_isshiftable as fn(&mut VM) -> i32);
        vec[OpCode::Open as usize] = Some(VM::opcode_open as fn(&mut VM) -> i32);
        vec[OpCode::Readline as usize] = Some(VM::opcode_readline as fn(&mut VM) -> i32);
        vec[OpCode::Bool as usize] = Some(VM::opcode_bool as fn(&mut VM) -> i32);
        vec[OpCode::Str as usize] = Some(VM::opcode_str as fn(&mut VM) -> i32);
        vec[OpCode::Int as usize] = Some(VM::opcode_int as fn(&mut VM) -> i32);
        vec[OpCode::Flt as usize] = Some(VM::opcode_flt as fn(&mut VM) -> i32);
        vec[OpCode::Rand as usize] = Some(VM::opcode_rand as fn(&mut VM) -> i32);
        vec[OpCode::Push as usize] = Some(VM::opcode_push as fn(&mut VM) -> i32);
        vec[OpCode::Pop as usize] = Some(VM::opcode_pop as fn(&mut VM) -> i32);
        vec[OpCode::ToggleMode as usize] = Some(VM::opcode_togglemode as fn(&mut VM) -> i32);
        vec[OpCode::PrintStack as usize] = Some(VM::opcode_printstack as fn(&mut VM) -> i32);
        vec[OpCode::ToFunction as usize] = Some(VM::opcode_tofunction as fn(&mut VM) -> i32);
        vec[OpCode::Import as usize] = Some(VM::opcode_import as fn(&mut VM) -> i32);
        vec[OpCode::Clone as usize] = Some(VM::opcode_clone as fn(&mut VM) -> i32);
        vec[OpCode::IsBool as usize] = Some(VM::opcode_is_bool as fn(&mut VM) -> i32);
        vec[OpCode::IsInt as usize] = Some(VM::opcode_is_int as fn(&mut VM) -> i32);
        vec[OpCode::IsBigInt as usize] = Some(VM::opcode_is_bigint as fn(&mut VM) -> i32);
        vec[OpCode::IsStr as usize] = Some(VM::opcode_is_str as fn(&mut VM) -> i32);
        vec[OpCode::IsFlt as usize] = Some(VM::opcode_is_flt as fn(&mut VM) -> i32);
        vec[OpCode::BigInt as usize] = Some(VM::opcode_bigint as fn(&mut VM) -> i32);
        vec
    };
    static ref RE_NOT_PARAMS: Regex = Regex::new("\\\\/[a-z]+$").unwrap();
    static ref RE_CAPTURE_PARAMS: Regex = Regex::new("/([a-z]+)$").unwrap();
    static ref RE_ESCAPED_SLASH: Regex = Regex::new("\\\\/").unwrap();
    static ref RE_NEWLINE: Regex = Regex::new("\n").unwrap();
    static ref RE_ERROR_PART: Regex = Regex::new(".*error:\\s*").unwrap();
}

impl VM {
    pub fn new(
        print_stack: bool,
        debug: bool,
        global_vars: Rc<RefCell<HashMap<String, Value>>>,
    ) -> VM {
        let ltz = iana_time_zone::get_timezone().unwrap();
        VM {
            debug,
            stack: Vec::new(),
            last_stack: Vec::new(),
            local_var_stack: Rc::new(RefCell::new(Vec::new())),
            print_stack,
            printing_stack: false,
            scopes: vec![global_vars],
            global_functions: HashMap::new(),
            call_stack_chunks: Vec::new(),
            running: Arc::new(AtomicBool::new(true)),
            chunk: Rc::new(RefCell::new(Chunk::new_standard("unused".to_string()))),
            i: 0,
            sys: System::new_all(),
            regexes: HashMap::new(),
            local_tz: chrono_tz::Tz::from_str(&ltz).unwrap(),
            utc_tz: chrono_tz::Tz::from_str("UTC").unwrap(),
            readline: None,
        }
    }

    /// Takes a chunk, an instruction index, and an error message as its
    /// arguments.  Prints the error message, including filename, line number
    /// and column number elements (if applicable).
    pub fn print_error(&self, error: &str) {
        let point = self.chunk.borrow().get_point(self.i);
        let name = &self.chunk.borrow().name;
        let error_start = if name == "(main)" {
            String::new()
        } else {
            format!("{}:", name)
        };
        match point {
            Some((line, col)) => {
                eprintln!("{}{}:{}: {}", error_start, line, col, error);
            }
            _ => {
                eprintln!("{}{}", error_start, error);
            }
        }
    }

    /// Toggles whether the stack is printed and cleared on command
    /// execution when running interactively.
    pub fn opcode_togglemode(&mut self) -> i32 {
        self.print_stack = !self.print_stack;
        1
    }

    /// Prints the stack.
    pub fn opcode_printstack(&mut self) -> i32 {
        let res = self.print_stack(self.chunk.clone(), self.i, true);
        if res {
            1
        } else {
            0
        }
    }

    /// Converts a callable (e.g. a string) into a function object.
    /// Calling funcall on the function object will be faster than
    /// calling it on the original string.
    pub fn opcode_tofunction(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("to-function requires one argument");
            return 0;
        }
        let fn_rr = self.stack.pop().unwrap();
        let backup_rr = fn_rr.clone();
        let fn_opt: Option<&str>;
        to_str!(fn_rr, fn_opt);

        let mut pushed = false;
        if let Some(s) = fn_opt {
            let sv = self.string_to_callable(s);
            if let Some(v) = sv {
                self.stack.push(v);
                pushed = true;
            }
        }
        if !pushed {
            self.stack.push(backup_rr);
        }
        1
    }

    /// Return a generator over the shell history.  If not being run
    /// in shell context, returns an error message.
    pub fn core_history(&mut self) -> i32 {
        let hist_gen = Value::HistoryGenerator(Rc::new(RefCell::new(0)));
        self.stack.push(hist_gen);
        return 1;
    }

    /// Push the elements from the last stack (i.e. the stack as at
    /// the conclusion of the last call) onto the stack.
    pub fn core_last(&mut self) -> i32 {
        for e in self.last_stack.iter() {
            self.stack.push(e.clone());
        }
        self.last_stack.clear();
        return 1;
    }

    /// Import the functions from the specified path into the current
    /// context.
    pub fn opcode_import(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("import requires one argument");
            return 0;
        }

        let lib_rr = self.stack.pop().unwrap();
        let lib_str_opt: Option<&str>;
        to_str!(lib_rr, lib_str_opt);

        match lib_str_opt {
            Some(s) => {
                let mut compiler = Compiler::new();
                let import_chunk_opt = compiler.deserialise(s);
                match import_chunk_opt {
                    Some(import_chunk) => {
                        for (k, v) in import_chunk.functions.iter() {
                            if !k.starts_with("anon") {
                                self.global_functions.insert(k.clone(), v.clone());
                            }
                        }
                        /* The main reason for running the chunk is so
                         * that global variables are introduced. */
                        self.run(Rc::new(RefCell::new(import_chunk)));
                        self.stack.clear();
                    }
                    None => {
                        let file_res = std::fs::File::open(s);
                        match file_res {
                            Ok(file) => {
                                let mut bufread: Box<dyn BufRead> = Box::new(BufReader::new(file));
                                let mut vm =
                                    VM::new(true, false, Rc::new(RefCell::new(HashMap::new())));
                                let functions = Rc::new(RefCell::new(HashMap::new()));
                                let chunk_opt = vm.interpret(functions, &mut bufread, s);
                                match chunk_opt {
                                    Some(chunk) => {
                                        for (k, v) in chunk.borrow().functions.iter() {
                                            if !k.starts_with("anon") {
                                                self.global_functions.insert(k.clone(), v.clone());
                                            }
                                        }
                                        self.run(chunk);
                                        self.stack.clear();
                                    }
                                    None => {
                                        self.print_error("unable to load import path");
                                        return 0;
                                    }
                                }
                            }
                            Err(_) => {
                                self.print_error("unable to open import path");
                                return 0;
                            }
                        }
                    }
                }
            }
            _ => {
                self.print_error("import argument must be a string");
                return 0;
            }
        }
        1
    }

    /// Takes all of the elements from the generator at the top of the
    /// stack, instantiates a list containing those elements, removes
    /// the generator from the stack, and places the new list at the
    /// top of the stack.
    pub fn generator_to_list(&mut self) -> i32 {
        let mut lst = VecDeque::new();
        loop {
            let dup_res = self.opcode_dup();
            if dup_res == 0 {
                return 0;
            }
            let shift_res = self.opcode_shift();
            if shift_res == 0 {
                return 0;
            }
            let element_rr = self.stack.pop().unwrap();
            match element_rr {
                Value::Null => {
                    self.stack.pop();
                    break;
                }
                _ => {
                    lst.push_back(element_rr);
                }
            }
        }
        self.stack.push(Value::List(Rc::new(RefCell::new(lst))));
        1
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
            Some(value_rr)
        } else {
            let value_opt: Option<&str>;
            to_str!(value_rr, value_opt);

            value_opt.map(|s| {
                Value::String(Rc::new(RefCell::new(StringTriple::new(
                    s.to_string(),
                    None,
                ))))
            })
        }
    }

    /// Takes a string and converts it into a regex.
    pub fn str_to_regex(&self, s_arg: &str) -> Option<(Regex, bool)> {
        let mut global = false;
        let mut s: &str = s_arg;
        let mut s_replacement: String;
        let mut params: HashSet<char> = HashSet::new();

        /* If the last slash is an escape, then the parts afterwards
         * are not flags. */
        if !RE_NOT_PARAMS.is_match(s) {
            let params_res = RE_CAPTURE_PARAMS.captures(s);
            params = match params_res {
                Some(params) => {
                    s_replacement = RE_CAPTURE_PARAMS.replace_all(s, "").to_string();
                    s = &s_replacement;
                    let param_str = params.get(1).unwrap().as_str();
                    param_str.chars().collect()
                }
                _ => HashSet::new(),
            };
        }

        s_replacement = RE_ESCAPED_SLASH.replace_all(s, "/").to_string();
        s = &s_replacement;

        /* The case_insensitive call here is to make rb a &mut
         * RegexBuilder, so that the 'rb = rb...' parts work. */
        let mut rb_init = RegexBuilder::new(s);
        let mut rb = rb_init.case_insensitive(false);
        if params.contains(&'i') {
            rb = rb.case_insensitive(true);
        }
        if params.contains(&'m') {
            rb = rb.multi_line(true);
        }
        if params.contains(&'s') {
            rb = rb.dot_matches_new_line(true);
        }
        if params.contains(&'g') {
            global = true;
        }

        let regex_res = rb.build();
        match regex_res {
            Ok(regex) => Some((regex, global)),
            Err(e) => {
                let mut err_str = format!("{}", e);
                err_str = RE_NEWLINE.replace_all(&err_str, "").to_string();
                err_str = RE_ERROR_PART.replace(&err_str, "").to_string();
                err_str = format!("invalid regex: {}", err_str);
                self.print_error(&err_str);
                None
            }
        }
    }

    /// Takes a value, converts it into a string, and then generates a
    /// regex from that string and returns it.
    pub fn gen_regex(&mut self, value_rr: Value) -> Option<(Rc<Regex>, bool)> {
        if let Value::String(st) = value_rr {
            if let Some(r) = &st.borrow().regex {
                return Some(r.clone());
            }
            let regex_res = self.str_to_regex(&st.borrow().escaped_string);
            match regex_res {
                Some((regex, global)) => {
                    let rc = Rc::new(regex);
                    st.borrow_mut().regex = Some((rc.clone(), global));
                    return Some((rc, global));
                }
                _ => {
                    return None;
                }
            }
        }

        let value_opt: Option<&str>;
        to_str!(value_rr, value_opt);

        match value_opt {
            Some(s) => {
                let rr = self.regexes.get(s);
                match rr {
                    Some(r) => Some(r.clone()),
                    _ => {
                        let regex_res = self.str_to_regex(s);
                        match regex_res {
                            Some((regex, global)) => {
                                let rc = Rc::new(regex);
                                self.regexes.insert(s.to_string(), (rc.clone(), global));
                                Some((rc, global))
                            }
                            _ => None,
                        }
                    }
                }
            }
            _ => {
                self.print_error("regex must be a string");
                None
            }
        }
    }

    /// Call a generator chunk.
    pub fn call_generator(&mut self, call_chunk: Rc<RefCell<Chunk>>) -> bool {
        let mut gen_args = Vec::new();
        let req_arg_count = call_chunk.borrow().req_arg_count;
        if self.stack.len() < req_arg_count.try_into().unwrap() {
            let err_str = format!(
                "generator requires {} argument{}",
                req_arg_count,
                if req_arg_count > 1 { "s" } else { "" }
            );
            self.print_error(&err_str);
            return false;
        }
        let mut arg_count = call_chunk.borrow().arg_count;
        if arg_count != 0 {
            while arg_count > 0 && !self.stack.is_empty() {
                gen_args.push(self.stack.pop().unwrap());
                arg_count -= 1;
            }
        }
        if gen_args.is_empty() {
            gen_args.push(Value::Null);
        }
        let mut gen_call_stack_chunks = Vec::new();
        for i in self.call_stack_chunks.iter() {
            gen_call_stack_chunks.push((*i).clone());
        }
        gen_call_stack_chunks.push((self.chunk.clone(), self.i));
        let gen_rr = Value::Generator(Rc::new(RefCell::new(GeneratorObject::new(
            /* Get a deep copy of the current local variable stack,
             * because a shallow copy will have all the variables
             * popped off before the generator can be called, in some
             * cases. */
            Rc::new(RefCell::new(self.local_var_stack.borrow().clone())),
            0,
            call_chunk,
            gen_call_stack_chunks,
            gen_args,
        ))));
        self.stack.push(gen_rr);
        true
    }

    /// Call a non-generator chunk.
    pub fn call_non_generator(
        &mut self,
        plvs_stack: Option<Rc<RefCell<Vec<Value>>>>,
        call_chunk: Rc<RefCell<Chunk>>,
    ) -> bool {
        if call_chunk.borrow().has_vars {
            self.scopes.push(Rc::new(RefCell::new(HashMap::new())));
        }

        let mut prev_stack = None;

        let has_plvs_stack = match plvs_stack {
            Some(plvs) => {
                prev_stack = Some(self.local_var_stack.clone());
                self.local_var_stack = plvs;
                true
            }
            None => false,
        };
        if !has_plvs_stack && !call_chunk.borrow().nested {
            prev_stack = Some(self.local_var_stack.clone());
            self.local_var_stack = Rc::new(RefCell::new(vec![]));
        }

        let res = self.run(call_chunk.clone());

        if has_plvs_stack || !call_chunk.borrow().nested {
            self.local_var_stack = prev_stack.unwrap();
        }

        res != 0
    }

    /// Call a named function.
    pub fn call_named_function(
        &mut self,
        plvs_stack: Option<Rc<RefCell<Vec<Value>>>>,
        call_chunk: Rc<RefCell<Chunk>>,
    ) -> bool {
        if call_chunk.borrow().is_generator {
            self.call_generator(call_chunk)
        } else {
            self.call_non_generator(plvs_stack, call_chunk)
        }
    }

    /// Convert a string into a callable object, by looking at the
    /// built-in forms and the current functions that are in scope.
    pub fn string_to_callable(&mut self, s: &str) -> Option<Value> {
        let sf_fn_opt = SIMPLE_FORMS.get(s);
        if let Some(sf_fn) = sf_fn_opt {
            let nv = Value::CoreFunction(*sf_fn);
            return Some(nv);
        }

        let global_function;
        let mut call_chunk_opt = None;

        let scb = self.chunk.borrow();
        if scb.functions.contains_key(s) {
            let call_chunk = scb.functions.get(s).unwrap();
            call_chunk_opt = Some(call_chunk.clone());
        }
        if call_chunk_opt.is_none() {
            for (sf, _) in self.call_stack_chunks.iter().rev() {
                let sfb = sf.borrow();
                if sfb.functions.contains_key(s) {
                    let call_chunk = sfb.functions.get(s).unwrap();
                    call_chunk_opt = Some(call_chunk.clone());
                    break;
                }
            }
        }
        if call_chunk_opt.is_none() && self.global_functions.contains_key(s) {
            global_function = self.global_functions.get(s).unwrap().clone();
            call_chunk_opt = Some(global_function);
        }
        if let Some(call_chunk) = call_chunk_opt {
            let nv = Value::NamedFunction(call_chunk);
            return Some(nv);
        }

        None
    }

    /// Attempt to call the function associated with the argument
    /// string.  If no function with that name can be found, and the
    /// call is implicit, then just put the function name back onto
    /// the stack and return.
    pub fn call_string(&mut self, is_implicit: bool, s: &str) -> bool {
        let sv = self.string_to_callable(s);
        match sv {
            Some(Value::CoreFunction(sf_fn)) => {
                let n = sf_fn(self);
                if n == 0 {
                    return false;
                }
                return true;
            }
            Some(Value::NamedFunction(named_fn)) => {
                let res = self.call_named_function(None, named_fn);
                return res;
            }
            _ => {}
        }

        if is_implicit {
            let value_rr = Value::String(Rc::new(RefCell::new(StringTriple::new(
                s.to_string(),
                None,
            ))));
            self.stack.push(value_rr);
        } else {
            self.print_error("function not found");
            return false;
        }

        true
    }

    /// Attempts to set the constant value at the given index with the
    /// callable object for the argument function string.
    pub fn populate_constant_value(&mut self, function_str: &str, function_str_index: i32) {
        let not_present;
        {
            let cfb = &self.chunk.borrow().constant_values;
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
            let sv = self.string_to_callable(function_str);
            if let Some(v) = sv {
                self.chunk
                    .borrow_mut()
                    .constant_values
                    .resize(function_str_index as usize, Value::Null);
                self.chunk
                    .borrow_mut()
                    .constant_values
                    .insert(function_str_index as usize, v);
            }
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
    pub fn call(&mut self, call_opcode: OpCode, function_rr: Value) -> bool {
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

        match function_rr {
            Value::Command(s, params) => {
                let i2 = self.core_command(&s, (*params).clone());
                if i2 == 0 {
                    return false;
                }
            }
            Value::CommandUncaptured(s) => {
                let i2 = self.core_command_uncaptured(&s);
                if i2 == 0 {
                    return false;
                }
            }
            Value::CoreFunction(cf) => {
                let n = cf(self);
                if n == 0 {
                    return false;
                }
                return true;
            }
            Value::NamedFunction(call_chunk_rc) => {
                return self.call_named_function(None, call_chunk_rc);
            }
            Value::AnonymousFunction(call_chunk_rc, lvs) => {
                return self.call_named_function(Some(lvs), call_chunk_rc);
            }
            Value::String(st) => {
                let s = &st.borrow().string;
                return self.call_string(is_implicit, s);
            }
            _ => {
                if is_implicit {
                    self.stack.push(function_rr.clone());
                } else {
                    self.print_error("function not found");
                    return false;
                }
            }
        }

        true
    }

    /// Run the bytecode associated with the given chunk.
    pub fn run(&mut self, chunk: Rc<RefCell<Chunk>>) -> usize {
        self.call_stack_chunks.push((self.chunk.clone(), self.i));
        self.chunk = chunk;
        self.i = 0;
        let res = self.run_inner();
        if res == 0 {
            return 0;
        }
        let mp = self.call_stack_chunks.pop().unwrap();
        let (c, i) = mp;
        self.chunk = c;
        self.i = i;
        res
    }

    /// Takes the global functions, the call stack chunks, the current
    /// chunk, the values for the current chunk, the instruction
    /// index, the global variables for the current generator (if
    /// applicable), the local variables for the current generator (if
    /// applicable), the previous local variable stacks, and the
    /// current line and column number as its arguments.  Runs the
    /// code from the chunk, beginning at the specified instruction
    /// index.
    pub fn run_inner(&mut self) -> usize {
        let mut i = self.i;
        let chunk = self.chunk.clone();

        let mut list_count = 0;
        let mut list_indexes = Vec::new();
        let mut list_types = Vec::new();

        while i < chunk.borrow().data.len() {
            self.i = i;
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
            if let Some(op_fn) = op_fn_opt {
                let res = op_fn(self);
                if res == 0 {
                    return 0;
                } else {
                    i += 1;
                    continue;
                }
            }
            match op {
                OpCode::AddConstant => {
                    i += 1;
                    let i_upper = chunk.borrow().data[i];
                    i += 1;
                    let i_lower = chunk.borrow().data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);
                    let mut done = false;
                    if chunk.borrow().has_constant_int(i2 as i32) {
                        let n = chunk.borrow().get_constant_int(i2 as i32);
                        let len = self.stack.len();
                        let v1_rr = self.stack.get_mut(len - 1).unwrap();
                        if let Value::Int(ref mut n1) = v1_rr {
                            *n1 += n;
                            done = true;
                        };
                    }
                    if !done {
                        let op_fn_opt = SIMPLE_OPS[OpCode::Add as usize];
                        self.stack.push(chunk.borrow().get_constant(i2 as i32));
                        let op_fn = op_fn_opt.unwrap();
                        self.i = i;
                        let res = op_fn(self);
                        if res == 0 {
                            return 0;
                        } else {
                            i += 1;
                            continue;
                        }
                    }
                }
                OpCode::SubtractConstant => {
                    i += 1;
                    let i_upper = chunk.borrow().data[i];
                    i += 1;
                    let i_lower = chunk.borrow().data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);
                    let mut done = false;
                    if chunk.borrow().has_constant_int(i2 as i32) {
                        let n = chunk.borrow().get_constant_int(i2 as i32);
                        let len = self.stack.len();
                        let v1_rr = self.stack.get_mut(len - 1).unwrap();
                        if let Value::Int(ref mut n1) = v1_rr {
                            *n1 -= n;
                            done = true;
                        }
                    }
                    if !done {
                        let op_fn_opt = SIMPLE_OPS[OpCode::Subtract as usize];
                        self.stack.push(chunk.borrow().get_constant(i2 as i32));
                        let op_fn = op_fn_opt.unwrap();
                        self.i = i;
                        let res = op_fn(self);
                        if res == 0 {
                            return 0;
                        } else {
                            i += 1;
                            continue;
                        }
                    }
                }
                OpCode::MultiplyConstant => {
                    i += 1;
                    let i_upper = chunk.borrow().data[i];
                    i += 1;
                    let i_lower = chunk.borrow().data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);
                    let mut done = false;
                    if chunk.borrow().has_constant_int(i2 as i32) {
                        let n = chunk.borrow().get_constant_int(i2 as i32);
                        let len = self.stack.len();
                        let v1_rr = self.stack.get_mut(len - 1).unwrap();
                        if let Value::Int(ref mut n1) = v1_rr {
                            *n1 *= n;
                            done = true;
                        }
                    }
                    if !done {
                        let op_fn_opt = SIMPLE_OPS[OpCode::Multiply as usize];
                        self.stack.push(chunk.borrow().get_constant(i2 as i32));
                        let op_fn = op_fn_opt.unwrap();
                        self.i = i;
                        let res = op_fn(self);
                        if res == 0 {
                            return 0;
                        } else {
                            i += 1;
                            continue;
                        }
                    }
                }
                OpCode::DivideConstant => {
                    i += 1;
                    let i_upper = chunk.borrow().data[i];
                    i += 1;
                    let i_lower = chunk.borrow().data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);
                    let mut done = false;
                    if chunk.borrow().has_constant_int(i2 as i32) {
                        let n = chunk.borrow().get_constant_int(i2 as i32);
                        let len = self.stack.len();
                        let v1_rr = self.stack.get_mut(len - 1).unwrap();
                        if let Value::Int(ref mut n1) = v1_rr {
                            *n1 /= n;
                            done = true;
                        }
                    }
                    if !done {
                        let op_fn_opt = SIMPLE_OPS[OpCode::Divide as usize];
                        self.stack.push(chunk.borrow().get_constant(i2 as i32));
                        let op_fn = op_fn_opt.unwrap();
                        self.i = i;
                        let res = op_fn(self);
                        if res == 0 {
                            return 0;
                        } else {
                            i += 1;
                            continue;
                        }
                    }
                }
                OpCode::EqConstant => {
                    i += 1;
                    let i_upper = chunk.borrow().data[i];
                    i += 1;
                    let i_lower = chunk.borrow().data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);
                    let n = chunk.borrow().get_constant_int(i2 as i32);

                    let len = self.stack.len();
                    let v1_rr = self.stack.get_mut(len - 1).unwrap();
                    let mut done = false;
                    if let Value::Int(ref n1) = v1_rr {
                        self.stack[len - 1] = Value::Bool(*n1 == n);
                        done = true;
                    };
                    if !done {
                        let op_fn_opt = SIMPLE_OPS[OpCode::Eq as usize];
                        self.stack.push(chunk.borrow().get_constant(i2 as i32));
                        let op_fn = op_fn_opt.unwrap();
                        self.i = i;
                        let res = op_fn(self);
                        if res == 0 {
                            return 0;
                        }
                    }
                }
                OpCode::StartList => {
                    list_indexes.push(self.stack.len());
                    list_types.push(ListType::List);
                    list_count += 1;
                }
                OpCode::StartHash => {
                    list_indexes.push(self.stack.len());
                    list_types.push(ListType::Hash);
                    list_count += 1;
                }
                OpCode::StartSet => {
                    list_indexes.push(self.stack.len());
                    list_types.push(ListType::Set);
                    list_count += 1;
                }
                OpCode::EndList => {
                    if list_count == 0 {
                        self.print_error("no start list found");
                        return 0;
                    }
                    let list_index = list_indexes.pop().unwrap();
                    let list_type = list_types.pop().unwrap();
                    list_count -= 1;

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
                                let key_str_opt: Option<&str>;
                                to_str!(key_rr, key_str_opt);
                                map.insert(key_str_opt.unwrap().to_string(), value_rr);
                            }
                            self.stack.push(Value::Hash(Rc::new(RefCell::new(map))));
                        }
                        ListType::Set => {
                            let mut map = IndexMap::new();
                            let mut value = None;
                            while self.stack.len() > list_index {
                                let value_rr = self.stack.pop().unwrap();
                                if value.is_none() {
                                    value = Some(value_rr.clone());
                                }

                                /* Disallow set creation for IP
                                 * addresses or IP sets: users should
                                 * just use IP sets in those cases. */
                                match value_rr {
                                    Value::IpSet(_)
                                    | Value::Ipv4(_)
                                    | Value::Ipv6(_)
                                    | Value::Ipv4Range(_)
                                    | Value::Ipv6Range(_) => {
                                        self.print_error(
                                            "cannot create sets over IP address objects (see ips)",
                                        );
                                        return 0;
                                    }
                                    _ => {}
                                }

                                let value_str_opt: Option<&str>;
                                to_str!(value_rr.clone(), value_str_opt);
                                match value_str_opt {
                                    None => {
                                        self.print_error("value cannot be added to set");
                                        return 0;
                                    }
                                    Some(s) => {
                                        if let Some(ref vv) = value {
                                            if !value_rr.variants_equal(vv) {
                                                self.print_error(
                                                    "set values must have the same type",
                                                );
                                                return 0;
                                            }
                                        }
                                        map.insert(s.to_string(), value_rr);
                                    }
                                }
                            }
                            map.reverse();
                            self.stack.push(Value::Set(Rc::new(RefCell::new(map))));
                        }
                    }
                }
                OpCode::Function => {
                    i += 1;
                    let i_upper = chunk.borrow().data[i];
                    i += 1;
                    let i_lower = chunk.borrow().data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);
                    let value_rr = chunk.borrow().get_constant(i2 as i32);
                    let mut copy = false;

                    match value_rr {
                        Value::String(ref st) => {
                            let s = &st.borrow().string;
                            let cfb = &chunk.borrow().constant_values;
                            match cfb.get(i2 as usize) {
                                Some(Value::String(_)) => {
                                    self.stack.push(Value::AnonymousFunction(
                                        chunk.borrow().functions.get(s).unwrap().clone(),
                                        self.local_var_stack.clone(),
                                    ))
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
                            Value::String(ref st) => self.stack.push(Value::AnonymousFunction(
                                chunk
                                    .borrow()
                                    .functions
                                    .get(&st.borrow().string.to_string())
                                    .unwrap()
                                    .clone(),
                                self.local_var_stack.clone(),
                            )),
                            _ => {
                                eprintln!("unexpected function value!");
                                std::process::abort();
                            }
                        }
                    }
                }
                OpCode::Constant => {
                    i += 1;
                    let i_upper = chunk.borrow().data[i];
                    i += 1;
                    let i_lower = chunk.borrow().data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);
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
                    i += 1;
                    let i_upper = chunk.borrow().data[i];
                    i += 1;
                    let i_lower = chunk.borrow().data[i];
                    let i2 = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);

                    let value_sd = chunk.borrow().constants[i2 as usize].clone();
                    match value_sd {
                        ValueSD::String(st, _) => {
                            self.i = i;

                            /* todo: the two lookups here may be affecting
                             * performance. */
                            let fsi = (i2 as u32).try_into().unwrap();
                            self.populate_constant_value(&st, fsi);
                            let cv = self.chunk.borrow().get_constant_value(fsi);
                            if let Value::Null = cv {
                                match op {
                                    OpCode::CallImplicitConstant => {
                                        let value_rr = Value::String(Rc::new(RefCell::new(
                                            StringTriple::new(st.to_string(), None),
                                        )));
                                        self.stack.push(value_rr);
                                        i += 1;
                                        continue;
                                    }
                                    _ => {
                                        self.print_error("function not found");
                                        return 0;
                                    }
                                }
                            }

                            let res = self.call(op, cv);
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
                    if self.stack.is_empty() {
                        self.print_error("call requires one argument");
                        return 0;
                    }

                    let function_rr = self.stack.pop().unwrap();

                    let res = self.call(op, function_rr);
                    if !res {
                        return 0;
                    }
                }
                OpCode::GLVCall => {
                    i += 1;
                    let var_index: u8 = chunk.borrow().data[i];

                    let function_rr = self
                        .local_var_stack
                        .borrow()
                        .index(var_index as usize)
                        .clone();

                    let res = self.call(OpCode::Call, function_rr);
                    if !res {
                        return 0;
                    }
                }
                OpCode::SetLocalVar => {
                    if self.stack.is_empty() {
                        self.print_error("! requires one argument");
                        return 0;
                    }

                    i += 1;
                    let var_index: u8 = chunk.borrow().data[i];
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
                    i += 1;
                    let var_index: u8 = chunk.borrow().data[i];

                    if usize::from(var_index + 1) > self.local_var_stack.borrow().len() {
                        /* It's not impossible this is due to some
                         * other bug or error, but in general it
                         * should be due to this problem. */
                        self.print_error("anonymous function environment has gone out of scope");
                        return 0;
                    }

                    let value_rr = self
                        .local_var_stack
                        .borrow()
                        .index(var_index as usize)
                        .clone();
                    self.stack.push(value_rr);
                }
                OpCode::GLVShift => {
                    i += 1;
                    self.i = i;
                    let var_index: u8 = chunk.borrow().data[i];

                    let mut pt = self
                        .local_var_stack
                        .borrow()
                        .index(var_index as usize)
                        .clone();
                    let i2 = self.opcode_shift_inner(&mut pt);
                    if i2 == 0 {
                        return 0;
                    }
                }
                OpCode::PopLocalVar => {
                    self.local_var_stack.borrow_mut().pop();
                }
                OpCode::Var => {
                    if self.stack.is_empty() {
                        self.print_error("var requires one argument");
                        return 0;
                    }

                    let var_name;
                    {
                        let var_name_rr = self.stack.pop().unwrap();
                        match var_name_rr {
                            Value::String(st) => {
                                var_name = st.borrow().string.clone().to_string();
                            }
                            _ => {
                                self.print_error("variable name must be a string");
                                return 0;
                            }
                        }
                    }

                    let var_name_ref = &var_name;
                    let has_existing_var = {
                        let last_scope = self.scopes.last().unwrap().borrow();
                        let existing_var = last_scope.get(var_name_ref);
                        match existing_var {
                            Some(_) => true,
                            _       => false
                        }
                    };
                    if has_existing_var {
                        self.print_error(
                            "variable has already been declared in this scope"
                        );
                        return 0;
                    }

                    let mut last_scope =
                        self.scopes.last_mut().unwrap().borrow_mut();
                    last_scope.insert(var_name.to_string(), Value::Int(0));
                }
                OpCode::SetVar => {
                    if self.stack.len() < 2 {
                        self.print_error("! requires two arguments");
                        return 0;
                    }

                    let var_name_rr = self.stack.pop().unwrap();
                    let value_rr = self.stack.pop().unwrap();

                    match var_name_rr {
                        Value::String(st) => {
                            let mut done = false;
                            let s = &st.borrow().string;

                            for scope in self.scopes.iter_mut().rev() {
                                if scope.borrow().contains_key(s) {
                                    scope.borrow_mut().insert(s.to_string(), value_rr.clone());
                                    done = true;
                                    break;
                                }
                            }

                            if !done {
                                self.print_error("could not find variable");
                                return 0;
                            }
                        }
                        _ => {
                            self.print_error("variable name must be a string");
                            return 0;
                        }
                    }
                }
                OpCode::GetVar => {
                    if self.stack.is_empty() {
                        self.print_error("@ requires one argument");
                        return 0;
                    }

                    let var_name_rr = self.stack.pop().unwrap();
                    match var_name_rr {
                        Value::String(st) => {
                            let mut done = false;
                            let s = &st.borrow().string;

                            for scope in self.scopes.iter().rev() {
                                if scope.borrow().contains_key(s) {
                                    self.stack.push(scope.borrow().get(s).unwrap().clone());
                                    done = true;
                                    break;
                                }
                            }
                            if !done {
                                self.print_error("could not find variable");
                                return 0;
                            }
                        }
                        _ => {
                            self.print_error("variable name must be a string");
                            return 0;
                        }
                    }
                }
                OpCode::JumpNe => {
                    if self.stack.is_empty() {
                        self.print_error("conditional requires one argument");
                        return 0;
                    }

                    let value_rr = self.stack.pop().unwrap();

                    i += 1;
                    let i1: usize = chunk.borrow().data[i].try_into().unwrap();
                    i += 1;
                    let i2: usize = chunk.borrow().data[i].try_into().unwrap();
                    let jmp_len: usize = (i1 << 8) | i2;

                    let b = value_rr.to_bool();
                    if !b {
                        i += jmp_len;
                    }
                }
                OpCode::JumpNeR => {
                    if self.stack.is_empty() {
                        self.print_error("conditional requires one argument");
                        return 0;
                    }

                    let value_rr = self.stack.pop().unwrap();

                    i += 1;
                    let i1: usize = chunk.borrow().data[i].try_into().unwrap();
                    i += 1;
                    let i2: usize = chunk.borrow().data[i].try_into().unwrap();
                    let jmp_len: usize = (i1 << 8) | i2;

                    let b = value_rr.to_bool();
                    if !b {
                        i -= jmp_len;
                    }
                }
                OpCode::JumpNeREqC => {
                    if self.stack.is_empty() {
                        self.print_error("conditional requires one argument");
                        return 0;
                    }

                    i += 1;
                    let i1: usize = chunk.borrow().data[i].try_into().unwrap();
                    i += 1;
                    let i2: usize = chunk.borrow().data[i].try_into().unwrap();
                    let jmp_len: usize = (i1 << 8) | i2;

                    i += 1;
                    let i_upper = chunk.borrow().data[i];
                    i += 1;
                    let i_lower = chunk.borrow().data[i];
                    let i3 = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);
                    let value_rr = self.stack.last().unwrap();
                    if chunk.borrow().has_constant_int(i3 as i32) {
                        let cmp_rr = chunk.borrow().get_constant_int(i3 as i32);

                        match value_rr {
                            Value::Int(n2) => {
                                if cmp_rr != *n2 {
                                    i -= jmp_len;
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
                    let i2 = self.opcode_shift();
                    if i2 == 0 {
                        return 0;
                    }
                }
                OpCode::Yield => {
                    if !chunk.borrow().is_generator {
                        eprintln!("yield can only be called within generator");
                        return 0;
                    }
                    return i + 1;
                }
                OpCode::Jump => {
                    i += 1;
                    let i1: usize = chunk.borrow().data[i].try_into().unwrap();
                    i += 1;
                    let i2: usize = chunk.borrow().data[i].try_into().unwrap();
                    let jmp_len: usize = (i1 << 8) | i2;
                    i += jmp_len;
                }
                OpCode::JumpR => {
                    i += 1;
                    let i1: usize = chunk.borrow().data[i].try_into().unwrap();
                    i += 1;
                    let i2: usize = chunk.borrow().data[i].try_into().unwrap();
                    let jmp_len: usize = (i1 << 8) | i2;
                    i -= jmp_len;
                }
                OpCode::Error => {
                    if self.stack.is_empty() {
                        self.print_error("error requires one argument");
                        return 0;
                    }

                    let line;
                    let col;
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

                    let error_rr = self.stack.pop().unwrap();
                    let error_str_opt: Option<&str>;
                    to_str!(error_rr, error_str_opt);

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
            i += 1;
        }

        if list_count > 0 {
            print_error(chunk, i, "unterminated list start");
            return 0;
        }

        if self.print_stack {
            self.last_stack.clear();
            let res = self.print_stack(chunk, i, false);
            self.stack.clear();
            if !res {
                return 0;
            }
        }

        i + 1
    }

    /// Takes the global functions and the file to read the program
    /// code from as its arguments.  Compiles the program code and
    /// executes it, returning the chunk (if compiled successfully).
    pub fn interpret(
        &mut self,
        global_functions: Rc<RefCell<HashMap<String, Rc<RefCell<Chunk>>>>>,
        fh: &mut Box<dyn BufRead>,
        name: &str,
    ) -> Option<Rc<RefCell<Chunk>>> {
        for (k, v) in global_functions.borrow().iter() {
            self.global_functions.insert(k.clone(), v.clone());
        }

        let mut compiler = Compiler::new();
        let chunk_opt = compiler.compile(fh, name);
        if chunk_opt.is_none() {
            return None;
        }
        let chunk = Rc::new(RefCell::new(chunk_opt.unwrap()));

        self.run(chunk.clone());
        if self.print_stack {
            self.stack.clear();
        }
        Some(chunk)
    }
}

use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::convert::TryInto;
use std::fs::File;
use std::fs::ReadDir;
use std::io::BufReader;
use std::io::LineWriter;
use std::rc::Rc;
use std::str;

use indexmap::IndexMap;
use num::FromPrimitive;
use num::ToPrimitive;
use num_bigint::BigInt;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::ChildStdout;

use opcode::{to_opcode, OpCode};

/// A chunk is a parsed/processed piece of code.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Chunk {
    /// The name of the chunk.  Either the name of the file that
    /// contains the associated code, or "(main)", for code entered at
    /// the REPL.
    pub name: String,
    /// The bytecode for the chunk.
    pub data: RefCell<Vec<u8>>,
    /// The line and column number information for the chunk.  The
    /// entries in this vector correspond to the entries in the
    /// bytecode vector.
    pub points: RefCell<Vec<(u32, u32)>>,
    /// The set of constant values for the chunk.
    pub constants: Vec<ValueSD>,
    /// The functions defined within the chunk.
    pub functions: RefCell<HashMap<String, Chunk>>,
    /// Whether the chunk is for a generator function.
    pub is_generator: bool,
    /// Whether the chunk deals with global variables.
    pub has_vars: bool,
    /// Whether the chunk deals with local variables.
    pub uses_local_vars: bool,
    /// The maximum argument count for a generator function
    /// (only set if is_generator is true).
    pub arg_count: i32,
    /// The required argument count for a generator function
    /// (only set if is_generator is true).
    pub req_arg_count: i32,
    /// Whether the chunk represents a nested function.
    pub nested: bool,
    /// The scope depth for the chunk.
    pub scope_depth: u32,
}

/// The core value type used by the compiler and VM.
#[derive(Debug)]
pub enum Value {
    /// Used to indicate that a generator is exhausted.
    Null,
    /// 32-bit integer.
    Int(i32),
    /// Unbounded integer.
    BigInt(num_bigint::BigInt),
    /// Floating-point number.
    Float(f64),
    /// String.  The second part here is the regex object that
    /// corresponds to the string, which is generated and cached
    /// when the string is used as a regex.
    String(String, Option<Regex>),
    /// An external command (wrapped in curly brackets), where the
    /// output is captured.
    Command(String),
    /// An external command (begins with $), where the output is not
    /// captured.
    CommandUncaptured(String),
    /// A list.
    List(VecDeque<Rc<RefCell<Value>>>),
    /// A hash.
    Hash(IndexMap<String, Rc<RefCell<Value>>>),
    /// An anonymous function that refers to a local stack, where the
    /// second value is the local variable stack index and the third
    /// value is a unique identifier for that stack (currently its
    /// pointer value).
    Function(String, u32, u64),
    /// A generator constructed by way of a generator function.
    Generator(
        /// The global variable state.
        HashMap<String, Rc<RefCell<Value>>>,
        /// The local variable stack.
        Vec<Rc<RefCell<Value>>>,
        /// The current instruction index.
        usize,
        /// The chunk of the associated generator function.
        Chunk,
        /// The chunks of the other functions in the call stack.
        Vec<Chunk>,
        /// The values that need to be passed into the generator when
        /// it is first called.
        Vec<Rc<RefCell<Value>>>,
        /// A hash of cached values for the chunk of the associated
        /// generator function.
        HashMap<String, Rc<RefCell<Value>>>,
    ),
    /// A generator for getting the output of a Command.
    CommandGenerator(BufReader<ChildStdout>),
    /// A generator over the keys of a hash.
    KeysGenerator(usize, Rc<RefCell<Value>>),
    /// A generator over the values of a hash.
    ValuesGenerator(usize, Rc<RefCell<Value>>),
    /// A generator over key-value pairs (lists) of a hash.
    EachGenerator(usize, Rc<RefCell<Value>>),
    /// A file reader value.
    FileReader(BufReader<File>),
    /// A file writer value.
    FileWriter(LineWriter<File>),
    /// A directory handle.
    DirectoryHandle(ReadDir),
}

/// An enum for the Value types that can be serialised and
/// deserialised (i.e. those that can be stored as constants in a
/// chunk).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueSD {
    Null,
    Int(i32),
    Float(f64),
    BigInt(String),
    String(String),
    Command(String),
    CommandUncaptured(String),
}

/// Takes a chunk, an instruction index, and an error message as its
/// arguments.  Prints the error message, including filename, line number
/// and column number elements (if applicable).
pub fn print_error(chunk: &Chunk, i: usize, error: &str) {
    let point = chunk.get_point(i);
    let name = &chunk.name;
    let error_start = if name == "(main)" {
        format!("")
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

impl Chunk {
    /// Construct a standard (non-generator) chunk.
    pub fn new_standard(name: String) -> Chunk {
        Chunk {
            name: name,
            data: RefCell::new(Vec::new()),
            points: RefCell::new(Vec::new()),
            constants: Vec::new(),
            functions: RefCell::new(HashMap::new()),
            is_generator: false,
            has_vars: true,
            uses_local_vars: false,
            arg_count: 0,
            req_arg_count: 0,
            nested: false,
            scope_depth: 0,
        }
    }

    /// Construct a generator chunk.
    pub fn new_generator(
        name: String, arg_count: i32, req_arg_count: i32
    ) -> Chunk {
        Chunk {
            name: name,
            data: RefCell::new(Vec::new()),
            points: RefCell::new(Vec::new()),
            constants: Vec::new(),
            functions: RefCell::new(HashMap::new()),
            is_generator: true,
            has_vars: true,
            uses_local_vars: false,
            arg_count: arg_count,
            req_arg_count: req_arg_count,
            nested: false,
            scope_depth: 0,
        }
    }

    /// Add a constant to the current chunk, and return its index in
    /// the constants list (for later calls to `get_constant`).
    pub fn add_constant(&mut self, value_rr: Rc<RefCell<Value>>) -> i32 {
        let value_rrb = value_rr.borrow();
        let value_sd = match &*value_rrb {
            Value::Null => ValueSD::Null,
            Value::Int(n) => ValueSD::Int(*n),
            Value::Float(n) => ValueSD::Float(*n),
            Value::BigInt(n) => ValueSD::BigInt(n.to_str_radix(10)),
            Value::String(s, _) => ValueSD::String(s.to_string()),
            Value::Command(s) => ValueSD::Command(s.to_string()),
            Value::CommandUncaptured(s) => {
                ValueSD::CommandUncaptured(s.to_string())
            }
            _ => {
                eprintln!("constant type cannot be added to chunk! {:?}",
                          value_rrb);
                std::process::abort();
            }
        };
        self.constants.push(value_sd);
        return (self.constants.len() - 1) as i32;
    }

    /// Get a constant from the current chunk.
    pub fn get_constant(&self, i: i32) -> Rc<RefCell<Value>> {
        let value_sd = &self.constants[i as usize];
        let value = match value_sd {
            ValueSD::Null => Value::Null,
            ValueSD::Int(n) => Value::Int(*n),
            ValueSD::Float(n) => Value::Float(*n),
            ValueSD::BigInt(n) => {
                let nn = n.parse::<num_bigint::BigInt>().unwrap();
                Value::BigInt(nn)
            }
            ValueSD::String(s) => Value::String(s.to_string(), None),
            ValueSD::Command(s) => Value::Command(s.to_string()),
            ValueSD::CommandUncaptured(s) => {
                Value::CommandUncaptured(s.to_string())
            }
        };
        return Rc::new(RefCell::new(value));
    }

    /// Add an opcode to the current chunk's data.
    pub fn add_opcode(&mut self, opcode: OpCode) {
        self.data.borrow_mut().push(opcode as u8);
    }

    /// Get the last opcode from the current chunk's data.
    pub fn get_last_opcode(&self) -> OpCode {
        return to_opcode(*self.data.borrow().last().unwrap());
    }

    /// Get the second-last opcode from the current chunk's data.
    /// Defaults to `OpCode::Call`, if the chunk does not have at
    /// least two opcodes.  Used for adding implicit call opcodes, if
    /// required.
    pub fn get_second_last_opcode(&self) -> OpCode {
        if self.data.borrow().len() < 2 {
            return OpCode::Call;
        }
        return to_opcode(
            *self
                .data
                .borrow()
                .get(self.data.borrow().len() - 2)
                .unwrap(),
        );
    }

    /// Add a raw byte to the current chunk's data.
    pub fn add_byte(&mut self, byte: u8) {
        self.data.borrow_mut().push(byte);
    }

    /// Remove the last byte from the current chunk's data.
    pub fn pop_byte(&mut self) {
        self.data.borrow_mut().pop();
    }

    /// Get the last byte from the current chunk's data.
    pub fn get_last_byte(&self) -> u8 {
        return *self.data.borrow().last().unwrap();
    }

    /// Get the chunk's most recently-added constant.
    pub fn get_last_constant(&mut self) -> Rc<RefCell<Value>> {
        return self
            .get_constant((self.constants.len() - 1).try_into().unwrap());
    }

    /// Set the line and column number data for the most
    /// recently-added opcode/byte.  If any of the preceding
    /// opcodes/bytes do not have point data, set the point data for
    /// those opcodes/bytes by using the most recently-added point
    /// data (putting aside the current call), too.
    pub fn set_next_point(&mut self, line_number: u32, column_number: u32) {
        let data_len = self.data.borrow().len();
        let mut points_b = self.points.borrow_mut();
        let points_len = points_b.len();
        let mut prev_line_number = 0;
        let mut prev_column_number = 0;
        if points_len > 0 {
            let last_point = points_b.get(points_len - 1).unwrap();
            let (prev_line_number_p, prev_column_number_p) = last_point;
            prev_line_number = *prev_line_number_p;
            prev_column_number = *prev_column_number_p;
        }
        while points_b.len() < data_len {
            points_b.push((prev_line_number, prev_column_number));
        }
        points_b.insert(data_len, (line_number, column_number));
    }

    /// Get the point data for the given index.
    pub fn get_point(&self, i: usize) -> Option<(u32, u32)> {
        let points_b = self.points.borrow();
        let point = points_b.get(i);
        match point {
            Some((0, 0)) => None,
            Some((_, _)) => Some(*(point.unwrap())),
            _            => None
        }
    }

    /// Print the disassembly for the current chunk to standard
    /// output.
    pub fn disassemble(&self, name: &str) {
        println!("== {} ==", name);
        let data_b = self.data.borrow();

        let mut i = 0;
        while i < data_b.len() {
            let opcode = to_opcode(data_b[i]);
            print!("{:^4} ", i);
            match opcode {
                OpCode::Constant => {
                    i = i + 1;
                    let i_upper = data_b[i];
                    i = i + 1;
                    let i_lower = data_b[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00)
                        | ((i_lower & 0xFF) as u16);
                    let value = self.get_constant(constant_i as i32);
                    println!("OP_CONSTANT {:?}", value);
                }
                OpCode::Add => {
                    println!("OP_ADD");
                }
                OpCode::Subtract => {
                    println!("OP_SUBTRACT");
                }
                OpCode::Multiply => {
                    println!("OP_MULTIPLY");
                }
                OpCode::Divide => {
                    println!("OP_DIVIDE");
                }
                OpCode::EndFn => {
                    println!("OP_ENDFN");
                }
                OpCode::Call => {
                    println!("OP_CALL");
                }
                OpCode::CallImplicit => {
                    println!("OP_CALLIMPLICIT");
                }
                OpCode::Function => {
                    println!("OP_FUNCTION");
                }
                OpCode::Var => {
                    println!("OP_VAR");
                }
                OpCode::SetVar => {
                    println!("OP_SETVAR");
                }
                OpCode::GetVar => {
                    println!("OP_GETVAR");
                }
                OpCode::SetLocalVar => {
                    i = i + 1;
                    let var_i = data_b[i];
                    println!("OP_SETLOCALVAR {}", var_i);
                }
                OpCode::GetLocalVar => {
                    i = i + 1;
                    let var_i = data_b[i];
                    println!("OP_GETLOCALVAR {}", var_i);
                }
                OpCode::PopLocalVar => {
                    println!("OP_POPLOCALVAR");
                }
                OpCode::Jump => {
                    i = i + 1;
                    let i1: usize = data_b[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = data_b[i].try_into().unwrap();
                    let jump_i: usize = (i1 << 8) | i2;
                    println!("OP_JUMP {:?}", jump_i);
                }
                OpCode::JumpNe => {
                    i = i + 1;
                    let i1: usize = data_b[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = data_b[i].try_into().unwrap();
                    let jump_i: usize = (i1 << 8) | i2;
                    println!("OP_JUMPNE {:?}", jump_i);
                }
                OpCode::JumpNeR => {
                    i = i + 1;
                    let i1: usize = data_b[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = data_b[i].try_into().unwrap();
                    let jump_i: usize = (i1 << 8) | i2;
                    println!("OP_JUMPNER {:?}", jump_i);
                }
                OpCode::Eq => {
                    println!("OP_EQ");
                }
                OpCode::Gt => {
                    println!("OP_GT");
                }
                OpCode::Lt => {
                    println!("OP_LT");
                }
                OpCode::Print => {
                    println!("OP_PRINT");
                }
                OpCode::Dup => {
                    println!("OP_DUP");
                }
                OpCode::Swap => {
                    println!("OP_SWAP");
                }
                OpCode::Drop => {
                    println!("OP_DROP");
                }
                OpCode::Rot => {
                    println!("OP_ROT");
                }
                OpCode::Over => {
                    println!("OP_OVER");
                }
                OpCode::Depth => {
                    println!("OP_DEPTH");
                }
                OpCode::Clear => {
                    println!("OP_CLEAR");
                }
                OpCode::StartList => {
                    println!("OP_STARTLIST");
                }
                OpCode::EndList => {
                    println!("OP_ENDLIST");
                }
                OpCode::StartHash => {
                    println!("OP_STARTHASH");
                }
                OpCode::Shift => {
                    println!("OP_SHIFT");
                }
                OpCode::Yield => {
                    println!("OP_YIELD");
                }
                OpCode::IsNull => {
                    println!("OP_ISNULL");
                }
                OpCode::IsList => {
                    println!("OP_ISLIST");
                }
                OpCode::IsCallable => {
                    println!("OP_ISCALLABLE");
                }
                OpCode::IsShiftable => {
                    println!("OP_ISSHIFTABLE");
                }
                OpCode::Open => {
                    println!("OP_OPEN");
                }
                OpCode::Readline => {
                    println!("OP_READLINE");
                }
                OpCode::Error => {
                    println!("OP_ERROR");
                }
                OpCode::Return => {
                    println!("OP_RETURN");
                }
                OpCode::Str => {
                    println!("OP_STR");
                }
                OpCode::Int => {
                    println!("OP_INT");
                }
                OpCode::Flt => {
                    println!("OP_FLT")
                }
                OpCode::Rand => {
                    println!("OP_RAND")
                }
                OpCode::Unknown => {
                    println!("(Unknown)");
                }
            }
            i = i + 1;
        }

        for (k, v) in self.functions.borrow().iter() {
            println!("== {}.{} ==", name, k);
            v.disassemble(k);
        }
    }
}

impl Value {
    /// Convert the current value into a string.  If the current value
    /// is a string, this will return the &str as the first element of
    /// the pair.  Otherwise, it will return a new String as the
    /// second element of the pair.  If the value is not representable
    /// as a string, both elements of the pair will be None.
    pub fn to_string(&self) -> (Option<&str>, Option<String>) {
        match self {
            Value::String(s, _) => (Some(s), None),
            Value::Int(n) => {
                let s = format!("{}", n);
                (None, Some(s))
            }
            Value::BigInt(n) => {
                let s = format!("{}", n);
                (None, Some(s))
            }
            Value::Float(f) => {
                let s = format!("{}", f);
                (None, Some(s))
            }
            Value::Null => (Some(&""), None),
            _ => (None, None),
        }
    }

    /// Convert the current value into an i32.  If the value is not
    /// representable as an i32, the result will be None.
    pub fn to_int(&self) -> Option<i32> {
        match self {
            Value::Int(n) => Some(*n),
            Value::BigInt(n) => n.to_i32(),
            Value::Float(f) => Some(*f as i32),
            Value::String(s, _) => {
                let n_r = s.parse::<i32>();
                match n_r {
                    Ok(n) => {
                        return Some(n);
                    }
                    _ => {
                        return None;
                    }
                }
            }
            Value::Null => Some(0),
            _ => None,
        }
    }

    /// Convert the current value into a bigint.  If the value is not
    /// representable as a bigint, the result will be None.
    pub fn to_bigint(&self) -> Option<BigInt> {
        match self {
            Value::Int(n) => Some(BigInt::from_i32(*n).unwrap()),
            Value::BigInt(n) => Some(n.clone()),
            Value::Float(f) => Some(BigInt::from_i32(*f as i32).unwrap()),
            Value::String(s, _) => {
                let n_r = s.to_string().parse::<num_bigint::BigInt>();
                match n_r {
                    Ok(n) => {
                        return Some(n);
                    }
                    _ => {
                        return None;
                    }
                }
            }
            Value::Null => Some(BigInt::from_i32(0).unwrap()),
            _ => None,
        }
    }

    /// Convert the current value into a floating-point number (f64).
    /// If the value is not representable in that type, the result
    /// will be None.
    pub fn to_float(&self) -> Option<f64> {
        match self {
            Value::Int(n) => Some(*n as f64),
            Value::BigInt(n) => Some(n.to_f64().unwrap()),
            Value::Float(f) => Some(*f),
            Value::String(s, _) => {
                let n_r = s.parse::<f64>();
                match n_r {
                    Ok(n) => {
                        return Some(n);
                    }
                    _ => {
                        return None;
                    }
                }
            }
            Value::Null => Some(0.0),
            _ => None,
        }
    }

    /// For a string value, generate the corresponding regex for the
    /// string value and store it in the value.  If called on a
    /// non-string value, this will abort the current process.
    pub fn gen_regex(&mut self, chunk: &Chunk, i: usize) -> bool {
        match self {
            Value::String(s, ref mut current_regex) => {
                let regex_res = Regex::new(s);
                match regex_res {
                    Ok(regex) => {
                        *current_regex = Some(regex.clone());
                        return true;
                    }
                    Err(e) => {
                        let mut err_str = format!("{}", e);
                        let regex_nl = Regex::new("\n").unwrap();
                        err_str = regex_nl.replace_all(&err_str, "").to_string();
                        let regex_errpart = Regex::new(".*error:\\s*").unwrap();
                        err_str = regex_errpart.replace(&err_str, "").to_string();
                        err_str = format!("invalid regex: {}", err_str);
                        print_error(chunk, i, &err_str);
                        return false;
                    }
                }
            }
            _ => {
                eprintln!("unable to make regex from non-string!");
                std::process::abort();
            }
        }
    }

    /// For a string value, return the corresponding regex.  For this
    /// function to work, gen_regex must have been called on the
    /// string value beforehand.
    pub fn to_regex(&self) -> Option<&Regex> {
        match self {
            Value::String(_, Some(ref regex)) => Some(regex),
            _ => {
                eprintln!("gen_regex must be called before to_regex!");
                std::process::abort();
            }
        }
    }
}

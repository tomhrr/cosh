use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::convert::TryInto;
use std::fmt;
use std::fs::File;
use std::fs::ReadDir;
use std::io::BufReader;
use std::io::BufWriter;
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
use vm::VM;

/// A chunk is a parsed/processed piece of code.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Chunk {
    /// The name of the chunk.  Either the name of the file that
    /// contains the associated code, or "(main)", for code entered at
    /// the REPL.
    pub name: String,
    /// The bytecode for the chunk.
    pub data: Vec<u8>,
    /// The line and column number information for the chunk.  The
    /// entries in this vector correspond to the entries in the
    /// bytecode vector.
    pub points: Vec<(u32, u32)>,
    /// The set of constant values for the chunk.
    pub constants: Vec<ValueSD>,
    /// The functions defined within the chunk.
    pub functions: HashMap<String, Rc<RefCell<Chunk>>>,
    #[serde(skip)]
    pub constant_values: Vec<Value>,
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

/// A string paired with its regex, to save regenerating that regex
/// repeatedly.
#[derive(Debug, Clone)]
pub struct StringPair {
    pub s: String,
    pub r: Option<Regex>,
}

impl StringPair {
    pub fn new(s: String, r: Option<Regex>) -> StringPair {
        StringPair { s: s, r: r }
    }
}

/// A generator object, containing a generator chunk along with all of
/// its associated state.
#[derive(Debug, Clone)]
pub struct GeneratorObject {
    /// The global variable state.
    pub global_vars: Rc<RefCell<HashMap<String, Value>>>,
    /// The local variable stack.
    pub local_vars_stack: Rc<RefCell<Vec<Value>>>,
    /// The current instruction index.
    pub index: usize,
    /// The chunk of the associated generator function.
    pub chunk: Rc<RefCell<Chunk>>,
    /// The chunks of the other functions in the call stack.
    pub call_stack_chunks: Vec<(Rc<RefCell<Chunk>>, usize)>,
    /// The values that need to be passed into the generator when
    /// it is first called.
    pub gen_args: Vec<Value>,
}

impl GeneratorObject {
    /// Construct a generator object.
    pub fn new(
        global_vars: Rc<RefCell<HashMap<String, Value>>>,
        local_vars_stack: Rc<RefCell<Vec<Value>>>,
        index: usize,
        chunk: Rc<RefCell<Chunk>>,
        call_stack_chunks: Vec<(Rc<RefCell<Chunk>>, usize)>,
        gen_args: Vec<Value>
    ) -> GeneratorObject {
        GeneratorObject {
            global_vars: global_vars,
            local_vars_stack: local_vars_stack,
            index: index,
            chunk: chunk,
            call_stack_chunks: call_stack_chunks,
            gen_args: gen_args,
        }
    }
}

/// A hash object paired with its current index, for use within
/// the various hash generators.
#[derive(Debug)]
pub struct HashWithIndex {
    pub i: usize,
    pub h: Value,
}

impl HashWithIndex {
    pub fn new(i: usize, h: Value) -> HashWithIndex {
        HashWithIndex { i: i, h: h }
    }
}

/// The core value type used by the compiler and VM.
#[derive(Clone)]
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
    String(Rc<RefCell<StringPair>>),
    /// An external command (wrapped in curly brackets), where the
    /// output is captured.
    Command(Rc<RefCell<String>>),
    /// An external command (begins with $), where the output is not
    /// captured.
    CommandUncaptured(Rc<RefCell<String>>),
    /// A list.
    List(Rc<RefCell<VecDeque<Value>>>),
    /// A hash.
    Hash(Rc<RefCell<IndexMap<String, Value>>>),
    /// An anonymous function (includes reference to local variable
    /// stack).
    AnonymousFunction(Rc<RefCell<Chunk>>, Rc<RefCell<Vec<Value>>>),
    /// A core function.  See SIMPLE_FORMS in the VM.
    CoreFunction(fn(&mut VM) -> i32),
    /// A named function.
    NamedFunction(Rc<RefCell<Chunk>>),
    /// A generator constructed by way of a generator function.
    Generator(Rc<RefCell<GeneratorObject>>),
    /// A generator for getting the output of a Command.
    CommandGenerator(Rc<RefCell<BufReader<ChildStdout>>>),
    /// A generator over the keys of a hash.
    KeysGenerator(Rc<RefCell<HashWithIndex>>),
    /// A generator over the values of a hash.
    ValuesGenerator(Rc<RefCell<HashWithIndex>>),
    /// A generator over key-value pairs (lists) of a hash.
    EachGenerator(Rc<RefCell<HashWithIndex>>),
    /// A file reader value.
    FileReader(Rc<RefCell<BufReader<File>>>),
    /// A file writer value.
    FileWriter(Rc<RefCell<BufWriter<File>>>),
    /// A directory handle.
    DirectoryHandle(Rc<RefCell<ReadDir>>),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Null => {
                write!(f, "Null")
            }
            Value::Int(i) => {
                write!(f, "{}", i)
            }
            Value::BigInt(i) => {
                write!(f, "{}", i)
            }
            Value::Float(i) => {
                write!(f, "{}", i)
            }
            Value::String(s) => {
                let ss = &s.borrow().s;
                write!(f, "\"{}\"", ss)
            }
            Value::Command(s) => {
                write!(f, "Command \"{}\"", s.borrow())
            }
            Value::CommandUncaptured(s) => {
                write!(f, "CommandUncaptured \"{}\"", s.borrow())
            }
            Value::List(ls) => {
                write!(f, "{:?}", ls)
            }
            Value::Hash(hs) => {
                write!(f, "{:?}", hs)
            }
            Value::AnonymousFunction(_, _) => {
                write!(f, "((Function))")
            }
            Value::CoreFunction(_) => {
                write!(f, "((CoreFunction))")
            }
            Value::NamedFunction(_) => {
                write!(f, "((NamedFunction))")
            }
            Value::Generator(_) => {
                write!(f, "((Generator))")
            }
            Value::CommandGenerator(_) => {
                write!(f, "((CommandGenerator))")
            }
            Value::KeysGenerator(_) => {
                write!(f, "((KeysGenerator))")
            }
            Value::ValuesGenerator(_) => {
                write!(f, "((ValuesGenerator))")
            }
            Value::EachGenerator(_) => {
                write!(f, "((EachGenerator))")
            }
            Value::FileReader(_) => {
                write!(f, "((FileReader))")
            }
            Value::FileWriter(_) => {
                write!(f, "((FileWriter))")
            }
            Value::DirectoryHandle(_) => {
                write!(f, "((DirectoryHandle))")
            }
        }
    }
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
pub fn print_error(chunk: Rc<RefCell<Chunk>>, i: usize, error: &str) {
    let point = chunk.borrow().get_point(i);
    let name = &chunk.borrow().name;
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
            data: Vec::new(),
            points: Vec::new(),
            constants: Vec::new(),
            functions: HashMap::new(),
            is_generator: false,
            has_vars: true,
            uses_local_vars: false,
            arg_count: 0,
            req_arg_count: 0,
            nested: false,
            scope_depth: 0,
            constant_values: Vec::new(),
        }
    }

    /// Construct a generator chunk.
    pub fn new_generator(name: String, arg_count: i32, req_arg_count: i32) -> Chunk {
        Chunk {
            name: name,
            data: Vec::new(),
            points: Vec::new(),
            constants: Vec::new(),
            functions: HashMap::new(),
            is_generator: true,
            has_vars: true,
            uses_local_vars: false,
            arg_count: arg_count,
            req_arg_count: req_arg_count,
            nested: false,
            scope_depth: 0,
            constant_values: Vec::new(),
        }
    }

    /// Add a constant to the current chunk, and return its index in
    /// the constants list (for later calls to `get_constant`).
    pub fn add_constant(&mut self, value_rr: Value) -> i32 {
        let value_sd = match value_rr {
            Value::Null => ValueSD::Null,
            Value::Int(n) => ValueSD::Int(n),
            Value::Float(n) => ValueSD::Float(n),
            Value::BigInt(n) => ValueSD::BigInt(n.to_str_radix(10)),
            Value::String(sp) => ValueSD::String(sp.borrow().s.to_string()),
            Value::Command(s) => ValueSD::Command(s.borrow().to_string()),
            Value::CommandUncaptured(s) => ValueSD::CommandUncaptured(s.borrow().to_string()),
            _ => {
                eprintln!("constant type cannot be added to chunk! {:?}", value_rr);
                std::process::abort();
            }
        };
        self.constants.push(value_sd);
        return (self.constants.len() - 1) as i32;
    }

    /// Get a constant from the current chunk.
    pub fn get_constant(&self, i: i32) -> Value {
        let value_sd = &self.constants[i as usize];
        let value = match value_sd {
            ValueSD::Null => Value::Null,
            ValueSD::Int(n) => Value::Int(*n),
            ValueSD::Float(n) => Value::Float(*n),
            ValueSD::BigInt(n) => {
                let nn = n.parse::<num_bigint::BigInt>().unwrap();
                Value::BigInt(nn)
            }
            ValueSD::String(sp) => {
                Value::String(Rc::new(RefCell::new(StringPair::new(sp.to_string(), None))))
            }
            ValueSD::Command(s) => Value::Command(Rc::new(RefCell::new(s.to_string()))),
            ValueSD::CommandUncaptured(s) => {
                Value::CommandUncaptured(Rc::new(RefCell::new(s.to_string())))
            }
        };
        return value;
    }

    pub fn get_constant_value(&self, i: i32) -> Value {
        let value = self.constant_values.get(i as usize);
        match value {
            Some(v) => { return v.clone(); }
            _ => { return Value::Null; }
        }
    }

    /// Get a constant int value from the current chunk.
    pub fn get_constant_int(&self, i: i32) -> i32 {
        let value_sd = &self.constants[i as usize];
        let value = match *value_sd {
            ValueSD::Int(n) => n,
            _ => 0,
        };
        return value;
    }

    /// Check whether the chunk has a constant int value at the
    /// specified index.
    pub fn has_constant_int(&self, i: i32) -> bool {
        let value_sd = &self.constants[i as usize];
        return match *value_sd {
            ValueSD::Int(_) => true,
            _ => false,
        };
    }

    /// Add an opcode to the current chunk's data.
    pub fn add_opcode(&mut self, opcode: OpCode) {
        self.data.push(opcode as u8);
    }

    /// Get the last opcode from the current chunk's data.
    pub fn get_last_opcode(&self) -> OpCode {
        return to_opcode(*self.data.last().unwrap());
    }

    /// Get the second-last opcode from the current chunk's data.
    /// Defaults to `OpCode::Call`, if the chunk does not have at
    /// least two opcodes.  Used for adding implicit call opcodes, if
    /// required.
    pub fn get_second_last_opcode(&self) -> OpCode {
        if self.data.len() < 2 {
            return OpCode::Call;
        }
        return to_opcode(
            *self
                .data
                .get(self.data.len() - 2)
                .unwrap(),
        );
    }

    /// Get the third-last opcode from the current chunk's data.
    pub fn get_third_last_opcode(&self) -> OpCode {
        if self.data.len() < 3 {
            return OpCode::Call;
        }
        return to_opcode(
            *self
                .data
                .get(self.data.len() - 3)
                .unwrap(),
        );
    }

    /// Get the fourth-last opcode from the current chunk's data.
    pub fn get_fourth_last_opcode(&self) -> OpCode {
        if self.data.len() < 4 {
            return OpCode::Call;
        }
        return to_opcode(
            *self
                .data
                .get(self.data.len() - 4)
                .unwrap(),
        );
    }

    /// Set the second-last opcode for the current chunk's data.
    pub fn set_second_last_opcode(&mut self, opcode: OpCode) {
        let len = self.data.len();
        if let Some(el) = self.data.get_mut(len - 2) {
            *el = opcode as u8;
        }
    }

    /// Set the third-last opcode for the current chunk's data.
    pub fn set_third_last_opcode(&mut self, opcode: OpCode) {
        let len = self.data.len();
        if let Some(el) = self.data.get_mut(len - 3) {
            *el = opcode as u8;
        }
    }

    /// Set the fourth-last opcode for the current chunk's data.
    pub fn set_fourth_last_opcode(&mut self, opcode: OpCode) {
        let len = self.data.len();
        if let Some(el) = self.data.get_mut(len - 4) {
            *el = opcode as u8;
        }
    }

    /// Set the last opcode for the current chunk's data.
    pub fn set_last_opcode(&mut self, opcode: OpCode) {
        let len = self.data.len();
        if let Some(el) = self.data.get_mut(len - 1) {
            *el = opcode as u8;
        }
    }

    /// Add a raw byte to the current chunk's data.
    pub fn add_byte(&mut self, byte: u8) {
        self.data.push(byte);
    }

    /// Remove the last byte from the current chunk's data.
    pub fn pop_byte(&mut self) {
        self.data.pop();
    }

    /// Get the last byte from the current chunk's data.
    pub fn get_last_byte(&self) -> u8 {
        return *self.data.last().unwrap();
    }

    /// Get the second-last byte from the current chunk's data.
    pub fn get_second_last_byte(&self) -> u8 {
        if self.data.len() < 2 {
            return 0;
        }
        return *self
            .data
            .get(self.data.len() - 2)
            .unwrap();
    }

    /// Get the third-last byte from the current chunk's data.
    pub fn get_third_last_byte(&self) -> u8 {
        if self.data.len() < 3 {
            return 0;
        }
        return *self
            .data
            .get(self.data.len() - 3)
            .unwrap();
    }

    /// Set the last byte for the current chunk's data.
    pub fn set_last_byte(&mut self, byte: u8) {
        let len = self.data.len();
        if let Some(el) = self.data.get_mut(len - 1) {
            *el = byte;
        }
    }

    /// Set the second-last byte for the current chunk's data.
    pub fn set_second_last_byte(&mut self, byte: u8) {
        let len = self.data.len();
        if let Some(el) = self.data.get_mut(len - 2) {
            *el = byte;
        }
    }

    /// Set the third-last byte for the current chunk's data.
    pub fn set_third_last_byte(&mut self, byte: u8) {
        let len = self.data.len();
        if let Some(el) = self.data.get_mut(len - 3) {
            *el = byte;
        }
    }

    /// Get the chunk's most recently-added constant.
    pub fn get_last_constant(&mut self) -> Value {
        return self.get_constant((self.constants.len() - 1).try_into().unwrap());
    }

    /// Set the line and column number data for the most
    /// recently-added opcode/byte.  If any of the preceding
    /// opcodes/bytes do not have point data, set the point data for
    /// those opcodes/bytes by using the most recently-added point
    /// data (putting aside the current call), too.
    pub fn set_next_point(&mut self, line_number: u32, column_number: u32) {
        let data_len = self.data.len();
        let points_len = self.points.len();
        let mut prev_line_number = 0;
        let mut prev_column_number = 0;
        if points_len > 0 {
            let last_point = self.points.get(points_len - 1).unwrap();
            let (prev_line_number_p, prev_column_number_p) = last_point;
            prev_line_number = *prev_line_number_p;
            prev_column_number = *prev_column_number_p;
        }
        while self.points.len() < data_len {
            self.points.push((prev_line_number, prev_column_number));
        }
        self.points.insert(data_len, (line_number, column_number));
    }

    /// Get the point data for the given index.
    pub fn get_point(&self, i: usize) -> Option<(u32, u32)> {
        let point = self.points.get(i);
        match point {
            Some((0, 0)) => None,
            Some((_, _)) => Some(*(point.unwrap())),
            _ => None,
        }
    }

    /// Reset the values for a previous point.  Required where
    /// existing data is being replaced/adjusted during compilation.
    pub fn set_previous_point(&mut self, i: usize, line_number: u32, column_number: u32) {
        let point = self.points.get_mut(i);
        match point {
            Some((ref mut a, ref mut b)) => {
                *a = line_number;
                *b = column_number;
            }
            _ => {
                eprintln!("point not found!");
                std::process::abort();
            }
        }
    }

    /// Print the disassembly for the current chunk to standard
    /// output.
    pub fn disassemble(&self, name: &str) {
        println!("== {} ==", name);

        let mut i = 0;
        while i < self.data.len() {
            let opcode = to_opcode(self.data[i]);
            print!("{:^4} ", i);
            match opcode {
                OpCode::Constant => {
                    i = i + 1;
                    let i_upper = self.data[i];
                    i = i + 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let value = self.get_constant(constant_i as i32);
                    println!("OP_CONSTANT {:?}", value);
                }
                OpCode::AddConstant => {
                    i = i + 1;
                    let i_upper = self.data[i];
                    i = i + 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let value = self.get_constant(constant_i as i32);
                    println!("OP_ADDCONSTANT {:?}", value);
                }
                OpCode::SubtractConstant => {
                    i = i + 1;
                    let i_upper = self.data[i];
                    i = i + 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let value = self.get_constant(constant_i as i32);
                    println!("OP_SUBTRACTCONSTANT {:?}", value);
                }
                OpCode::DivideConstant => {
                    i = i + 1;
                    let i_upper = self.data[i];
                    i = i + 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let value = self.get_constant(constant_i as i32);
                    println!("OP_DIVIDECONSTANT {:?}", value);
                }
                OpCode::MultiplyConstant => {
                    i = i + 1;
                    let i_upper = self.data[i];
                    i = i + 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let value = self.get_constant(constant_i as i32);
                    println!("OP_MULTIPLYCONSTANT {:?}", value);
                }
                OpCode::EqConstant => {
                    i = i + 1;
                    let i_upper = self.data[i];
                    i = i + 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let value = self.get_constant(constant_i as i32);
                    println!("OP_EQCONSTANT {:?}", value);
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
                    let var_i = self.data[i];
                    println!("OP_SETLOCALVAR {}", var_i);
                }
                OpCode::GetLocalVar => {
                    i = i + 1;
                    let var_i = self.data[i];
                    println!("OP_GETLOCALVAR {}", var_i);
                }
                OpCode::GLVShift => {
                    i = i + 1;
                    let var_i = self.data[i];
                    println!("OP_GLVSHIFT {}", var_i);
                }
                OpCode::GLVCall => {
                    i = i + 1;
                    let var_i = self.data[i];
                    println!("OP_GLVCALL {}", var_i);
                }
                OpCode::PopLocalVar => {
                    println!("OP_POPLOCALVAR");
                }
                OpCode::Jump => {
                    i = i + 1;
                    let i1: usize = self.data[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = self.data[i].try_into().unwrap();
                    let jump_i: usize = (i1 << 8) | i2;
                    println!("OP_JUMP {:?}", jump_i);
                }
                OpCode::JumpR => {
                    i = i + 1;
                    let i1: usize = self.data[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = self.data[i].try_into().unwrap();
                    let jump_i: usize = (i1 << 8) | i2;
                    println!("OP_JUMPR {:?}", jump_i);
                }
                OpCode::JumpNe => {
                    i = i + 1;
                    let i1: usize = self.data[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = self.data[i].try_into().unwrap();
                    let jump_i: usize = (i1 << 8) | i2;
                    println!("OP_JUMPNE {:?}", jump_i);
                }
                OpCode::JumpNeR => {
                    i = i + 1;
                    let i1: usize = self.data[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = self.data[i].try_into().unwrap();
                    let jump_i: usize = (i1 << 8) | i2;
                    println!("OP_JUMPNER {:?}", jump_i);
                }
                OpCode::JumpNeREqC => {
                    i = i + 1;
                    let i1: usize = self.data[i].try_into().unwrap();
                    i = i + 1;
                    let i2: usize = self.data[i].try_into().unwrap();
                    let jump_i: usize = (i1 << 8) | i2;

                    i = i + 1;
                    let i_upper = self.data[i];
                    i = i + 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let value = self.get_constant(constant_i as i32);

                    println!("OP_JUMPNEREQC {:?} {:?}", jump_i, value);
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
                OpCode::Push => {
                    println!("OP_PUSH")
                }
                OpCode::Pop => {
                    println!("OP_POP")
                }
                OpCode::DupIsNull => {
                    println!("OP_DUPISNULL")
                }
                OpCode::ToggleMode => {
                    println!("OP_TOGGLEMODE")
                }
                OpCode::PrintStack => {
                    println!("OP_PRINTSTACK")
                }
                OpCode::ToFunction => {
                    println!("OP_TOFUNCTION")
                }
                OpCode::Import => {
                    println!("OP_IMPORT")
                }
                OpCode::CallConstant => {
                    i = i + 1;
                    let i_upper = self.data[i];
                    i = i + 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let value = self.get_constant(constant_i as i32);
                    println!("OP_CALLCONSTANT {:?}", value);
                }
                OpCode::CallImplicitConstant => {
                    i = i + 1;
                    let i_upper = self.data[i];
                    i = i + 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | ((i_lower & 0xFF) as u16);
                    let value = self.get_constant(constant_i as i32);
                    println!("OP_CALLIMPLICITCONSTANT {:?}", value);
                }
                OpCode::Unknown => {
                    println!("(Unknown)");
                }
            }
            i = i + 1;
        }

        for (k, v) in self.functions.iter() {
            println!("== {}.{} ==", name, k);
            v.borrow().disassemble(k);
        }
    }
}

impl Value {
    /// Convert the current value into a string.  Not intended for use
    /// with Value::String.
    pub fn to_string(&self) -> Option<String> {
        match self {
            Value::String(_) => {
                eprintln!("to_string should not be called with Value::String");
                std::process::exit(1);
            }
            Value::Int(n) => {
                let s = format!("{}", n);
                Some(s)
            }
            Value::BigInt(n) => {
                let s = format!("{}", n);
                Some(s)
            }
            Value::Float(f) => {
                let s = format!("{}", f);
                Some(s)
            }
            Value::Null => Some("".to_string()),
            _ => None,
        }
    }

    /// Convert the current value into an i32.  If the value is not
    /// representable as an i32, the result will be None.
    pub fn to_int(&self) -> Option<i32> {
        match self {
            Value::Int(n) => Some(*n),
            Value::BigInt(n) => n.to_i32(),
            Value::Float(f) => Some(*f as i32),
            Value::String(sp) => {
                let s = &sp.borrow().s;
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
            Value::String(sp) => {
                let s = &sp.borrow().s;
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
            Value::String(sp) => {
                let s = &sp.borrow().s;
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
    pub fn gen_regex(&mut self) -> bool {
        match self {
            Value::String(sp) => {
                match sp.borrow().r {
                    None => {}
                    _ => {
                        return true;
                    }
                }
            },
            _ => {
                eprintln!("unable to make regex from non-string!");
                std::process::abort();
            }
        }
        match self {
            Value::String(sp) => {
                let regex_res = Regex::new(&sp.borrow().s);
                match regex_res {
                    Ok(regex) => {
                        sp.borrow_mut().r = Some(regex.clone());
                        return true;
                    }
                    Err(e) => {
                        let mut err_str = format!("{}", e);
                        let regex_nl = Regex::new("\n").unwrap();
                        err_str = regex_nl.replace_all(&err_str, "").to_string();
                        let regex_errpart = Regex::new(".*error:\\s*").unwrap();
                        err_str = regex_errpart.replace(&err_str, "").to_string();
                        err_str = format!("invalid regex: {}", err_str);
                        //print_error(chunk, i, &err_str);
                        eprintln!("{}", err_str);
                        return false;
                    }
                }
            }
            _ => {}
        }
        return true;
    }
}

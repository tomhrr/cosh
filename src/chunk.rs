use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::convert::TryInto;
use std::fmt;
use std::fs::File;
use std::fs::ReadDir;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Write;
use std::net::{Ipv4Addr, Ipv6Addr, TcpStream};
use std::rc::Rc;
use std::str::FromStr;
use std::thread;
use std::time;

use chrono::format::{parse, Parsed, StrftimeItems};
use chrono::prelude::*;
use indexmap::IndexMap;
use ipnet::{Ipv4Net, Ipv6Net};
use iprange::IpRange;
use nix::{sys::signal::Signal,
          sys::wait::waitpid,
          sys::wait::WaitPidFlag,
          sys::wait::WaitStatus};
use nonblock::NonBlockingReader;
use num::FromPrimitive;
use num::ToPrimitive;
use num_bigint::BigInt;
use num_traits::{Zero, Num};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::{ChildStderr, ChildStdout};
use sqlx::{MySql, Postgres, Sqlite};

use crate::opcode::{to_opcode, OpCode};
use crate::vm::*;

/// A chunk is a parsed/processed piece of code.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Chunk {
    /// The name of the chunk.  Either the name of the file that
    /// contains the associated code, or "(main)", for code entered at
    /// the REPL.
    pub name: String,
    /// The bytecode for the chunk.
    pub data: Vec<u8>,
    /// Whether the byte at this position in data is an opcode.
    pub is_opcode: Vec<bool>,
    /// The line and column number information for the chunk.  The
    /// entries in this vector correspond to the entries in the
    /// bytecode vector.
    pub points: Vec<(u32, u32)>,
    /// The set of constant values for the chunk.
    pub constants: Vec<ValueLiteral>,
    /// The functions defined within the chunk.
    pub functions: HashMap<String, Rc<RefCell<Chunk>>>,
    #[serde(skip)]
    /// Initialised values for the constants of the chunk.
    pub constant_values: Vec<Value>,
    /// Whether the chunk is for a generator function.
    pub is_generator: bool,
    /// Whether the chunk deals with global variables.
    pub has_vars: bool,
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

/// StringTriple is used for the core string type.  It binds together
/// a display string (i.e. a raw string), an escaped string (to save
/// repeating that operation), and the corresponding regex (to save
/// regenerating that regex).  The bool flag indicates whether global
/// matching should be used for the regex.  The display string is the
/// 'real' string, and includes e.g. literal newline characters,
/// whereas the escaped string includes escapes for those characters.
#[derive(Debug, Clone)]
pub struct StringTriple {
    pub string: String,
    pub escaped_string: String,
    pub regex: Option<(Rc<Regex>, bool)>,
}

/// Takes a display string and returns an escaped string.
fn escape_string(s: &str) -> String {
    let mut s2 = String::from("");
    let mut next_escaped = false;
    for c in s.chars() {
        if next_escaped {
            match c {
                '\\' => {
                    s2.push('\\');
                }
                '"' => {
                    s2.push('\\');
                    s2.push('\\');
                    s2.push(c);
                }
                _ => {
                    s2.push('\\');
                    s2.push(c);
                }
            }
            next_escaped = false;
        } else {
            match c {
                '\\' => {
                    next_escaped = true;
                }
                '\n' => {
                    s2.push('\\');
                    s2.push('n');
                }
                '\r' => {
                    s2.push('\\');
                    s2.push('r');
                }
                '\t' => {
                    s2.push('\\');
                    s2.push('t');
                }
                '"' => {
                    s2.push('\\');
                    s2.push('"');
                }
                _ => {
                    s2.push(c);
                }
            }
        }
    }
    s2
}

impl StringTriple {
    pub fn new(s: String, r: Option<(Rc<Regex>, bool)>) -> StringTriple {
        let e = escape_string(&s);
        StringTriple {
            string: s,
            escaped_string: e,
            regex: r,
        }
    }
    pub fn new_with_escaped(s: String, e: String, r: Option<(Rc<Regex>, bool)>) -> StringTriple {
        StringTriple {
            string: s,
            escaped_string: e,
            regex: r,
        }
    }
}

/// A generator object, containing a generator chunk along with all of
/// its associated state.
#[derive(Debug, Clone)]
pub struct GeneratorObject {
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
        local_vars_stack: Rc<RefCell<Vec<Value>>>,
        index: usize,
        chunk: Rc<RefCell<Chunk>>,
        call_stack_chunks: Vec<(Rc<RefCell<Chunk>>, usize)>,
        gen_args: Vec<Value>,
    ) -> GeneratorObject {
        GeneratorObject {
            local_vars_stack,
            index,
            chunk,
            call_stack_chunks,
            gen_args,
        }
    }
}

/// A file BufReader paired with an additional buffer, for dealing
/// with calls to read.
#[derive(Debug)]
pub struct BufReaderWithBuffer<T: Read> {
    pub reader: BufReader<T>,
    pub buffer: [u8; 1024],
    pub buffer_index: i32,
    pub buffer_limit: i32,
}

impl<T: Read> BufReaderWithBuffer<T> {
    pub fn new(reader: BufReader<T>) -> BufReaderWithBuffer<T> {
        BufReaderWithBuffer {
            reader,
            buffer: [0; 1024],
            buffer_index: -1,
            buffer_limit: -1,
        }
    }

    fn fill_buffer(&mut self) -> i32 {
        let n_res = self.reader.read(&mut self.buffer);
        match n_res {
            Ok(n) => {
                if n == 0 {
                    return 0;
                }
                self.buffer_index = 0;
                self.buffer_limit = n as i32;
                return 1;
            }
            Err(_) => {
                return -1;
            }
        }
    }

    pub fn read(&mut self, mut n: usize) -> Option<Value> {
        let mut buflst = VecDeque::new();
        while n > 0 {
            if self.buffer_index > -1 {
                while (self.buffer_index < self.buffer_limit)
                        && (n > 0) {
                    buflst.push_back(
                        Value::Byte(
                            self.buffer[self.buffer_index as usize]
                        )
                    );
                    self.buffer_index += 1;
                    n -= 1;
                }
                if self.buffer_index == self.buffer_limit {
                    self.buffer_index = -1;
                    self.buffer_limit = -1;
                }
            } else {
                let res = self.fill_buffer();
                if res == 0 {
                    break;
                } else if res == -1 {
                    return None;
                }
            }
        }
        if buflst.len() == 0 {
            Some(Value::Null)
        } else {
            Some(Value::List(Rc::new(RefCell::new(buflst))))
        }
    }

    pub fn readline(&mut self) -> Option<Value> {
        if self.buffer_index == -1 {
            let res = self.fill_buffer();
            if res == 0 {
                return Some(Value::Null);
            } else if res == -1 {
                return None;
            }
        }

        let mut i = self.buffer_index;
        let mut found = false;
        while i < self.buffer_limit {
            if self.buffer[i as usize] == '\n' as u8 {
                i += 1;
                found = true;
                break;
            }
            i += 1;
        }
        if found {
            let slice = &self.buffer[self.buffer_index as usize..i as usize];
            let s = String::from_utf8_lossy(slice);
            self.buffer_index = i;
            if self.buffer_index == self.buffer_limit {
                self.buffer_index = -1;
                self.buffer_limit = -1;
            }
            Some(new_string_value(s.to_string()))
        } else {
            let mut sbuf = Vec::new();
            let res = self.reader.read_until('\n' as u8, &mut sbuf);
            match res {
                Ok(_) => {
                    let bufvec =
                        &mut self.buffer[self.buffer_index as usize..
                                         self.buffer_limit as usize].to_vec();
                    bufvec.append(&mut sbuf);
                    let s = String::from_utf8_lossy(bufvec);
                    self.buffer_index = -1;
                    self.buffer_limit = -1;
                    Some(new_string_value(s.to_string()))
                }
                _ => None
            }
        }
    }
}

/// A hash object paired with its current index, for use within
/// the various hash generators.
#[derive(Debug, Clone)]
pub struct HashWithIndex {
    pub i: usize,
    pub h: Value,
}

impl HashWithIndex {
    pub fn new(i: usize, h: Value) -> HashWithIndex {
        HashWithIndex { i, h }
    }
}

/// An IPv4 range object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ipv4Range {
    pub s: Ipv4Addr,
    pub e: Ipv4Addr,
}

impl Ipv4Range {
    pub fn new(s: Ipv4Addr, e: Ipv4Addr) -> Ipv4Range {
        Ipv4Range { s, e }
    }
}

/// An IPv6 range object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ipv6Range {
    pub s: Ipv6Addr,
    pub e: Ipv6Addr,
}

impl Ipv6Range {
    pub fn new(s: Ipv6Addr, e: Ipv6Addr) -> Ipv6Range {
        Ipv6Range { s, e }
    }
}

/// An IP set object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IpSet {
    pub ipv4: IpRange<Ipv4Net>,
    pub ipv6: IpRange<Ipv6Net>,
}

impl IpSet {
    pub fn new(ipv4: IpRange<Ipv4Net>, ipv6: IpRange<Ipv6Net>) -> IpSet {
        IpSet { ipv4, ipv6 }
    }

    pub fn shift(&mut self) -> Value {
	/* todo: not sure how else to implement this, since
	 * the IP range object doesn't order by address.
	 * Could serialise to a vector in the IPSet object,
	 * but that might make other operations less
	 * efficient. */
	let mut ipranges = self.ipv4.iter().collect::<Vec<Ipv4Net>>();
	ipranges.sort_by_key(|a| a.network());
	let next = ipranges.first();
	if let Some(next_value) = next {
	    let mut next_range = IpRange::new();
	    next_range.add(*next_value);
	    let new_set = self.ipv4.exclude(&next_range);
	    self.ipv4 = new_set;
	    return Value::Ipv4(*next_value);
	}
	let mut ipranges2 = self.ipv6.iter().collect::<Vec<Ipv6Net>>();
	ipranges2.sort_by_key(|a| a.network());
	let next2 = ipranges2.first();
	if let Some(next2_value) = next2 {
	    let mut next2_range = IpRange::new();
	    next2_range.add(*next2_value);
	    let new_set = self.ipv6.exclude(&next2_range);
	    self.ipv6 = new_set;
	    return Value::Ipv6(*next2_value);
	}
        return Value::Null;
    }

    pub fn get(&self, n: usize) -> Value {
	let mut ipranges = self.ipv4.iter().collect::<Vec<Ipv4Net>>();
        if ipranges.len() >= n + 1 {
            ipranges.sort_by_key(|a| a.network());
            return Value::Ipv4(*ipranges.get(n).unwrap());
        }
        let ipv6_index = n - ipranges.len();
	let mut ipranges2 = self.ipv6.iter().collect::<Vec<Ipv6Net>>();
        if ipranges2.len() >= ipv6_index + 1 {
            ipranges2.sort_by_key(|a| a.network());
            return Value::Ipv6(*ipranges2.get(ipv6_index).unwrap());
        }
        return Value::Null;
    }
}

/// MySQL database objects.
#[derive(Debug, Clone)]
pub struct DBConnectionMySQL {
    pub pool: sqlx::Pool<MySql>,
}

impl DBConnectionMySQL {
    pub fn new(pool: sqlx::Pool<MySql>) -> DBConnectionMySQL {
        DBConnectionMySQL { pool }
    }
}

#[derive(Debug)]
pub struct DBStatementMySQL {
    pub pool: sqlx::Pool<MySql>,
    pub query: String,
}

impl DBStatementMySQL {
    pub fn new(pool: sqlx::Pool<MySql>, query: String) -> DBStatementMySQL {
        DBStatementMySQL { pool, query }
    }
}

/// PostgreSQL database objects.
#[derive(Debug, Clone)]
pub struct DBConnectionPostgres {
    pub pool: sqlx::Pool<Postgres>,
}

impl DBConnectionPostgres {
    pub fn new(pool: sqlx::Pool<Postgres>) -> DBConnectionPostgres {
        DBConnectionPostgres { pool }
    }
}

#[derive(Debug)]
pub struct DBStatementPostgres {
    pub pool: sqlx::Pool<Postgres>,
    pub query: String,
}

impl DBStatementPostgres {
    pub fn new(pool: sqlx::Pool<Postgres>, query: String) -> DBStatementPostgres {
        DBStatementPostgres { pool, query }
    }
}

/// SQLite database objects.
#[derive(Debug, Clone)]
pub struct DBConnectionSQLite {
    pub pool: sqlx::Pool<Sqlite>,
}

impl DBConnectionSQLite {
    pub fn new(pool: sqlx::Pool<Sqlite>) -> DBConnectionSQLite {
        DBConnectionSQLite { pool }
    }
}

#[derive(Debug)]
pub struct DBStatementSQLite {
    pub pool: sqlx::Pool<Sqlite>,
    pub query: String,
}

impl DBStatementSQLite {
    pub fn new(pool: sqlx::Pool<Sqlite>, query: String) -> DBStatementSQLite {
        DBStatementSQLite { pool, query }
    }
}

/// A channel generator object (see pmap).
#[derive(Debug)]
pub struct ChannelGenerator {
    pub rx: std::fs::File,
    pub pid: nix::unistd::Pid,
    pub input_generator: Value,
    pub finished: bool,
}

impl ChannelGenerator {
    pub fn new(rx: std::fs::File,
               pid: nix::unistd::Pid,
               input_generator: Value) -> ChannelGenerator {
        ChannelGenerator { rx, pid, input_generator, finished: false }
    }
}

/// A command generator object.
pub struct CommandGenerator {
    /* The two pids are stored individually, rather than as a list,
     * because there will only ever be one or two processes that need
     * to be waited on. */
    pub pid: Option<nix::unistd::Pid>,
    pub pid2: Option<nix::unistd::Pid>,
    pub value: Option<Value>,
    pub stdout: NonBlockingReader<ChildStdout>,
    pub stderr: NonBlockingReader<ChildStderr>,
    pub stdout_buffer: Vec<u8>,
    pub stderr_buffer: Vec<u8>,
    get_stdout: bool,
    get_stderr: bool,
    pub get_combined: bool,
    pub get_bytes: bool,
    pub status: Option<i32>,
    dropped: bool,
}

impl CommandGenerator {
    pub fn new(
        pid: Option<nix::unistd::Pid>,
        pid2: Option<nix::unistd::Pid>,
        value: Option<Value>,
        stdout: NonBlockingReader<ChildStdout>,
        stderr: NonBlockingReader<ChildStderr>,
        get_stdout: bool,
        get_stderr: bool,
        get_combined: bool,
        get_bytes: bool,
    ) -> CommandGenerator {
        CommandGenerator {
            pid,
            pid2,
            value,
            stdout,
            stderr,
            stdout_buffer: Vec::new(),
            stderr_buffer: Vec::new(),
            get_stdout,
            get_stderr,
            get_combined,
            get_bytes,
            status: None,
            dropped: false
        }
    }

    /// Determine whether standard output has been exhausted.
    fn stdout_eof(&mut self) -> bool {
        return self.stdout_buffer.is_empty() && self.stdout.is_eof();
    }

    /// Determine whether standard error has been exhausted.
    fn stderr_eof(&mut self) -> bool {
        return self.stderr_buffer.is_empty() && self.stderr.is_eof();
    }

    /// Read a line from standard output (non-blocking).
    fn stdout_read_line_nb(&mut self) -> Option<String> {
        let mut index = self.stdout_buffer.iter().position(|&r| r == b'\n');

        if index.is_none() && !self.stdout.is_eof() {
            let _res = self.stdout.read_available(&mut self.stdout_buffer);
            index = self.stdout_buffer.iter().position(|&r| r == b'\n');
        }
        match index {
            Some(n) => {
                /* todo: may not work if newline falls within
                 * a multibyte Unicode character?  Not sure if this
                 * is possible, though. */
                let new_buf: Vec<u8> = (&mut self.stdout_buffer).drain(0..(n + 1)).collect();
                let new_str =
                    String::from_utf8_lossy(&new_buf).to_string();
                Some(new_str)
            }
            _ => {
                if !self.stdout_buffer.is_empty() && self.stdout.is_eof() {
                    let new_buf: Vec<u8> = (&mut self.stdout_buffer).drain(..).collect();
                    let new_str =
                        String::from_utf8_lossy(&new_buf).to_string();
                    Some(new_str)
                } else {
                    None
                }
            }
        }
    }

    /// Read a line from standard error (non-blocking).
    fn stderr_read_line_nb(&mut self) -> Option<String> {
        let mut index = self.stderr_buffer.iter().position(|&r| r == b'\n');

        if index.is_none() && !self.stderr.is_eof() {
            let _res = self.stderr.read_available(&mut self.stderr_buffer);
            index = self.stderr_buffer.iter().position(|&r| r == b'\n');
        }
        match index {
            Some(n) => {
                /* todo: may not work if newline falls within
                 * a multibyte Unicode character?  Not sure if this
                 * is possible, though. */
                let new_buf: Vec<u8> = (&mut self.stderr_buffer).drain(0..(n + 1)).collect();
                let new_str =
                    String::from_utf8_lossy(&new_buf).to_string();
                Some(new_str)
            }
            _ => {
                if !self.stderr_buffer.is_empty() && self.stderr.is_eof() {
                    let new_buf: Vec<u8> = (&mut self.stderr_buffer).drain(..).collect();
                    let new_str =
                        String::from_utf8_lossy(&new_buf).to_string();
                    Some(new_str)
                } else {
                    None
                }
            }
        }
    }

    /// Read a line from standard output or standard error.  This
    /// blocks until one returns a line.
    pub fn read_line(&mut self) -> Option<String> {
        loop {
            if self.get_stdout {
                let s = self.stdout_read_line_nb();
                if s.is_some() {
                    return s;
                } else if self.stdout_eof() && (!self.get_stderr || self.stderr_eof()) {
                    return None;
                }
            }
            if self.get_stderr {
                let s = self.stderr_read_line_nb();
                if s.is_some() {
                    return s;
                } else if self.stderr_eof() && (!self.get_stdout || self.stdout_eof()) {
                    return None;
                }
            }
        }
    }

    /// Read a line from standard output or standard error, and return
    /// an identifier for the stream and the string.  This blocks
    /// until one returns a line.
    pub fn read_line_combined(&mut self) -> Option<(i32, String)> {
        loop {
            let so = self.stdout_read_line_nb();
            match so {
                Some(ss) => {
                    return Some((1, ss));
                }
                _ => {
                    if self.stdout_eof() && self.stderr_eof() {
                        return None;
                    }
                }
            }

            let se = self.stderr_read_line_nb();
            match se {
                Some(ss) => {
                    return Some((2, ss));
                }
                _ => {
                    if self.stderr_eof() && self.stdout_eof() {
                        return None;
                    }
                }
            }
        }
    }

    /// Read bytes from standard output and return the bytes as a
    /// list.  By default, 1024 bytes are read on each call.
    pub fn read_bytes(&mut self) -> Option<Vec<u8>> {
        /* If get_bytes is set, then this is the only function that's
         * called, and it doesn't make use of the buffer. */
        let mut bytes = Vec::new();
        loop {
            let res_n = self.stdout.read_available(&mut bytes);
            match res_n {
                Ok(0) => {
                    if self.stdout.is_eof() {
                        return None;
                    } else {
                        continue;
                    }
                }
                Ok(_) => {
                    return Some(bytes);
                }
                _ => {
                    return None;
                }
            }
        }
    }

    pub fn status(&self) -> Value {
        match self.status {
            Some(n) => { Value::Int(n) }
            _       => { Value::Null   }
        }
    }

    /// Clean up the associated processes and store the exit code.
    #[allow(unused_must_use)]
    pub fn cleanup(&mut self) {
        if self.dropped {
            return;
        }
        match self.pid {
            Some(p) => {
                let res = waitpid(p, Some(WaitPidFlag::WNOHANG));
                match res {
                    Ok(WaitStatus::Exited(_, n)) => {
                        self.status = Some(n);
                    }
                    Ok(WaitStatus::StillAlive) => {
                        let res = nix::sys::signal::kill(p, Signal::SIGTERM);
                        match res {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("unable to kill process: {}", e);
                            }
                        }
                    }
                    _ => {}
                }
                if self.status.is_none() {
                    let es_res = waitpid(p, None);
                    match es_res {
                        Ok(WaitStatus::Exited(_, n)) => {
                            self.status = Some(n);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        match self.pid2 {
            Some(p) => {
                let res = waitpid(p, Some(WaitPidFlag::WNOHANG));
                match res {
                    Ok(WaitStatus::StillAlive) => {
                        let res = nix::sys::signal::kill(p, Signal::SIGTERM);
                        match res {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("unable to kill process: {}", e);
                            }
                        }
                    }
                    _ => {}
                }
                waitpid(p, None);
            }
            _ => {}
        }
        self.dropped = true;
    }
}

impl Drop for CommandGenerator {
    fn drop(&mut self) {
        self.cleanup();
    }
}

impl Drop for ChannelGenerator {
    /// Kill the associated process when this is dropped.
    #[allow(unused_must_use)]
    fn drop(&mut self) {
        let p = self.pid;
        let res = waitpid(p, Some(WaitPidFlag::WNOHANG));
        match res {
            Ok(WaitStatus::StillAlive) => {
                let res = nix::sys::signal::kill(p, Signal::SIGTERM);
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("unable to kill process: {}", e);
                    }
                }
            }
            _ => {}
        }
        waitpid(p, None);
    }
}

/// The core value type used by the compiler and VM.
#[derive(Clone)]
pub enum Value {
    /// Used to indicate that a generator is exhausted.
    Null,
    /// Boolean.
    Bool(bool),
    /// Byte.
    Byte(u8),
    /// 32-bit integer.
    Int(i32),
    /// Unbounded integer.
    BigInt(num_bigint::BigInt),
    /// Floating-point number.
    Float(f64),
    /// String.  The second part here is the regex object that
    /// corresponds to the string, which is generated and cached
    /// when the string is used as a regex.
    String(Rc<RefCell<StringTriple>>),
    /// An external command (wrapped in curly brackets), where the
    /// output is captured.
    Command(Rc<String>, Rc<HashSet<char>>),
    /// An external command (begins with $), where the output is not
    /// captured.
    CommandUncaptured(Rc<String>),
    /// A list.
    List(Rc<RefCell<VecDeque<Value>>>),
    /// A hash.
    Hash(Rc<RefCell<IndexMap<String, Value>>>),
    /// A set.  The stringification of the value is used as the map
    /// key, and the set may only contain values of a single type.
    /// (Not terribly efficient, but can be made decent later without
    /// affecting the language interface.)
    Set(Rc<RefCell<IndexMap<String, Value>>>),
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
    CommandGenerator(Rc<RefCell<CommandGenerator>>),
    /// A generator over the keys of a hash.
    KeysGenerator(Rc<RefCell<HashWithIndex>>),
    /// A generator over the values of a hash.
    ValuesGenerator(Rc<RefCell<HashWithIndex>>),
    /// A generator over key-value pairs (lists) of a hash.
    EachGenerator(Rc<RefCell<HashWithIndex>>),
    /// A file reader value.
    FileReader(Rc<RefCell<BufReaderWithBuffer<File>>>),
    /// A file writer value.
    FileWriter(Rc<RefCell<BufWriter<File>>>),
    /// A directory handle.
    DirectoryHandle(Rc<RefCell<ReadDir>>),
    /// A datetime with a named timezone.
    DateTimeNT(DateTime<chrono_tz::Tz>),
    /// A datetime with an offset timezone.
    DateTimeOT(DateTime<FixedOffset>),
    /// An IPv4 address/prefix object.
    Ipv4(Ipv4Net),
    /// An IPv6 address/prefix object.
    Ipv6(Ipv6Net),
    /// An IPv4 range object (arbitrary start/end addresses).
    Ipv4Range(Ipv4Range),
    /// An IPv6 range object (arbitrary start/end addresses).
    Ipv6Range(Ipv6Range),
    /// An IP set (IPv4 and IPv6 together).
    IpSet(Rc<RefCell<IpSet>>),
    /// Multiple generators combined together.
    MultiGenerator(Rc<RefCell<VecDeque<Value>>>),
    /// A generator over the shell history.  This is presented as a
    /// 'plain' generator outside of the compiler.
    HistoryGenerator(Rc<RefCell<i32>>),
    /// A generator from a channel from a forked process.
    ChannelGenerator(Rc<RefCell<ChannelGenerator>>),
    /// A MySQL database connection.
    DBConnectionMySQL(Rc<RefCell<DBConnectionMySQL>>),
    /// A MySQL database statement.
    DBStatementMySQL(Rc<RefCell<DBStatementMySQL>>),
    /// A PostgreSQL database connection.
    DBConnectionPostgres(Rc<RefCell<DBConnectionPostgres>>),
    /// A PostgreSQL database statement.
    DBStatementPostgres(Rc<RefCell<DBStatementPostgres>>),
    /// A SQLite database connection.
    DBConnectionSQLite(Rc<RefCell<DBConnectionSQLite>>),
    /// A SQLite database statement.
    DBStatementSQLite(Rc<RefCell<DBStatementSQLite>>),
    /// A value that when accessed, indicates that a scope error has
    /// occurred.
    ScopeError,
    /// A TCP socket reader.
    TcpSocketReader(Rc<RefCell<BufReaderWithBuffer<TcpStream>>>),
    /// A TCP socket writer.
    TcpSocketWriter(Rc<RefCell<BufWriter<TcpStream>>>),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Null => {
                write!(f, "Null")
            }
            Value::Byte(i) => {
                write!(f, "{:#04x}", i)
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
            Value::Bool(b) => {
                write!(f, "{}", b)
            }
            Value::String(s) => {
                let ss = &s.borrow().string;
                write!(f, "\"{}\"", ss)
            }
            Value::Command(s, _) => {
                write!(f, "Command \"{}\"", s)
            }
            Value::CommandUncaptured(s) => {
                write!(f, "CommandUncaptured \"{}\"", s)
            }
            Value::List(ls) => {
                write!(f, "{:?}", ls)
            }
            Value::Hash(hs) => {
                write!(f, "{:?}", hs)
            }
            Value::Set(st) => {
                write!(f, "{:?}", st)
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
            Value::DateTimeNT(_) => {
                write!(f, "((DateTimeNT))")
            }
            Value::DateTimeOT(_) => {
                write!(f, "((DateTimeOT))")
            }
            Value::Ipv4(_) => {
                write!(f, "((IPv4))")
            }
            Value::Ipv4Range(_) => {
                write!(f, "((IPv4Range))")
            }
            Value::Ipv6(_) => {
                write!(f, "((IPv6))")
            }
            Value::Ipv6Range(_) => {
                write!(f, "((IPv6))")
            }
            Value::IpSet(_) => {
                write!(f, "((IpSet))")
            }
            Value::MultiGenerator(_) => {
                write!(f, "((MultiGenerator))")
            }
            Value::HistoryGenerator(_) => {
                write!(f, "((Generator))")
            }
            Value::DBConnectionMySQL(_) => {
                write!(f, "((DBConnection))")
            }
            Value::DBStatementMySQL(_) => {
                write!(f, "((DBStatement))")
            }
            Value::DBConnectionPostgres(_) => {
                write!(f, "((DBConnection))")
            }
            Value::DBStatementPostgres(_) => {
                write!(f, "((DBStatement))")
            }
            Value::DBConnectionSQLite(_) => {
                write!(f, "((DBConnection))")
            }
            Value::DBStatementSQLite(_) => {
                write!(f, "((DBStatement))")
            }
            Value::ChannelGenerator(_) => {
                write!(f, "((ChannelGenerator))")
            }
            Value::ScopeError => {
                write!(f, "((ScopeError))")
            }
            Value::TcpSocketReader(_) => {
                write!(f, "((SocketReader))")
            }
            Value::TcpSocketWriter(_) => {
                write!(f, "((SocketWriter))")
            }
        }
    }
}

/// An enum for the Value types that can be parsed from literals,
/// (i.e. those that can be stored as constants in a chunk).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueLiteral {
    Null,
    Bool(bool),
    Int(i32),
    Float(f64),
    BigInt(String),
    String(String, String),
    Command(String, HashSet<char>),
    CommandUncaptured(String),
}

/// An enum for the Value types that can be serialised and
/// deserialised (i.e. those that can be passed from/to a child
/// process (see pmap)).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueSD {
    Null,
    Bool(bool),
    Int(i32),
    Float(f64),
    BigInt(String),
    String(String),
    DateTimeOT(DateTime<FixedOffset>),
    DateTimeNT(String, String),
    Ipv4(Ipv4Net),
    Ipv6(Ipv6Net),
    Ipv4Range(Ipv4Range),
    Ipv6Range(Ipv6Range),
    IpSet(IpSet),
    List(VecDeque<ValueSD>),
    Hash(IndexMap<String, ValueSD>),
    Set(IndexMap<String, ValueSD>),
}

pub fn valuesd_to_value(value_sd: ValueSD) -> Value {
    match value_sd {
        ValueSD::Null => Value::Null,
        ValueSD::Bool(b) => Value::Bool(b),
        ValueSD::Int(n) => Value::Int(n),
        ValueSD::Float(f) => Value::Float(f),
        ValueSD::BigInt(bis) =>
            Value::BigInt(BigInt::from_str_radix(&bis, 10).unwrap()),
        ValueSD::String(s) => new_string_value(s),
        ValueSD::DateTimeOT(d) => Value::DateTimeOT(d),
        ValueSD::Ipv4(d) => Value::Ipv4(d),
        ValueSD::Ipv6(d) => Value::Ipv6(d),
        ValueSD::Ipv4Range(d) => Value::Ipv4Range(d),
        ValueSD::Ipv6Range(d) => Value::Ipv6Range(d),
        ValueSD::IpSet(d) => Value::IpSet(Rc::new(RefCell::new(d))),
        ValueSD::DateTimeNT(s, tzs) => {
            let mut parsed = Parsed::new();
            let pattern = StrftimeItems::new("%FT%T");
            parse(&mut parsed, &s, pattern).unwrap();
            let dt_res = parsed
                .to_naive_date()
                .unwrap()
                .and_time(parsed.to_naive_time().unwrap());
            let tz = chrono_tz::Tz::from_str(&tzs).unwrap();
            Value::DateTimeNT(tz.from_local_datetime(&dt_res).unwrap())
        }
        ValueSD::List(lst) => {
            let mut vds = VecDeque::new();
            for e in lst.iter() {
                vds.push_back(valuesd_to_value((*e).clone()));
            }
            Value::List(Rc::new(RefCell::new(vds)))
        }
        ValueSD::Hash(hsh) => {
            let mut new_hsh = IndexMap::new();
            for (k, v) in hsh.iter() {
                new_hsh.insert(k.clone(), valuesd_to_value(v.clone()));
            }
            Value::Hash(Rc::new(RefCell::new(new_hsh)))
        }
        ValueSD::Set(hsh) => {
            let mut new_hsh = IndexMap::new();
            for (k, v) in hsh.iter() {
                new_hsh.insert(k.clone(), valuesd_to_value(v.clone()));
            }
            Value::Set(Rc::new(RefCell::new(new_hsh)))
        }
    }
}

pub fn value_to_valuesd(value: Value) -> ValueSD {
    match value {
        Value::Null => ValueSD::Null,
        Value::Bool(b) => ValueSD::Bool(b),
        Value::Int(n) => ValueSD::Int(n),
        Value::Float(f) => ValueSD::Float(f),
        Value::BigInt(bi) => ValueSD::BigInt(bi.to_str_radix(10)),
        Value::String(s) => ValueSD::String(s.borrow().string.clone()),
        Value::DateTimeOT(d) => ValueSD::DateTimeOT(d),
        /* todo: Look at making this more efficient. */
        Value::DateTimeNT(d) => {
            let tzs = d.timezone().to_string();
            let ss = d.format("%FT%T").to_string();
            ValueSD::DateTimeNT(ss, tzs)
        }
        Value::Ipv4(d) => ValueSD::Ipv4(d),
        Value::Ipv6(d) => ValueSD::Ipv6(d),
        Value::Ipv4Range(d) => ValueSD::Ipv4Range(d),
        Value::Ipv6Range(d) => ValueSD::Ipv6Range(d),
        Value::IpSet(d) => ValueSD::IpSet(d.borrow().clone()),
        Value::List(lst_rr) => {
            let vd = lst_rr.borrow();
            let mut vds = VecDeque::new();
            for e in vd.iter() {
                vds.push_back(value_to_valuesd(e.clone()));
            }
            ValueSD::List(vds)
        }
        Value::Hash(hsh_rr) => {
            let hsh = hsh_rr.borrow();
            let mut new_hsh = IndexMap::new();
            for (k, v) in hsh.iter() {
                new_hsh.insert(k.clone(), value_to_valuesd(v.clone()));
            }
            ValueSD::Hash(new_hsh)
        }
        Value::Set(hsh_rr) => {
            let hsh = hsh_rr.borrow();
            let mut new_hsh = IndexMap::new();
            for (k, v) in hsh.iter() {
                new_hsh.insert(k.clone(), value_to_valuesd(v.clone()));
            }
            ValueSD::Set(new_hsh)
        }
        _ => ValueSD::Null
    }
}

/// A value along with a boolean indicating whether it was defined
/// with 'var' or 'varm'.
#[derive(Debug, Clone)]
pub struct Variable {
    pub value: Value,
    pub defined_with_var: bool,
}

impl Variable {
    pub fn new(
        value: Value,
        defined_with_var: bool
    ) -> Variable {
        Variable {
            value,
            defined_with_var,
        }
    }
}

pub fn new_string_value(s: String) -> Value {
    Value::String(Rc::new(RefCell::new(StringTriple::new(s, None))))
}

/// Convert a four-byte array into an i32 value.
pub fn bytes_to_i32(bytes: &Vec<u8>) -> i32 {
    let n0 = *bytes.get(0).unwrap() as i32;
    let n1 = *bytes.get(1).unwrap() as i32;
    let n2 = *bytes.get(2).unwrap() as i32;
    let n3 = *bytes.get(3).unwrap() as i32;
    let n = n0 | (n1 << 8) | (n2 << 16) | (n3 << 24);
    return n.into();
}

/// Write an i32 value to a four-byte array.
pub fn i32_to_bytes(n: i32, bytes: &mut Vec<u8>) {
    bytes[0] = (n         & 0xFF) as u8;
    bytes[1] = ((n >> 8)  & 0xFF) as u8;
    bytes[2] = ((n >> 16) & 0xFF) as u8;
    bytes[3] = ((n >> 24) & 0xFF) as u8;
}

/// Read a ValueSD value from the specified file.
pub fn read_valuesd(file: &mut std::fs::File) -> Option<ValueSD> {
    let mut size_buf = vec![0u8; 4];
    let read_res = file.read_exact(&mut size_buf);
    match read_res {
        Ok(_) => {
            let n = bytes_to_i32(&size_buf);
            let mut content_buf = vec![0u8; n as usize];
            loop {
                let read_res2 = file.read_exact(&mut content_buf);
                match read_res2 {
                    Ok(_) => {
                        let vsd: ValueSD = bincode::deserialize(&content_buf).unwrap();
                        return Some(vsd);
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        let dur = time::Duration::from_secs_f64(0.05);
                        thread::sleep(dur);
                    }
                    Err(_) => {
                        return None;
                    }
                }
            }
        }
        Err(_) => {
            return None;
        }
    }
}

/// Write a ValueSD value to the specified file.
pub fn write_valuesd(file: &mut std::fs::File, value: ValueSD) -> bool {
    let mut vec = bincode::serialize(&value).unwrap();
    let mut size_buf = vec![0u8; 4];
    let len = vec.len() as i32;
    i32_to_bytes(len, &mut size_buf);
    size_buf.append(&mut vec);
    let res = file.write(&size_buf);
    match res {
        Ok(_)  => { true }
        Err(_) => { false }
    }
}

/// Takes a chunk, an instruction index, and an error message as its
/// arguments.  Prints the error message, including filename, line number
/// and column number elements (if applicable).
pub fn print_error(chunk: Rc<RefCell<Chunk>>, i: usize, error: &str) {
    let point = chunk.borrow().get_point(i);
    let name = &chunk.borrow().name;
    let error_start = if name == "(main)" {
        String::new()
    } else {
        format!("{}:", name)
    };
    match point {
        Some((line, col)) => {
            let s = format!("{}{}:{}: {}", error_start, line, col, error).to_string();
            eprintln!("{}", s);
        }
        _ => {
            let s = format!("{}{}", error_start, error).to_string();
            eprintln!("{}", s);
        }
    }
}

impl Chunk {
    /// Construct a standard (non-generator) chunk.
    pub fn new_standard(name: String) -> Chunk {
        Chunk {
            name,
            data: Vec::new(),
            is_opcode: Vec::new(),
            points: Vec::new(),
            constants: Vec::new(),
            functions: HashMap::new(),
            is_generator: false,
            has_vars: true,
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
            name,
            data: Vec::new(),
            is_opcode: Vec::new(),
            points: Vec::new(),
            constants: Vec::new(),
            functions: HashMap::new(),
            is_generator: true,
            has_vars: true,
            arg_count,
            req_arg_count,
            nested: false,
            scope_depth: 0,
            constant_values: Vec::new(),
        }
    }

    /// Add a constant to the current chunk, and return its index in
    /// the constants list (for later calls to `get_constant`).
    pub fn add_constant(&mut self, value_rr: Value) -> i32 {
        let value_sd = match value_rr {
            Value::Null => ValueLiteral::Null,
            Value::Int(n) => ValueLiteral::Int(n),
            Value::Float(n) => ValueLiteral::Float(n),
            Value::BigInt(n) => ValueLiteral::BigInt(n.to_str_radix(10)),
            Value::String(st) => ValueLiteral::String(
                st.borrow().string.to_string(),
                st.borrow().escaped_string.to_string(),
            ),
            Value::Command(s, params) => ValueLiteral::Command(s.to_string(), (*params).clone()),
            Value::CommandUncaptured(s) => ValueLiteral::CommandUncaptured(s.to_string()),
            Value::Bool(b) => ValueLiteral::Bool(b),
            _ => {
                eprintln!("constant type cannot be added to chunk! {:?}", value_rr);
                std::process::abort();
            }
        };
        self.constants.push(value_sd);
        (self.constants.len() - 1) as i32
    }

    /// Add a constant to the current chunk, as well as the bytes for
    /// the index.
    pub fn add_constant_and_index(&mut self, value_rr: Value) -> i32 {
        let i = self.add_constant(value_rr.clone());
        let i_upper = (i >> 8) & 0xFF;
        let i_lower = i & 0xFF;
        self.add_byte(i_upper as u8);
        self.add_byte(i_lower as u8);
        return i;
    }

    /// Get a constant from the current chunk.
    pub fn get_constant(&self, i: i32) -> Value {
        let value_sd = &self.constants[i as usize];
        match value_sd {
            ValueLiteral::Null => Value::Null,
            ValueLiteral::Bool(b) => Value::Bool(*b),
            ValueLiteral::Int(n) => Value::Int(*n),
            ValueLiteral::Float(n) => Value::Float(*n),
            ValueLiteral::BigInt(n) => {
                let nn = n.parse::<num_bigint::BigInt>().unwrap();
                Value::BigInt(nn)
            }
            ValueLiteral::String(st1, st2) => {
                let st = StringTriple::new_with_escaped(st1.to_string(), st2.to_string(), None);
                Value::String(Rc::new(RefCell::new(st)))
            }
            ValueLiteral::Command(s, params) => {
                Value::Command(Rc::new(s.to_string()), Rc::new((*params).clone()))
            }
            ValueLiteral::CommandUncaptured(s) => Value::CommandUncaptured(Rc::new(s.to_string())),
        }
    }

    /// Get a constant value from the current chunk.  If the relevant
    /// constant value has not been initialised, this will return
    /// Value::Null.
    pub fn get_constant_value(&self, i: i32) -> Value {
        let value = self.constant_values.get(i as usize);
        match value {
            Some(v) => v.clone(),
            _ => Value::Null,
        }
    }

    /// Get a constant int value from the current chunk.
    pub fn get_constant_int(&self, i: i32) -> i32 {
        let value_sd = &self.constants[i as usize];
        match *value_sd {
            ValueLiteral::Int(n) => n,
            _ => 0,
        }
    }

    /// Check whether the chunk has a constant int value at the
    /// specified index.
    pub fn has_constant_int(&self, i: i32) -> bool {
        let value_sd = &self.constants[i as usize];
        matches!(*value_sd, ValueLiteral::Int(_))
    }

    /// Add an opcode to the current chunk's data.
    pub fn add_opcode(&mut self, opcode: OpCode) {
        self.data.push(opcode as u8);
        self.is_opcode.push(true);
    }

    /// If the last byte in the data is an opcode, return that opcode.
    pub fn get_last_opcode(&self) -> Option<OpCode> {
        if *self.is_opcode.last().unwrap() {
            Some(to_opcode(*self.data.last().unwrap()))
        } else {
            None
        }
    }

    /// If the second-last byte in the data is an opcode, return that
    /// opcode.
    pub fn get_second_last_opcode(&self) -> Option<OpCode> {
        if self.data.len() < 2 {
            return None;
        }
        if *self.is_opcode.get(self.is_opcode.len() - 2).unwrap() {
            Some(to_opcode(*self.data.get(self.data.len() - 2).unwrap()))
        } else {
            None
        }
    }

    /// If the third-last byte in the data is an opcode, return that
    /// opcode.
    pub fn get_third_last_opcode(&self) -> Option<OpCode> {
        if self.data.len() < 3 {
            return None;
        }
        if *self.is_opcode.get(self.is_opcode.len() - 3).unwrap() {
            Some(to_opcode(*self.data.get(self.data.len() - 3).unwrap()))
        } else {
            None
        }
    }

    /// If the fourth-last byte in the data is an opcode, return that
    /// opcode.
    pub fn get_fourth_last_opcode(&self) -> Option<OpCode> {
        if self.data.len() < 4 {
            return None;
        }
        if *self.is_opcode.get(self.is_opcode.len() - 4).unwrap() {
            Some(to_opcode(*self.data.get(self.data.len() - 4).unwrap()))
        } else {
            None
        }
    }

    /// Set the last opcode for the current chunk's data.
    pub fn set_last_opcode(&mut self, opcode: OpCode) {
        let len = self.data.len();
        if let Some(el) = self.data.get_mut(len - 1) {
            *el = opcode as u8;
            let io = self.is_opcode.get_mut(len - 1).unwrap();
            *io = true;
        }
    }

    /// Set the second-last opcode for the current chunk's data.
    pub fn set_second_last_opcode(&mut self, opcode: OpCode) {
        let len = self.data.len();
        if let Some(el) = self.data.get_mut(len - 2) {
            *el = opcode as u8;
            let io = self.is_opcode.get_mut(len - 2).unwrap();
            *io = true;
        }
    }

    /// Set the third-last opcode for the current chunk's data.
    pub fn set_third_last_opcode(&mut self, opcode: OpCode) {
        let len = self.data.len();
        if let Some(el) = self.data.get_mut(len - 3) {
            *el = opcode as u8;
            let io = self.is_opcode.get_mut(len - 3).unwrap();
            *io = true;
        }
    }

    /// Set the fourth-last opcode for the current chunk's data.
    pub fn set_fourth_last_opcode(&mut self, opcode: OpCode) {
        let len = self.data.len();
        if let Some(el) = self.data.get_mut(len - 4) {
            *el = opcode as u8;
            let io = self.is_opcode.get_mut(len - 4).unwrap();
            *io = true;
        }
    }

    /// Add a raw byte to the current chunk's data.
    pub fn add_byte(&mut self, byte: u8) {
        self.data.push(byte);
        self.is_opcode.push(false);
    }

    /// Remove the last byte from the current chunk's data.
    pub fn pop_byte(&mut self) {
        self.data.pop();
        self.is_opcode.pop();
    }

    /// If the last byte in the data is a raw byte, return that byte.
    pub fn get_last_byte(&self) -> Option<u8> {
        if !*self.is_opcode.last().unwrap() {
            Some(*self.data.last().unwrap())
        } else {
            None
        }
    }

    /// If the second-last byte in the data is a raw byte, return that
    /// byte.
    pub fn get_second_last_byte(&self) -> Option<u8> {
        if self.data.len() < 2 {
            return None;
        }
        if !*self.is_opcode.get(self.is_opcode.len() - 2).unwrap() {
            Some(*self.data.get(self.data.len() - 2).unwrap())
        } else {
            None
        }
    }

    /// If the third-last byte in the data is a raw byte, return that
    /// byte.
    pub fn get_third_last_byte(&self) -> Option<u8> {
        if self.data.len() < 3 {
            return None;
        }
        if !*self.is_opcode.get(self.is_opcode.len() - 3).unwrap() {
            Some(*self.data.get(self.data.len() - 3).unwrap())
        } else {
            None
        }
    }

    /// Set the last byte for the current chunk's data as a raw byte.
    pub fn set_last_byte(&mut self, byte: u8) {
        let len = self.data.len();
        if let Some(el) = self.data.get_mut(len - 1) {
            *el = byte;
            let io = self.is_opcode.get_mut(len - 1).unwrap();
            *io = false;
        }
    }

    /// Set the second-last byte for the current chunk's data as a raw
    /// byte.
    pub fn set_second_last_byte(&mut self, byte: u8) {
        let len = self.data.len();
        if let Some(el) = self.data.get_mut(len - 2) {
            *el = byte;
            let io = self.is_opcode.get_mut(len - 2).unwrap();
            *io = false;
        }
    }

    /// Set the third-last byte for the current chunk's data as a raw
    /// byte.
    pub fn set_third_last_byte(&mut self, byte: u8) {
        let len = self.data.len();
        if let Some(el) = self.data.get_mut(len - 3) {
            *el = byte;
            let io = self.is_opcode.get_mut(len - 3).unwrap();
            *io = false;
        }
    }

    /// Check that the chunk has at least one constant.
    pub fn has_constant(&mut self) -> bool {
        self.constants.len() != 0
    }

    /// Get the chunk's most recently-added constant.
    pub fn get_last_constant(&mut self) -> Value {
        self.get_constant((self.constants.len() - 1).try_into().unwrap())
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
                OpCode::Clone => {
                    println!("OP_CLONE");
                }
                OpCode::Constant => {
                    i += 1;
                    let i_upper = self.data[i];
                    i += 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);
                    let value = self.get_constant(constant_i as i32);
                    println!("OP_CONSTANT {:?}", value);
                }
                OpCode::AddConstant => {
                    i += 1;
                    let i_upper = self.data[i];
                    i += 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);
                    let value = self.get_constant(constant_i as i32);
                    println!("OP_ADDCONSTANT {:?}", value);
                }
                OpCode::SubtractConstant => {
                    i += 1;
                    let i_upper = self.data[i];
                    i += 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);
                    let value = self.get_constant(constant_i as i32);
                    println!("OP_SUBTRACTCONSTANT {:?}", value);
                }
                OpCode::DivideConstant => {
                    i += 1;
                    let i_upper = self.data[i];
                    i += 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);
                    let value = self.get_constant(constant_i as i32);
                    println!("OP_DIVIDECONSTANT {:?}", value);
                }
                OpCode::MultiplyConstant => {
                    i += 1;
                    let i_upper = self.data[i];
                    i += 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);
                    let value = self.get_constant(constant_i as i32);
                    println!("OP_MULTIPLYCONSTANT {:?}", value);
                }
                OpCode::EqConstant => {
                    i += 1;
                    let i_upper = self.data[i];
                    i += 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);
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
                OpCode::Remainder => {
                    println!("OP_REMAINDER");
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
                OpCode::VarM => {
                    println!("OP_VARM");
                }
                OpCode::SetVar => {
                    println!("OP_SETVAR");
                }
                OpCode::GetVar => {
                    println!("OP_GETVAR");
                }
                OpCode::SetLocalVar => {
                    i += 1;
                    let var_i = self.data[i];
                    println!("OP_SETLOCALVAR {}", var_i);
                }
                OpCode::GetLocalVar => {
                    i += 1;
                    let var_i = self.data[i];
                    println!("OP_GETLOCALVAR {}", var_i);
                }
                OpCode::GLVShift => {
                    i += 1;
                    let var_i = self.data[i];
                    println!("OP_GLVSHIFT {}", var_i);
                }
                OpCode::GLVCall => {
                    i += 1;
                    let var_i = self.data[i];
                    println!("OP_GLVCALL {}", var_i);
                }
                OpCode::PopLocalVar => {
                    println!("OP_POPLOCALVAR");
                }
                OpCode::Jump => {
                    i += 1;
                    let i1: usize = self.data[i].try_into().unwrap();
                    i += 1;
                    let i2: usize = self.data[i].try_into().unwrap();
                    let jump_i: usize = (i1 << 8) | i2;
                    println!("OP_JUMP {:?}", jump_i);
                }
                OpCode::JumpR => {
                    i += 1;
                    let i1: usize = self.data[i].try_into().unwrap();
                    i += 1;
                    let i2: usize = self.data[i].try_into().unwrap();
                    let jump_i: usize = (i1 << 8) | i2;
                    println!("OP_JUMPR {:?}", jump_i);
                }
                OpCode::JumpNe => {
                    i += 1;
                    let i1: usize = self.data[i].try_into().unwrap();
                    i += 1;
                    let i2: usize = self.data[i].try_into().unwrap();
                    let jump_i: usize = (i1 << 8) | i2;
                    println!("OP_JUMPNE {:?}", jump_i);
                }
                OpCode::JumpNeR => {
                    i += 1;
                    let i1: usize = self.data[i].try_into().unwrap();
                    i += 1;
                    let i2: usize = self.data[i].try_into().unwrap();
                    let jump_i: usize = (i1 << 8) | i2;
                    println!("OP_JUMPNER {:?}", jump_i);
                }
                OpCode::JumpNeREqC => {
                    i += 1;
                    let i1: usize = self.data[i].try_into().unwrap();
                    i += 1;
                    let i2: usize = self.data[i].try_into().unwrap();
                    let jump_i: usize = (i1 << 8) | i2;

                    i += 1;
                    let i_upper = self.data[i];
                    i += 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);
                    let value = self.get_constant(constant_i as i32);

                    println!("OP_JUMPNEREQC {:?} {:?}", jump_i, value);
                }
                OpCode::Cmp => {
                    println!("OP_CMP");
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
                OpCode::StartSet => {
                    println!("OP_STARTSET");
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
                    i += 1;
                    let i_upper = self.data[i];
                    i += 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);
                    let value = self.get_constant(constant_i as i32);
                    println!("OP_CALLCONSTANT {:?}", value);
                }
                OpCode::CallImplicitConstant => {
                    i += 1;
                    let i_upper = self.data[i];
                    i += 1;
                    let i_lower = self.data[i];
                    let constant_i = (((i_upper as u16) << 8) & 0xFF00) | (i_lower as u16);
                    let value = self.get_constant(constant_i as i32);
                    println!("OP_CALLIMPLICITCONSTANT {:?}", value);
                }
                OpCode::Bool => {
                    println!("OP_BOOL");
                }
                OpCode::IsBool => {
                    println!("OP_ISBOOL");
                }
                OpCode::IsInt => {
                    println!("OP_ISINT");
                }
                OpCode::IsBigInt => {
                    println!("OP_ISBIGINT");
                }
                OpCode::IsStr => {
                    println!("OP_ISSTR");
                }
                OpCode::IsFlt => {
                    println!("OP_ISFLT");
                }
                OpCode::BigInt => {
                    println!("OP_BIGINT");
                }
                OpCode::Byte => {
                    println!("OP_BYTE");
                }
                OpCode::IsByte => {
                    println!("OP_ISBYTE");
                }
                OpCode::Read => {
                    println!("OP_READ");
                }
                OpCode::Unknown => {
                    println!("(Unknown)");
                }
            }
            i += 1;
        }

        for (k, v) in self.functions.iter() {
            println!("== {}.{} ==", name, k);
            v.borrow().disassemble(k);
        }
    }
}

/// A macro for converting a value into a string, that avoids any
/// copies or similar if the argument is already a string.
macro_rules! to_str {
    ($val:expr, $var:expr) => {
        let lib_str_s;
        let lib_str_b;
        let lib_str_str;
        let lib_str_bk: Option<String>;
        $var = match $val {
            Value::String(st) => {
                lib_str_s = st;
                lib_str_b = lib_str_s.borrow();
                Some(&lib_str_b.string)
            }
            _ => {
                lib_str_bk = $val.to_string();
                match lib_str_bk {
                    Some(s) => {
                        lib_str_str = s;
                        Some(&lib_str_str)
                    }
                    _ => None,
                }
            }
        }
    };
}

impl Value {
    /// Convert the current value into a string.  Not intended for use
    /// with Value::String.
    pub fn to_string(&self) -> Option<String> {
        match self {
            Value::String(_) => {
                eprintln!("to_string should not be called with Value::String!");
                std::process::abort();
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
            Value::Ipv4(ipv4net) => {
                let prefix_len = ipv4net.prefix_len();
                if prefix_len == 32 {
                    let ip_str = format!("{}", ipv4net);
                    let ip_str_no_len =
                        ip_str.chars().take_while(|&c| c != '/').collect::<String>();
                    Some(ip_str_no_len)
                } else {
                    let s = format!("{}", ipv4net);
                    Some(s)
                }
            }
            Value::Ipv6(ipv6net) => {
                let prefix_len = ipv6net.prefix_len();
                if prefix_len == 128 {
                    let s = format!("{}", ipv6net.network());
                    Some(s)
                } else {
                    let s = format!("{}/{}", ipv6net.network(), ipv6net.prefix_len());
                    Some(s)
                }
            }
            Value::Ipv4Range(ipv4range) => {
                let s = format!("{}-{}", ipv4range.s, ipv4range.e);
                Some(s)
            }
            Value::Ipv6Range(ipv6range) => {
                let s = format!("{}-{}", ipv6range.s, ipv6range.e);
                Some(s)
            }
            Value::IpSet(ipset) => {
                let ipv4range = &ipset.borrow().ipv4;
                let ipv6range = &ipset.borrow().ipv6;
                let mut lst = Vec::new();
                let mut ipv4lst = ipv4range.iter().collect::<Vec<Ipv4Net>>();
                ipv4lst.sort_by_key(|a| a.network());
                for ipv4net in ipv4lst.iter() {
                    let prefix_len = ipv4net.prefix_len();
                    if prefix_len == 32 {
                        let ip_str = format!("{}", ipv4net);
                        let ip_str_no_len =
                            ip_str.chars().take_while(|&c| c != '/').collect::<String>();
                        lst.push(ip_str_no_len);
                    } else {
                        let ip_str = format!("{}", ipv4net);
                        lst.push(ip_str);
                    }
                }
                let mut ipv6lst = ipv6range.iter().collect::<Vec<Ipv6Net>>();
                ipv6lst.sort_by_key(|a| a.network());
                for ipv6net in ipv6lst.iter() {
                    let prefix_len = ipv6net.prefix_len();
                    if prefix_len == 128 {
                        let ip_str = format!("{}", ipv6net.network());
                        lst.push(ip_str);
                    } else {
                        let ip_str = format!("{}/{}", ipv6net.network(), ipv6net.prefix_len());
                        lst.push(ip_str);
                    }
                }
                let s = lst.join(",");
                Some(s)
            }
            Value::Null => Some("".to_string()),
            Value::List(lst) => {
                let mut bytes = Vec::<u8>::new();
                for e in lst.borrow().iter() {
                    match e {
                        Value::Byte(b) => {
                            bytes.push(*b);
                        }
                        _ => {
                            return None;
                        }
                    }
                }
                let s = String::from_utf8_lossy(&bytes[..]);
                return Some(s.to_string());
            }
            _ => None,
        }
    }

    /// Convert the current value into an i32.  If the value is not
    /// representable as an i32, the result will be None.
    pub fn to_int(&self) -> Option<i32> {
        match self {
            Value::Byte(b) => Some(*b as i32),
            Value::Int(n) => Some(*n),
            Value::BigInt(n) => n.to_i32(),
            Value::Float(f) => Some(*f as i32),
            Value::String(st) => {
                let s = &st.borrow().string;
                let n_r = s.parse::<i32>();
                match n_r {
                    Ok(n) => Some(n),
                    _ => None,
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
            Value::Byte(b) => Some(BigInt::from_i32(*b as i32).unwrap()),
            Value::Int(n) => Some(BigInt::from_i32(*n).unwrap()),
            Value::BigInt(n) => Some(n.clone()),
            Value::Float(f) => Some(BigInt::from_i32(*f as i32).unwrap()),
            Value::String(st) => {
                let s = &st.borrow().string;
                let n_r = s.to_string().parse::<num_bigint::BigInt>();
                match n_r {
                    Ok(n) => Some(n),
                    _ => None,
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
            Value::String(st) => {
                let s = &st.borrow().string;
                let n_r = s.parse::<f64>();
                match n_r {
                    Ok(n) => Some(n),
                    _ => None,
                }
            }
            Value::Null => Some(0.0),
            _ => None,
        }
    }

    pub fn to_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(0) => false,
            Value::Float(n) => *n != 0.0,
            Value::String(st) => {
                let ss = &st.borrow().string;
                !ss.is_empty() && ss != "0" && ss != "0.0"
            }
            Value::BigInt(n) => *n == Zero::zero(),
            Value::Null => false,
            _ => true,
        }
    }

    pub fn value_clone(&self) -> Value {
        match self {
            Value::Null => self.clone(),
            Value::Bool(_) => self.clone(),
            Value::Byte(_) => self.clone(),
            Value::Int(_) => self.clone(),
            Value::BigInt(_) => self.clone(),
            Value::Float(_) => self.clone(),
            Value::String(_) => self.clone(),
            Value::Command(_, _) => self.clone(),
            Value::CommandUncaptured(_) => self.clone(),
            Value::List(lst) => {
                let cloned_lst = lst.borrow().iter().map(|v| v.value_clone()).collect();
                Value::List(Rc::new(RefCell::new(cloned_lst)))
            }
            Value::Hash(hsh) => {
                let mut cloned_hsh = IndexMap::new();
                for (k, v) in hsh.borrow().iter() {
                    cloned_hsh.insert(k.clone(), v.value_clone());
                }
                Value::Hash(Rc::new(RefCell::new(cloned_hsh)))
            }
            Value::Set(hsh) => {
                let mut cloned_hsh = IndexMap::new();
                for (k, v) in hsh.borrow().iter() {
                    cloned_hsh.insert(k.clone(), v.value_clone());
                }
                Value::Set(Rc::new(RefCell::new(cloned_hsh)))
            }
            Value::AnonymousFunction(_, _) => self.clone(),
            Value::CoreFunction(_) => self.clone(),
            Value::NamedFunction(_) => self.clone(),
            Value::Generator(gen_ref) => {
                let gen = gen_ref.borrow();
                let local_vars_stack = gen.local_vars_stack.clone();
                let index = gen.index;
                let chunk = gen.chunk.clone();
                let call_stack_chunks = gen.call_stack_chunks.clone();
                let gen_args = gen.gen_args.clone();
                let new_gen = GeneratorObject::new(
                    Rc::new(RefCell::new(local_vars_stack.borrow().clone())),
                    index,
                    chunk,
                    call_stack_chunks,
                    gen_args,
                );
                Value::Generator(Rc::new(RefCell::new(new_gen)))
            }
            Value::CommandGenerator(_) => self.clone(),
            Value::KeysGenerator(keys_gen_ref) => {
                Value::KeysGenerator(Rc::new(RefCell::new(keys_gen_ref.borrow().clone())))
            }
            Value::ValuesGenerator(values_gen_ref) => {
                Value::ValuesGenerator(Rc::new(RefCell::new(values_gen_ref.borrow().clone())))
            }
            Value::EachGenerator(each_gen_ref) => {
                Value::EachGenerator(Rc::new(RefCell::new(each_gen_ref.borrow().clone())))
            }
            Value::FileReader(_) => self.clone(),
            Value::FileWriter(_) => self.clone(),
            Value::DirectoryHandle(_) => self.clone(),
            Value::DateTimeNT(_) => self.clone(),
            Value::DateTimeOT(_) => self.clone(),
            Value::Ipv4(_) => self.clone(),
            Value::Ipv4Range(_) => self.clone(),
            Value::Ipv6(_) => self.clone(),
            Value::Ipv6Range(_) => self.clone(),
            Value::IpSet(ipset_ref) => {
                Value::IpSet(Rc::new(RefCell::new(ipset_ref.borrow().clone())))
            },
            Value::MultiGenerator(_) => self.clone(),
            Value::HistoryGenerator(_) => self.clone(),
            Value::ChannelGenerator(_) => self.clone(),
            Value::DBConnectionMySQL(_) => self.clone(),
            Value::DBStatementMySQL(_) => self.clone(),
            Value::DBConnectionPostgres(_) => self.clone(),
            Value::DBStatementPostgres(_) => self.clone(),
            Value::DBConnectionSQLite(_) => self.clone(),
            Value::DBStatementSQLite(_) => self.clone(),
            Value::ScopeError => self.clone(),
            Value::TcpSocketReader(_) => self.clone(),
            Value::TcpSocketWriter(_) => self.clone(),
        }
    }

    pub fn variants_equal(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(..), Value::Bool(..)) => true,
            (Value::Int(..), Value::Int(..)) => true,
            (Value::BigInt(..), Value::BigInt(..)) => true,
            (Value::Float(..), Value::Float(..)) => true,
            (Value::String(..), Value::String(..)) => true,
            (Value::Command(..), Value::Command(..)) => true,
            (Value::CommandUncaptured(..), Value::CommandUncaptured(..)) => true,
            (Value::List(..), Value::List(..)) => true,
            (Value::Hash(..), Value::Hash(..)) => true,
            (Value::Set(..), Value::Set(..)) => true,
            (Value::AnonymousFunction(..), Value::AnonymousFunction(..)) => true,
            (Value::CoreFunction(..), Value::CoreFunction(..)) => true,
            (Value::NamedFunction(..), Value::NamedFunction(..)) => true,
            (Value::Generator(..), Value::Generator(..)) => true,
            (Value::CommandGenerator(..), Value::CommandGenerator(..)) => true,
            (Value::KeysGenerator(..), Value::KeysGenerator(..)) => true,
            (Value::ValuesGenerator(..), Value::ValuesGenerator(..)) => true,
            (Value::EachGenerator(..), Value::EachGenerator(..)) => true,
            (Value::FileReader(..), Value::FileReader(..)) => true,
            (Value::FileWriter(..), Value::FileWriter(..)) => true,
            (Value::DirectoryHandle(..), Value::DirectoryHandle(..)) => true,
            (Value::DateTimeNT(..), Value::DateTimeNT(..)) => true,
            (Value::DateTimeOT(..), Value::DateTimeOT(..)) => true,
            (Value::Ipv4(..), Value::Ipv4(..)) => true,
            (Value::Ipv6(..), Value::Ipv6(..)) => true,
            (Value::Ipv4Range(..), Value::Ipv4Range(..)) => true,
            (Value::Ipv6Range(..), Value::Ipv6Range(..)) => true,
            (Value::IpSet(..), Value::IpSet(..)) => true,
            (Value::MultiGenerator(..), Value::MultiGenerator(..)) => true,
            (Value::HistoryGenerator(..), Value::HistoryGenerator(..)) => true,
            (Value::DBConnectionMySQL(..), Value::DBConnectionMySQL(..)) => true,
            (Value::DBStatementMySQL(..), Value::DBStatementMySQL(..)) => true,
            (Value::DBConnectionPostgres(..), Value::DBConnectionPostgres(..)) => true,
            (Value::DBStatementPostgres(..), Value::DBStatementPostgres(..)) => true,
            (Value::DBConnectionSQLite(..), Value::DBConnectionSQLite(..)) => true,
            (Value::DBStatementSQLite(..), Value::DBStatementSQLite(..)) => true,
            (Value::TcpSocketReader(..), Value::TcpSocketReader(..)) => true,
            (Value::TcpSocketWriter(..), Value::TcpSocketWriter(..)) => true,
            (..) => false,
        }
    }

    pub fn is_generator(&self) -> bool {
        matches!(
            self,
            Value::Generator(..)
                | Value::KeysGenerator(..)
                | Value::ValuesGenerator(..)
                | Value::EachGenerator(..)
                | Value::FileReader(..)
                | Value::DirectoryHandle(..)
                | Value::MultiGenerator(..)
                | Value::HistoryGenerator(..)
                | Value::CommandGenerator(..)
                | Value::ChannelGenerator(..)
                | Value::TcpSocketReader(..)
        )
    }

    pub fn is_shiftable(&self) -> bool {
        if self.is_generator() {
            return true;
        }
        matches!(
            self,
            Value::List(_)
                | Value::Set(_)
                | Value::IpSet(_)
        )
    }

    pub fn type_string(&self) -> String {
        let s = match self {
            Value::Null => "null",
            Value::Bool(..) => "bool",
            Value::Byte(..) => "byte",
            Value::Int(..) => "int",
            Value::BigInt(..) => "bigint",
            Value::Float(..) => "float",
            Value::String(..) => "str",
            Value::Command(..) => "command",
            Value::CommandUncaptured(..) => "command",
            Value::List(..) => "list",
            Value::Hash(..) => "hash",
            Value::Set(..) => "set",
            Value::AnonymousFunction(..) => "anon-fn",
            Value::CoreFunction(..) => "core-fn",
            Value::NamedFunction(..) => "named-fn",
            Value::Generator(..) => "gen",
            Value::CommandGenerator(..) => "command-gen",
            Value::KeysGenerator(..) => "keys-gen",
            Value::ValuesGenerator(..) => "values-gen",
            Value::EachGenerator(..) => "each-gen",
            Value::FileReader(..) => "file-reader",
            Value::FileWriter(..) => "file-writer",
            Value::DirectoryHandle(..) => "dir-handle",
            Value::DateTimeNT(..) => "datetime",
            Value::DateTimeOT(..) => "datetime",
            Value::Ipv4(..) => "ip",
            Value::Ipv6(..) => "ip",
            Value::Ipv4Range(..) => "ip",
            Value::Ipv6Range(..) => "ip",
            Value::IpSet(..) => "ips",
            Value::MultiGenerator(..) => "multi-gen",
            Value::HistoryGenerator(..) => "gen",
            Value::ChannelGenerator(..) => "channel-gen",
            Value::DBConnectionMySQL(..) => "db-connection",
            Value::DBStatementMySQL(..) => "db-statement",
            Value::DBConnectionPostgres(..) => "db-connection",
            Value::DBStatementPostgres(..) => "db-statement",
            Value::DBConnectionSQLite(..) => "db-connection",
            Value::DBStatementSQLite(..) => "db-statement",
            Value::ScopeError => "scope-error",
            Value::TcpSocketReader(..) => "socket-reader",
            Value::TcpSocketWriter(..) => "socket-writer",
        };
        s.to_string()
    }
}

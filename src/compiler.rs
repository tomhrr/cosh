use std::cell::RefCell;
use std::collections::HashSet;
use std::convert::TryInto;
use std::fs;
use std::io::BufRead;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Write;
use std::rc::Rc;
use std::str;

use lazy_static::lazy_static;
use regex::Regex;

use chunk::{Chunk, StringTriple, Value};
use opcode::OpCode;

/// The various token types used by the compiler.
#[derive(Debug)]
pub enum TokenType {
    True,
    False,
    Null,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    StartFunction,
    StartGenerator,
    EndFunction,
    Int(i32),
    BigInt(num_bigint::BigInt),
    Float(f64),
    String(String),
    Command(String, HashSet<char>),
    /// This is a Command that includes a ';' character at the end, or
    /// otherwise should be executed immediately (e.g. it's followed
    /// by a newline).
    CommandExplicit(String, HashSet<char>),
    CommandUncaptured(String),
    Word(String),
    /// This is a Word that should be executed implicitly (e.g. it's
    /// followed by a newline).
    WordImplicit(String),
    StartList,
    StartHash,
    StartSet,
    /// The EndList token also serves as the terminating token for
    /// hashes and sets, which is why there is no EndHash or EndSet
    /// token.
    EndList,
    /// A dummy token that indicates that the caller should try to
    /// fetch another token.
    Retry,
    Error,
    Eof,
}

/// The token struct, which includes the line and column number where
/// the token begins.
#[derive(Debug)]
pub struct Token {
    token_type: TokenType,
    column_number: u32,
    line_number: u32,
}

impl Token {
    fn new(token_type: TokenType, line_number: u32, column_number: u32) -> Token {
        Token {
            token_type: token_type,
            line_number: line_number,
            column_number: column_number,
        }
    }
}

/// A Local is a local variable.  The depth of a Local is its scope
/// depth, which will always be greater than one.
#[derive(Debug)]
pub struct Local {
    name: String,
    depth: u32,
}

impl Local {
    pub fn new(name: String, depth: u32) -> Local {
        Local {
            name: name,
            depth: depth,
        }
    }
}

/// A Scanner is used to get tokens from a BufRead object.  It manages
/// a single character of lookahead.
pub struct Scanner<'a> {
    fh: &'a mut Box<dyn BufRead>,
    line_number: u32,
    column_number: u32,
    token_line_number: u32,
    token_column_number: u32,
    has_lookahead: bool,
    lookahead: u8,
    next_is_eof: bool,
}

lazy_static! {
    static ref INT: Regex = Regex::new(r"^-?\d+$").unwrap();
    static ref FLOAT: Regex = Regex::new(r"^-?\d+\.\d+$").unwrap();
}

impl<'a> Scanner<'a> {
    pub fn new(fh: &mut Box<dyn BufRead>) -> Scanner {
        Scanner {
            fh: fh,
            line_number: 1,
            column_number: 1,
            token_line_number: 1,
            token_column_number: 1,
            has_lookahead: false,
            lookahead: 0,
            next_is_eof: false,
        }
    }

    /// Scans the BufRead for potential parameters, and returns a
    /// char set for those parameters if they are present.
    pub fn scan_parameters(&mut self) -> Option<HashSet<char>> {
        let mut buffer = [0; 1];
        let mut eof = false;
        let mut done = false;
        let mut parameters: HashSet<char> = HashSet::new();

        if self.has_lookahead {
            buffer[0] = self.lookahead;
            self.has_lookahead = false;
        } else {
            self.fh.read_exact(&mut buffer).unwrap_or_else(|e| {
                if e.kind() == ErrorKind::UnexpectedEof {
                    eof = true;
                } else {
                    eprintln!("unable to read from buffer!");
                    std::process::abort();
                }
            });
            if eof {
                self.next_is_eof = true;
                return None;
            }
        }

        match buffer[0] as char {
            '/' => {
                /* Has parameters. */
                self.column_number = self.column_number + 1;
                while !done {
                    self.fh.read_exact(&mut buffer).unwrap_or_else(|e| {
                        if e.kind() == ErrorKind::UnexpectedEof {
                            eof = true;
                        } else {
                            eprintln!("unable to read from buffer!");
                            std::process::abort();
                        }
                    });
                    if eof {
                        self.next_is_eof = true;
                        return Some(parameters);
                    }
                    if char::is_whitespace(buffer[0] as char)
                            || buffer[0] == ';' as u8 {
                        self.lookahead = buffer[0];
                        self.has_lookahead = true;
                        done = true;
                    } else {
                        parameters.insert(buffer[0] as char);
                        self.column_number = self.column_number + 1;
                    }
                }
                return Some(parameters);
            }
            _ => {
                self.lookahead = buffer[0];
                self.has_lookahead = true;
                return None;
            }
        }
    }

    /// Skip any whitespace, updating line and column numbers
    /// accordingly.  The next non-whitespace character will be set as
    /// the lookahead character.  Returns a boolean indicating whether
    /// EOF was not hit.
    pub fn skip_whitespace(&mut self) -> bool {
        let mut buffer = [0; 1];
        let mut eof = false;

        loop {
            if self.has_lookahead {
                buffer[0] = self.lookahead;
                self.has_lookahead = false;
                self.column_number = self.column_number + 1;
                if self.token_line_number == 0 {
                    self.token_line_number = self.line_number;
                    self.token_column_number = self.column_number;
                }
            } else {
                self.fh.read_exact(&mut buffer).unwrap_or_else(|e| {
                    if e.kind() == ErrorKind::UnexpectedEof {
                        eof = true;
                    } else {
                        eprintln!("unable to read from buffer!");
                        std::process::abort();
                    }
                });
            }
            if eof {
                return false;
            }
            match buffer[0] as char {
                '\n' => {
                    self.line_number = self.line_number + 1;
                    self.column_number = 1;
                }
                ' ' => {
                    self.column_number = self.column_number + 1;
                }
                '\t' => {
                    self.column_number = self.column_number + (self.column_number % 4);
                }
                _ => {
                    self.has_lookahead = true;
                    self.lookahead = buffer[0] as u8;
                    self.column_number = self.column_number - 1;
                    return true;
                }
            }
        }
    }

    /// Returns a new token object for the given token type.
    pub fn get_token(&self, token_type: TokenType) -> Token {
        return Token::new(token_type, self.token_line_number, self.token_column_number);
    }

    /// Scans the BufRead for the next token, and returns it.
    pub fn scan(&mut self) -> Token {
        let at_start_of_line = self.column_number == 1;

        if self.next_is_eof {
            self.next_is_eof = false;
            return Token::new(TokenType::Eof, self.line_number, self.column_number);
        }

        /* For storing the token as a whole. */
        let mut result = [0; 2048];
        /* The current index into which the next token character
         * should be written. */
        let mut result_index = 0;
        /* The buffer for reading a character from the input stream.
         * */
        let mut buffer = [0; 1];

        /* Whether the token is a string (i.e. can contain
         * whitespace). */
        let mut is_string = false;
        /* Whether token parsing is currently inside the string. */
        let mut in_string = false;
        /* The delimiter for the string (if applicable). */
        let mut string_delimiter = ' ';

        /* Skip whitespace, and deal with the first character of the
         * token. */

        let res = self.skip_whitespace();
        if res == false {
            return self.get_token(TokenType::Eof);
        }

        buffer[0] = self.lookahead;
        self.has_lookahead = false;
        self.column_number = self.column_number + 1;

        self.token_line_number = self.line_number;
        self.token_column_number = self.column_number;

        self.column_number = self.column_number + 1;

        if (buffer[0] as char == '"')
            || (buffer[0] as char == '\'')
            || (buffer[0] as char == '{')
            || (buffer[0] as char == '$')
        {
            string_delimiter = buffer[0] as char;
            in_string = true;
            is_string = true;
        } else {
            result[result_index] = buffer[0];
            result_index = result_index + 1;

            match buffer[0] as char {
                '#' => {
                    /* Treat this as a comment only if it occurs
                     * at the start of the line (whether after
                     * whitespace or not). */
                    if at_start_of_line {
                        let mut ignored = String::new();
                        self.fh.read_line(&mut ignored).unwrap();
                        self.line_number = self.line_number + 1;
                        self.column_number = 1;
                        return self.get_token(TokenType::Retry);
                    }
                }
                '(' => { return self.get_token(TokenType::StartList); },
                ')' => { return self.get_token(TokenType::EndList); },
                '[' => { return self.get_token(TokenType::LeftBracket); },
                ']' => { return self.get_token(TokenType::RightBracket); },
                _   => {}
            }
        }

        /* This loop is for getting the rest of the token. */

        let mut done = false;
        let mut brace_count = 0;
        let mut params: HashSet<char> = HashSet::new();
        while !done {
            let mut eof = false;
            self.fh.read_exact(&mut buffer).unwrap_or_else(|e| {
                if e.kind() == ErrorKind::UnexpectedEof {
                    eof = true;
                } else {
                    eprintln!("unable to read from buffer!");
                    std::process::abort();
                }
            });
            if eof {
                if result_index == 0 {
                    return self.get_token(TokenType::Eof);
                } else {
                    self.next_is_eof = true;
                    break;
                }
            }
            match buffer[0] as char {
                '\n' => {
                    self.line_number = self.line_number + 1;
                    self.column_number = 1;
                }
                ' ' => {
                    self.column_number = self.column_number + 1;
                }
                '\t' => {
                    self.column_number = self.column_number + (self.column_number % 4);
                }
                '(' => {
                    self.column_number = self.column_number + 1;
                    if result_index == 1 {
                        match result[0] as char {
                            'h' => { return self.get_token(TokenType::StartHash) },
                            's' => { return self.get_token(TokenType::StartSet); },
                            _ => {}
                        }
                    }
                }
                _ => {
                    self.column_number = self.column_number + 1;
                }
            }
            if in_string {
                if string_delimiter == '{' {
                    /* Commands may contain nested braces, which are
                     * used for value substitution, which is why there is
                     * extra processing here.  Commands may also have
                     * parameters attached to the end of them. */
                    if buffer[0] as char == '{' {
                        brace_count = brace_count + 1;
                    } else if buffer[0] as char == '}' {
                        brace_count = brace_count - 1;
                    }
                    if brace_count < 0 {
                        in_string = false;
                        done = true;
                        let params_opt = self.scan_parameters();
                        if !params_opt.is_none() {
                            params = params_opt.unwrap();
                        }
                    } else {
                        result[result_index] = buffer[0];
                        result_index = result_index + 1;
                    }
                } else if string_delimiter == '$' {
                    /* Uncaptured commands do not need to include a
                     * terminating delimiter. */
                    result[result_index] = buffer[0];
                    result_index = result_index + 1;
                } else if buffer[0] as char == string_delimiter {
                    if result_index > 0 && result[result_index - 1] as char == '\\' {
                        result[result_index - 1] = buffer[0];
                    } else {
                        in_string = false;
                        done = true;
                    }
                } else {
                    result[result_index] = buffer[0];
                    result_index = result_index + 1;
                }
            } else {
                match buffer[0] as char {
                    '\n' | ' ' | '\t' => {
                        done = true;
                    }
                    /* A token that ends in a right parenthesis or
                     * right bracket is stopped on the previous
                     * character, to allow for syntax like '(1 2 3)'.
                     * */
                    ')' => {
                        self.has_lookahead = true;
                        self.lookahead = buffer[0] as u8;
                        self.column_number = self.column_number - 1;
                        done = true;
                    }
                    ']' => {
                        self.has_lookahead = true;
                        self.lookahead = buffer[0] as u8;
                        self.column_number = self.column_number - 1;
                        done = true;
                    }
                    _ => {
                        if result_index >= 2048 {
                            eprintln!("token is too long (more than 2048 chars)");
                            return self.get_token(TokenType::Error);
                        }
                        result[result_index] = buffer[0];
                        result_index = result_index + 1;
                        let c = buffer[0] as char;
                        if result_index == 1 && (c == '{' || c == '}' || c == '[' || c == ']') {
                            done = true;
                        }
                    }
                }
            }
            /* Allow for the execution character ';' to occur after
             * whitespace. */
            if done && (buffer[0] as char) != '\n' {
                let res = self.skip_whitespace();
                if res == true && self.lookahead == ';' as u8 {
                    self.column_number = self.column_number + 1;
                    result[result_index] = self.lookahead;
                    self.has_lookahead = false;
                    result_index = result_index + 1;
                }
            }
        }

        if in_string {
            /* Uncaptured commands do not need to include a
             * terminating delimiter, so there is special handling for
             * them here. */
            if string_delimiter == '$' {
                result[result_index] = 0;
                let s_all = str::from_utf8(&result).unwrap();
                let s = &s_all[..result_index];
                return self.get_token(TokenType::CommandUncaptured(s.to_string()));
            } else {
                eprintln!(
                    "{}:{}: unterminated string literal",
                    self.token_line_number, self.token_column_number
                );
                return self.get_token(TokenType::Error);
            }
        }

        /* Determine whether the word is explicit or implicit.  An
         * explicit word terminates in the execution character, while
         * an implicit word terminates with a newline, the right
         * bracket of an anonymous function, or EOF. */
        let mut is_explicit_word = false;
        let mut is_implicit_word = false;
        if result_index > 1 && (result[result_index - 1] as char) == ';' {
            is_explicit_word = true;
            result_index = result_index - 1;
        }
        if (buffer[0] as char) == '\n'
            || self.next_is_eof
            || (self.has_lookahead && self.lookahead == (']' as u8))
        {
            is_implicit_word = true;
        }

        result[result_index] = 0;

        let s_all = str::from_utf8(&result).unwrap();
        let s = &s_all[..result_index];
        let token_type = if !is_string {
            match s {
                "h("   => TokenType::StartHash,
                "s("   => TokenType::StartSet,
                "("    => TokenType::StartList,
                ")"    => TokenType::EndList,
                "{"    => TokenType::LeftBrace,
                "}"    => TokenType::RightBrace,
                "["    => TokenType::LeftBracket,
                "]"    => TokenType::RightBracket,
                ":"    => TokenType::StartFunction,
                ":~"   => TokenType::StartGenerator,
                ",,"   => TokenType::EndFunction,
                ".t"   => TokenType::True,
                ".f"   => TokenType::False,
                "null" => TokenType::Null,
                _ => {
                    if INT.is_match(s) {
                        let n_res = s.to_string().parse::<i32>();
                        match n_res {
                            Ok(n) => TokenType::Int(n),
                            Err(_) => {
                                let n = s.to_string().parse::<num_bigint::BigInt>().unwrap();
                                TokenType::BigInt(n)
                            }
                        }
                    } else if FLOAT.is_match(s) {
                        TokenType::Float(s.to_string().parse::<f64>().unwrap())
                    } else if is_explicit_word {
                        TokenType::Word(s.to_string())
                    } else if is_implicit_word {
                        TokenType::WordImplicit(s.to_string())
                    } else {
                        TokenType::String(s.to_string())
                    }
                }
            }
        } else {
            if string_delimiter == '{' {
                if is_explicit_word || is_implicit_word {
                    TokenType::CommandExplicit(s.to_string(), params)
                } else {
                    TokenType::Command(s.to_string(), params)
                }
            } else {
                TokenType::String(s.to_string())
            }
        };

        return self.get_token(token_type);
    }
}

/// A Compiler compiles program code (by way of a scanner) into
/// bytecode, in the form of a chunk.
#[derive(Debug)]
pub struct Compiler {
    locals: Vec<Local>,
    scope_depth: u32,
}

/// Unescapes a single string value, by replacing string
/// representations of certain characters (e.g. "\n") with the actual
/// character.
pub fn unescape_string(s: &str) -> String {
    let mut s2 = String::from("");
    let mut next_escaped = false;
    for c in s.chars() {
        if next_escaped {
            match c {
                'n'  => { s2.push('\n'); },
                't'  => { s2.push('\t'); },
                'r'  => { s2.push('\r'); },
                '\\' => { s2.push('\\'); },
                _    => { s2.push('\\');
                          s2.push(c); }
            }
            next_escaped = false;
        } else {
            match c {
                '\\' => { next_escaped = true; }
                _    => { next_escaped = false;
                          s2.push(c); }
            }
        }
    }
    return s2;
}

impl Compiler {
    pub fn new() -> Compiler {
        Compiler {
            locals: Vec::new(),
            scope_depth: 0,
        }
    }

    /// Increases the scope depth.  Used when a new function is
    /// defined (whether a named function or an anonymous one).
    fn increase_scope_depth(&mut self) {
        self.scope_depth = self.scope_depth + 1;
    }

    /// Decreases the scope depth.  This adds appropriate pop opcodes
    /// for dealing with local variables that will no longer be in use
    /// after the scope depth is decreased.
    fn decrease_scope_depth(&mut self, chunk: &mut Chunk) {
        while self.locals.len() > 0 && (self.locals.last().unwrap().depth == self.scope_depth) {
            chunk.add_opcode(OpCode::PopLocalVar);
            self.locals.pop();
        }
        if self.scope_depth == 0 {
            eprintln!("scope depth is already zero!");
            std::process::abort();
        }
        self.scope_depth = self.scope_depth - 1;
    }

    /// Takes a scanner and a chunk as its arguments.  Reads tokens by
    /// way of the scanner, and compiles that token data into
    /// bytecode, which is added to the chunk.  Returns a boolean
    /// indicating whether compilation was successful.
    fn compile_inner(&mut self, scanner: &mut Scanner, chunk: &mut Chunk) -> bool {
        // Stores instruction indexes for various types of statement,
        // in order to be able to jump later.
        let mut if_indexes: Vec<(Option<usize>, Option<usize>)> = Vec::new();
        let mut if_index = None;
        let mut else_index = None;
        let mut begin_indexes: Vec<(Option<usize>, Vec<usize>)> = Vec::new();
        let mut begin_index = None;
        let mut leave_indexes: Vec<usize> = Vec::new();

        // The current anonymous function index.
        let mut anon_index = 0;

        // Whether this chunk has global variables.
        let mut has_vars = false;

        loop {
            let token = scanner.scan();

            chunk.set_next_point(token.line_number, token.column_number);
            let mut is_implicit = false;
            let token_type = token.token_type;
            match token_type {
                TokenType::WordImplicit(_) => {
                    is_implicit = true;
                }
                _ => {}
            }
            match token_type {
                TokenType::Retry => {
                    continue;
                }
                TokenType::StartList => {
                    chunk.add_opcode(OpCode::StartList);
                }
                TokenType::StartHash => {
                    chunk.add_opcode(OpCode::StartHash);
                }
                TokenType::StartSet => {
                    chunk.add_opcode(OpCode::StartSet);
                }
                TokenType::EndList => {
                    chunk.add_opcode(OpCode::EndList);
                }
                TokenType::Eof => {
                    break;
                }
                TokenType::Error => {
                    return false;
                }
                TokenType::StartGenerator => {
                    let name_token = scanner.scan();
                    let name_str: String;
                    match name_token.token_type {
                        TokenType::Word(s) => {
                            name_str = s;
                        }
                        TokenType::WordImplicit(s) => {
                            name_str = s;
                        }
                        TokenType::String(s) => {
                            name_str = s;
                        }
                        _ => {
                            eprintln!(
                                "{}:{}: expected name token",
                                name_token.line_number, name_token.column_number
                            );
                            return false;
                        }
                    }

                    let arg_count_token = scanner.scan();
                    let arg_count: i32;
                    match arg_count_token.token_type {
                        TokenType::Int(n) => {
                            arg_count = n;
                        }
                        _ => {
                            eprintln!(
                                "{}:{}: expected argument count token",
                                arg_count_token.line_number, arg_count_token.column_number
                            );
                            return false;
                        }
                    }

                    let req_arg_count_token = scanner.scan();
                    let req_arg_count: i32;
                    match req_arg_count_token.token_type {
                        TokenType::Int(n) => {
                            req_arg_count = n;
                        }
                        _ => {
                            eprintln!(
                                "{}:{}: expected required argument count token",
                                req_arg_count_token.line_number, req_arg_count_token.column_number
                            );
                            return false;
                        }
                    }

                    let mut generator_chunk =
                        Chunk::new_generator(chunk.name.to_string(), arg_count, req_arg_count);

                    self.increase_scope_depth();
                    let res = self.compile_inner(scanner, &mut generator_chunk);
                    if !res {
                        return false;
                    }
                    chunk
                        .functions
                        .insert(name_str, Rc::new(RefCell::new(generator_chunk)));
                }
                TokenType::StartFunction => {
                    let mut function_chunk = Chunk::new_standard(chunk.name.to_string());
                    let name_token = scanner.scan();
                    let name_str: String;
                    match name_token.token_type {
                        TokenType::Word(s) => {
                            name_str = s;
                        }
                        TokenType::WordImplicit(s) => {
                            name_str = s;
                        }
                        TokenType::String(s) => {
                            name_str = s;
                        }
                        _ => {
                            eprintln!(
                                "{}:{}: expected name token",
                                name_token.line_number, name_token.column_number
                            );
                            return false;
                        }
                    }
                    self.increase_scope_depth();
                    if self.scope_depth > 1 {
                        function_chunk.nested = true;
                        function_chunk.scope_depth = self.scope_depth;
                    }
                    let res = self.compile_inner(scanner, &mut function_chunk);
                    if !res {
                        return false;
                    }
                    chunk
                        .functions
                        .insert(name_str, Rc::new(RefCell::new(function_chunk)));
                }
                TokenType::EndFunction => {
                    self.decrease_scope_depth(chunk);
                    chunk.add_opcode(OpCode::EndFn);
                    if !has_vars {
                        chunk.has_vars = false;
                    }
                    return true;
                }
                TokenType::LeftBracket => {
                    let mut function_chunk = Chunk::new_standard(chunk.name.to_string());
                    if self.scope_depth > 0 {
                        function_chunk.nested = true;
                        function_chunk.scope_depth = self.scope_depth;
                    }
                    let name_str = format!("anon{}", anon_index);
                    anon_index = anon_index + 1;
                    self.increase_scope_depth();
                    let res = self.compile_inner(scanner, &mut function_chunk);
                    if !res {
                        return false;
                    }
                    let name_str_rr = Value::String(Rc::new(RefCell::new(StringTriple::new(
                        name_str.as_str().to_string(),
                        None,
                    ))));
                    let i = chunk.add_constant(name_str_rr);
                    chunk.add_opcode(OpCode::Function);
                    let i_upper = (i >> 8) & 0xFF;
                    let i_lower = i & 0xFF;
                    chunk.add_byte(i_upper as u8);
                    chunk.add_byte(i_lower as u8);
                    chunk
                        .functions
                        .insert(name_str, Rc::new(RefCell::new(function_chunk)));
                }
                TokenType::RightBracket => {
                    match chunk.get_second_last_opcode() {
                        OpCode::Constant => match chunk.get_last_opcode() {
                            OpCode::Call => {}
                            OpCode::CallImplicit => {}
                            _ => {
                                chunk.add_opcode(OpCode::CallImplicit);
                            }
                        },
                        _ => {}
                    }
                    self.decrease_scope_depth(chunk);
                    chunk.add_opcode(OpCode::EndFn);
                    if !has_vars {
                        chunk.has_vars = false;
                    }
                    return true;
                }
                TokenType::True => {
                    let value_rr = Value::Bool(true);
                    let i = chunk.add_constant(value_rr);
                    chunk.add_opcode(OpCode::Constant);
                    let i_upper = (i >> 8) & 0xFF;
                    let i_lower = i & 0xFF;
                    chunk.add_byte(i_upper as u8);
                    chunk.add_byte(i_lower as u8);
                }
                TokenType::False => {
                    let value_rr = Value::Bool(false);
                    let i = chunk.add_constant(value_rr);
                    chunk.add_opcode(OpCode::Constant);
                    let i_upper = (i >> 8) & 0xFF;
                    let i_lower = i & 0xFF;
                    chunk.add_byte(i_upper as u8);
                    chunk.add_byte(i_lower as u8);
                }
                TokenType::Int(n) => {
                    let value_rr = Value::Int(n);
                    let i = chunk.add_constant(value_rr);
                    chunk.add_opcode(OpCode::Constant);
                    let i_upper = (i >> 8) & 0xFF;
                    let i_lower = i & 0xFF;
                    chunk.add_byte(i_upper as u8);
                    chunk.add_byte(i_lower as u8);
                }
                TokenType::BigInt(n) => {
                    let value_rr = Value::BigInt(n);
                    let i = chunk.add_constant(value_rr);
                    chunk.add_opcode(OpCode::Constant);
                    let i_upper = (i >> 8) & 0xFF;
                    let i_lower = i & 0xFF;
                    chunk.add_byte(i_upper as u8);
                    chunk.add_byte(i_lower as u8);
                }
                TokenType::Float(n) => {
                    let value_rr = Value::Float(n);
                    let i = chunk.add_constant(value_rr);
                    chunk.add_opcode(OpCode::Constant);
                    let i_upper = (i >> 8) & 0xFF;
                    let i_lower = i & 0xFF;
                    chunk.add_byte(i_upper as u8);
                    chunk.add_byte(i_lower as u8);
                }
                TokenType::Word(s) | TokenType::WordImplicit(s) => {
                    if s == "+" {
                        match chunk.get_third_last_opcode() {
                            OpCode::Constant => {
                                let mlen = chunk.data.len() - 1;
                                chunk.set_previous_point(
                                    mlen,
                                    token.line_number,
                                    token.column_number,
                                );
                                chunk.set_third_last_opcode(OpCode::AddConstant);
                            }
                            _ => {
                                chunk.add_opcode(OpCode::Add);
                            }
                        }
                    } else if s == "-" {
                        match chunk.get_third_last_opcode() {
                            OpCode::Constant => {
                                let mlen = chunk.data.len() - 1;
                                chunk.set_previous_point(
                                    mlen,
                                    token.line_number,
                                    token.column_number,
                                );
                                chunk.set_third_last_opcode(OpCode::SubtractConstant);
                            }
                            _ => {
                                chunk.add_opcode(OpCode::Subtract);
                            }
                        }
                    } else if s == "*" {
                        match chunk.get_third_last_opcode() {
                            OpCode::Constant => {
                                let mlen = chunk.data.len() - 1;
                                chunk.set_previous_point(
                                    mlen,
                                    token.line_number,
                                    token.column_number,
                                );
                                chunk.set_third_last_opcode(OpCode::MultiplyConstant);
                            }
                            _ => {
                                chunk.add_opcode(OpCode::Multiply);
                            }
                        }
                    } else if s == "/" {
                        match chunk.get_third_last_opcode() {
                            OpCode::Constant => {
                                let mlen = chunk.data.len() - 1;
                                chunk.set_previous_point(
                                    mlen,
                                    token.line_number,
                                    token.column_number,
                                );
                                chunk.set_third_last_opcode(OpCode::DivideConstant);
                            }
                            _ => {
                                chunk.add_opcode(OpCode::Divide);
                            }
                        }
                    } else if s == ">" {
                        chunk.add_opcode(OpCode::Gt);
                    } else if s == "<" {
                        chunk.add_opcode(OpCode::Lt);
                    } else if s == "=" {
                        match chunk.get_third_last_opcode() {
                            OpCode::Constant => {
                                chunk.set_third_last_opcode(OpCode::EqConstant);
                            }
                            _ => {
                                chunk.add_opcode(OpCode::Eq);
                            }
                        }
                    } else if s == "var" {
                        if self.scope_depth == 0 {
                            chunk.add_opcode(OpCode::Var);
                            has_vars = true;
                        } else {
                            let last_constant_rr = chunk.get_last_constant();
                            chunk.pop_byte();
                            chunk.pop_byte();
                            let last_opcode = chunk.get_last_opcode();
                            let not_constant;
                            match last_opcode {
                                OpCode::Constant => {
                                    not_constant = false;
                                }
                                _ => {
                                    not_constant = true;
                                }
                            }
                            if not_constant {
                                eprintln!(
                                    "{}:{}: variable name must precede var",
                                    token.line_number, token.column_number
                                );
                                return false;
                            }
                            chunk.pop_byte();
                            match last_constant_rr {
                                Value::String(sp) => {
                                    let local = Local::new(
                                        sp.borrow().s.to_string(),
                                        self.scope_depth.into(),
                                    );
                                    self.locals.push(local);
                                }
                                _ => {
                                    eprintln!(
                                        "{}:{}: variable name must be a string",
                                        token.line_number, token.column_number
                                    );
                                    return false;
                                }
                            }
                            let value_rr = Value::Int(0);
                            let i = chunk.add_constant(value_rr);
                            chunk.add_opcode(OpCode::Constant);
                            let i_upper = (i >> 8) & 0xFF;
                            let i_lower = i & 0xFF;
                            chunk.add_byte(i_upper as u8);
                            chunk.add_byte(i_lower as u8);
                            chunk.add_opcode(OpCode::SetLocalVar);
                            chunk.add_byte((self.locals.len() - 1) as u8);
                        }
                    } else if s == "!" {
                        let last_constant_rr = chunk.get_last_constant();
                        chunk.pop_byte();
                        chunk.pop_byte();
                        let last_opcode = chunk.get_last_opcode();
                        let not_constant;
                        match last_opcode {
                            OpCode::Constant => {
                                not_constant = false;
                            }
                            _ => {
                                not_constant = true;
                            }
                        }
                        if not_constant {
                            eprintln!(
                                "{}:{}: variable name must precede !",
                                token.line_number, token.column_number
                            );
                            return false;
                        }
                        chunk.pop_byte();
                        let mut success = false;
                        {
                            match last_constant_rr {
                                Value::String(ref sp) => {
                                    if self.locals.len() != 0 {
                                        let mut i = self.locals.len() - 1;
                                        loop {
                                            let local = &self.locals[i];
                                            if local.name.eq(&sp.borrow().s) {
                                                chunk.add_opcode(OpCode::SetLocalVar);
                                                chunk.add_byte(i as u8);
                                                success = true;
                                                break;
                                            }
                                            if i == 0 {
                                                break;
                                            }
                                            i = i - 1;
                                        }
                                    }
                                }
                                _ => {
                                    eprintln!(
                                        "{}:{}: variable name must be a string",
                                        token.line_number, token.column_number
                                    );
                                    return false;
                                }
                            }
                        }
                        if !success {
                            let i = chunk.add_constant(last_constant_rr);
                            chunk.add_opcode(OpCode::Constant);
                            let i_upper = (i >> 8) & 0xFF;
                            let i_lower = i & 0xFF;
                            chunk.add_byte(i_upper as u8);
                            chunk.add_byte(i_lower as u8);
                            chunk.add_opcode(OpCode::SetVar);
                        }
                    } else if s == "@" {
                        let last_constant_rr = chunk.get_last_constant();
                        chunk.pop_byte();
                        chunk.pop_byte();
                        let last_opcode = chunk.get_last_opcode();
                        chunk.pop_byte();
                        let not_constant;
                        match last_opcode {
                            OpCode::Constant => {
                                not_constant = false;
                            }
                            _ => {
                                not_constant = true;
                            }
                        }
                        if not_constant {
                            eprintln!(
                                "{}:{}: variable name must precede @",
                                token.line_number, token.column_number
                            );
                            return false;
                        }
                        let mut success = false;
                        {
                            match last_constant_rr {
                                Value::String(ref sp) => {
                                    if self.locals.len() != 0 {
                                        let mut i = self.locals.len() - 1;
                                        loop {
                                            let local = &self.locals[i];
                                            if local.name.eq(&sp.borrow().s) {
                                                chunk.add_opcode(OpCode::GetLocalVar);
                                                chunk.add_byte(i as u8);
                                                success = true;
                                                break;
                                            }
                                            if i == 0 {
                                                break;
                                            }
                                            i = i - 1;
                                        }
                                    }
                                }
                                _ => {
                                    eprintln!(
                                        "{}:{}: variable name must be a string",
                                        token.line_number, token.column_number
                                    );
                                    return false;
                                }
                            }
                        }
                        if !success {
                            let i = chunk.add_constant(last_constant_rr);
                            chunk.add_opcode(OpCode::Constant);
                            let i_upper = (i >> 8) & 0xFF;
                            let i_lower = i & 0xFF;
                            chunk.add_byte(i_upper as u8);
                            chunk.add_byte(i_lower as u8);
                            chunk.add_opcode(OpCode::GetVar);
                        }
                    } else if s == ".s" {
                        chunk.add_opcode(OpCode::PrintStack);
                    } else if s == "error" {
                        chunk.add_opcode(OpCode::Error);
                    } else if s == "print" {
                        chunk.add_opcode(OpCode::Print);
                    } else if s == "drop" {
                        chunk.add_opcode(OpCode::Drop);
                    } else if s == "funcall" {
                        let mut done = false;
                        match chunk.get_second_last_opcode() {
                            OpCode::GetLocalVar => {
                                chunk.set_second_last_opcode(
                                    OpCode::GLVCall
                                );
                                done = true;
                            }
                            _ => {}
                        }
                        if !done {
                            chunk.add_opcode(OpCode::Call);
                        }
                    } else if s == "shift" {
                        let mut done = false;
                        match chunk.get_second_last_opcode() {
                            OpCode::GetLocalVar => {
                                chunk.set_second_last_opcode(
                                    OpCode::GLVShift
                                );
                                done = true;
                            }
                            _ => {}
                        }
                        if !done {
                            chunk.add_opcode(OpCode::Shift);
                        }
                    } else if s == "yield" {
                        chunk.add_opcode(OpCode::Yield);
                    } else if s == "clear" {
                        chunk.add_opcode(OpCode::Clear);
                    } else if s == "dup" {
                        chunk.add_opcode(OpCode::Dup);
                    } else if s == "swap" {
                        chunk.add_opcode(OpCode::Swap);
                    } else if s == "rot" {
                        chunk.add_opcode(OpCode::Rot);
                    } else if s == "depth" {
                        chunk.add_opcode(OpCode::Depth);
                    } else if s == "over" {
                        chunk.add_opcode(OpCode::Over);
                    } else if s == "is-null" {
                        match chunk.get_last_opcode() {
                            OpCode::Dup => {
                                chunk.set_last_opcode(OpCode::DupIsNull);
                            }
                            _ => {
                                chunk.add_opcode(OpCode::IsNull);
                            }
                        }
                    } else if s == "is-list" {
                        chunk.add_opcode(OpCode::IsList);
                    } else if s == "is-callable" {
                        chunk.add_opcode(OpCode::IsCallable);
                    } else if s == "is-shiftable" {
                        chunk.add_opcode(OpCode::IsShiftable);
                    } else if s == "toggle-mode" {
                        chunk.add_opcode(OpCode::ToggleMode);
                    } else if s == "to-function" {
                        chunk.add_opcode(OpCode::ToFunction);
                    } else if s == "import" {
                        chunk.add_opcode(OpCode::Import);
                    } else if s == "clone" {
                        chunk.add_opcode(OpCode::Clone);
                    } else if s == "open" {
                        chunk.add_opcode(OpCode::Open);
                    } else if s == "readline" {
                        chunk.add_opcode(OpCode::Readline);
                    } else if s == "begin-scope" {
                        self.increase_scope_depth();
                    } else if s == "end-scope" {
                        self.decrease_scope_depth(chunk);
                    } else if s == "push" {
                        chunk.add_opcode(OpCode::Push);
                    } else if s == "pop" {
                        chunk.add_opcode(OpCode::Pop);
                    } else if s == "if" {
                        chunk.add_opcode(OpCode::JumpNe);
                        match if_index {
                            Some(_) => {
                                if_indexes.push((if_index, else_index));
                            }
                            _ => {}
                        }
                        if_index = Some(chunk.data.len());
                        else_index = None;
                        chunk.add_byte(0);
                        chunk.add_byte(0);
                    } else if s == "then" {
                        let mut has_else = false;
                        match else_index {
                            Some(n) => {
                                let jmp_len = chunk.data.len() - n - 2;
                                chunk.data[n] =
                                    ((jmp_len >> 8) & 0xff).try_into().unwrap();
                                chunk.data[n + 1] =
                                    (jmp_len & 0xff).try_into().unwrap();
                                has_else = true;
                                else_index = None;
                            }
                            _ => {}
                        }
                        if !has_else {
                            match if_index {
                                Some(n) => {
                                    let jmp_len = chunk.data.len() - n - 2;
                                    chunk.data[n] =
                                        ((jmp_len >> 8) & 0xff).try_into().unwrap();
                                    chunk.data[n + 1] =
                                        (jmp_len & 0xff).try_into().unwrap();
                                    if_index = None;
                                }
                                _ => {
                                    eprintln!(
                                        "{}:{}: 'then' without 'if'",
                                        token.line_number, token.column_number
                                    );
                                    return false;
                                }
                            }
                        }
                        if if_indexes.len() >= 1 {
                            let (prev_if_index, prev_else_index) = if_indexes.pop().unwrap();
                            if_index = prev_if_index;
                            else_index = prev_else_index;
                        }
                    } else if s == "else" {
                        chunk.add_opcode(OpCode::Jump);
                        match else_index {
                            Some(_) => {
                                eprintln!(
                                    "{}:{}: multiple 'else'",
                                    token.line_number, token.column_number
                                );
                                return false;
                            }
                            _ => {}
                        }
                        else_index = Some(chunk.data.len());
                        chunk.add_byte(0);
                        chunk.add_byte(0);
                        match if_index {
                            Some(n) => {
                                let jmp_len = chunk.data.len() - n - 2;
                                chunk.data[n] =
                                    ((jmp_len >> 8) & 0xff).try_into().unwrap();
                                chunk.data[n + 1] =
                                    (jmp_len & 0xff).try_into().unwrap();
                            }
                            _ => {
                                eprintln!(
                                    "{}:{}: 'else' without 'if'",
                                    token.line_number, token.column_number
                                );
                                return false;
                            }
                        }
                    } else if s == "begin" {
                        match begin_index {
                            Some(_) => {
                                begin_indexes.push((begin_index, leave_indexes));
                                leave_indexes = Vec::new();
                            }
                            _ => {}
                        }
                        begin_index = Some(chunk.data.len());
                    } else if s == "leave" {
                        match begin_index {
                            Some(_) => {
                                chunk.add_opcode(OpCode::Jump);
                                leave_indexes.push(chunk.data.len());
                                chunk.add_byte(0);
                                chunk.add_byte(0);
                            }
                            _ => {
                                eprintln!(
                                    "{}:{}: 'leave' without 'begin'",
                                    token.line_number, token.column_number
                                );
                                return false;
                            }
                        }
                    } else if s == "until" {
                        match begin_index {
                            Some(n) => {
                                let mut done = false;
                                match (
                                    chunk.get_third_last_opcode(),
                                    chunk.get_fourth_last_opcode(),
                                ) {
                                    (OpCode::EqConstant, OpCode::Dup) => {
                                        chunk.set_fourth_last_opcode(OpCode::JumpNeREqC);
                                        let cb1 = chunk.get_second_last_byte();
                                        let cb2 = chunk.get_last_byte();
                                        let i3 =
                                            (((cb1 as u16) << 8) & 0xFF00) | ((cb2 & 0xFF) as u16);
                                        if chunk.has_constant_int(i3 as i32) {
                                            let jmp_len = chunk.data.len() - n + 1;
                                            chunk.set_third_last_byte(
                                                ((jmp_len >> 8) & 0xff).try_into().unwrap(),
                                            );
                                            chunk.set_second_last_byte(
                                                (jmp_len & 0xff).try_into().unwrap(),
                                            );
                                            chunk.set_last_byte(cb1);
                                            chunk.add_byte(cb2);
                                            done = true;
                                        }
                                    }
                                    _ => {}
                                };
                                if !done {
                                    let mut done2 = false;
                                    match chunk.get_third_last_opcode() {
                                        OpCode::Constant => {
                                            let i_upper = chunk.get_second_last_byte();
                                            let i_lower = chunk.get_last_byte();
                                            let constant_i = (((i_upper as u16) << 8) & 0xFF00)
                                                | ((i_lower & 0xFF) as u16);
                                            let v = chunk.get_constant(constant_i.into());
                                            match v {
                                                Value::Int(0) => {
                                                    chunk.set_third_last_opcode(OpCode::JumpR);
                                                    let jmp_len = chunk.data.len() - n;
                                                    chunk.set_second_last_byte(
                                                        ((jmp_len >> 8) & 0xff).try_into().unwrap(),
                                                    );
                                                    chunk.set_last_byte(
                                                        (jmp_len & 0xff).try_into().unwrap(),
                                                    );
                                                    done2 = true;
                                                }
                                                _ => {}
                                            }
                                        }
                                        _ => {}
                                    };
                                    if !done2 {
                                        chunk.add_opcode(OpCode::JumpNeR);
                                        let jmp_len = chunk.data.len() - n + 2;
                                        chunk.add_byte(((jmp_len >> 8) & 0xff).try_into().unwrap());
                                        chunk.add_byte((jmp_len & 0xff).try_into().unwrap());
                                    }
                                }
                            }
                            _ => {
                                eprintln!(
                                    "{}:{}: 'until' without 'begin'",
                                    token.line_number, token.column_number
                                );
                                return false;
                            }
                        }
                        for leave_index in leave_indexes.iter() {
                            let jmp_len = chunk.data.len() - *leave_index - 2;
                            chunk.data[*leave_index] =
                                ((jmp_len >> 8) & 0xff).try_into().unwrap();
                            chunk.data[*leave_index + 1] =
                                (jmp_len & 0xff).try_into().unwrap();
                        }
                        if begin_indexes.len() > 0 {
                            let (prev_begin_index, prev_leave_indexes) =
                                begin_indexes.pop().unwrap();
                            begin_index = prev_begin_index;
                            leave_indexes = prev_leave_indexes;
                        }
                    } else if s == "return" {
                        chunk.add_opcode(OpCode::Return);
                    } else if s == "str" {
                        chunk.add_opcode(OpCode::Str);
                    } else if s == "int" {
                        chunk.add_opcode(OpCode::Int);
                    } else if s == "flt" {
                        chunk.add_opcode(OpCode::Flt);
                    } else if s == "bool" {
                        chunk.add_opcode(OpCode::Bool);
                    } else if s == "rand" {
                        chunk.add_opcode(OpCode::Rand);
                    } else {
                        let s_raw = unescape_string(&s);
                        let s_rr =
                            Value::String(Rc::new(RefCell::new(StringTriple::new_with_escaped(s_raw, s, None))));
                        let i = chunk.add_constant(s_rr);

                        if is_implicit {
                            chunk.add_opcode(OpCode::CallImplicitConstant);
                            let i_upper = (i >> 8) & 0xFF;
                            let i_lower = i & 0xFF;
                            chunk.add_byte(i_upper as u8);
                            chunk.add_byte(i_lower as u8);
                        } else {
                            chunk.add_opcode(OpCode::CallConstant);
                            let i_upper = (i >> 8) & 0xFF;
                            let i_lower = i & 0xFF;
                            chunk.add_byte(i_upper as u8);
                            chunk.add_byte(i_lower as u8);
                        }
                    }
                }
                TokenType::Command(s, params) => {
                    let s_raw = unescape_string(&s);
                    let s_rr = Value::Command(Rc::new(s_raw),
                                              Rc::new(params));
                    let i = chunk.add_constant(s_rr);
                    chunk.add_opcode(OpCode::Constant);
                    let i_upper = (i >> 8) & 0xFF;
                    let i_lower = i & 0xFF;
                    chunk.add_byte(i_upper as u8);
                    chunk.add_byte(i_lower as u8);
                }
                TokenType::CommandUncaptured(s) => {
                    let s_raw = unescape_string(&s);
                    let s_rr = Value::CommandUncaptured(Rc::new(s_raw));
                    let i = chunk.add_constant(s_rr);
                    chunk.add_opcode(OpCode::Constant);
                    let i_upper = (i >> 8) & 0xFF;
                    let i_lower = i & 0xFF;
                    chunk.add_byte(i_upper as u8);
                    chunk.add_byte(i_lower as u8);
                    chunk.add_opcode(OpCode::Call);
                }
                TokenType::CommandExplicit(s, params) => {
                    let s_raw = unescape_string(&s);
                    let s_rr = Value::Command(Rc::new(s_raw),
                                              Rc::new(params));
                    let i = chunk.add_constant(s_rr);
                    chunk.add_opcode(OpCode::Constant);
                    let i_upper = (i >> 8) & 0xFF;
                    let i_lower = i & 0xFF;
                    chunk.add_byte(i_upper as u8);
                    chunk.add_byte(i_lower as u8);
                    chunk.add_opcode(OpCode::Call);
                }
                TokenType::String(s) => {
                    let s_raw = unescape_string(&s);
                    let s_rr =
                        Value::String(Rc::new(RefCell::new(StringTriple::new_with_escaped(s_raw, s, None))));
                    let i = chunk.add_constant(s_rr);
                    chunk.add_opcode(OpCode::Constant);
                    let i_upper = (i >> 8) & 0xFF;
                    let i_lower = i & 0xFF;
                    chunk.add_byte(i_upper as u8);
                    chunk.add_byte(i_lower as u8);
                }
                TokenType::Null => {
                    let i = chunk.add_constant(Value::Null);
                    chunk.add_opcode(OpCode::Constant);
                    let i_upper = (i >> 8) & 0xFF;
                    let i_lower = i & 0xFF;
                    chunk.add_byte(i_upper as u8);
                    chunk.add_byte(i_lower as u8);
                }
                _ => {
                    eprintln!(
                        "{}:{}: unhandled token!",
                        token.line_number, token.column_number
                    );
                    std::process::abort();
                }
            }
        }

        if !has_vars {
            chunk.has_vars = false;
        }

        return true;
    }

    /// Takes a BufRead and a chunk name as its arguments.  Compiles
    /// the program code found in the BufRead, and returns a chunk
    /// containing the compiled code.
    pub fn compile(&mut self, fh: &mut Box<dyn BufRead>, name: &str) -> Option<Chunk> {
        let mut scanner = Scanner::new(fh);
        let mut chunk = Chunk::new_standard(name.to_string());
        let res = self.compile_inner(&mut scanner, &mut chunk);
        if !res {
            return None;
        } else {
            return Some(chunk);
        }
    }

    /// Takes a chunk and a file object as its arguments.  Serialises
    /// the chunk to the file.
    pub fn serialise(&mut self, chunk: &Chunk, file: &mut std::fs::File) -> bool {
        let encoded_res = bincode::serialize(&chunk);
        match encoded_res {
            Ok(encoded) => {
                let res = file.write_all(&encoded);
                match res {
                    Ok(_) => {
                        return true;
                    }
                    Err(e) => {
                        eprintln!("unable to write compiled file: {}", e);
                        return false;
                    }
                }
            }
            Err(e) => {
                eprintln!("unable to serialise compiled file: {}", e);
                return false;
            }
        }
    }

    /// Takes a file path as its single argument.  Deserialises a
    /// chunk from that file and returns the chunk.
    pub fn deserialise(&mut self, file: &str) -> Option<Chunk> {
        let data_res = fs::read(file);
        match data_res {
            Ok(data) => {
                let data_chars: &[u8] = &&data;
                let decoded_res = bincode::deserialize(data_chars);
                match decoded_res {
                    Ok(decoded) => {
                        return Some(decoded);
                    }
                    Err(_) => {
                        return None;
                    }
                }
            }
            Err(_) => {
                return None;
            }
        }
    }
}

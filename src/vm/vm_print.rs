use std::convert::TryInto;
use std::io;
use std::io::Write;
use std::str;

use termion::input::TermRead;
use termion::raw::IntoRawMode;

use chunk::{Chunk, Value};
use vm::*;

/// Unescapes a single string value, by replacing certain
/// characters (like newline) with string representations.
fn unescape_string(s: &str) -> String {
    let s1 = s.replace("\n", "\\n");
    let s2 = s1.replace("\"", "\\\"");
    let s3 = s2.replace("\'", "\\\'");
    return s3;
}

/// Helper function for print_stack_value.  Takes a string, an indent
/// count, whether the first indent needs to be skipped, the window
/// height, and the number of lines that can be printed without
/// waiting as its arguments.  Prints the string to standard output,
/// waiting for user input as required.  Returns the new number of
/// lines that can be printed without waiting.  Returns -1 if the user
/// cancels further output.  (A window height of zero indicates that
/// the current program is not being run interactively, in which case
/// no waiting is required.)
fn psv_helper(
    s: &str,
    indent: i32,
    no_first_indent: bool,
    window_height: i32,
    mut lines_to_print: i32,
) -> i32 {
    if window_height != 0 {
        if lines_to_print == 0 {
            let mut stdout = io::stdout().into_raw_mode().unwrap();
            let stdin = std::io::stdin();
            for c in stdin.keys() {
                match c.unwrap() {
                    termion::event::Key::Char('q') => {
                        stdout.suspend_raw_mode().unwrap();
                        return -1;
                    }
                    termion::event::Key::Ctrl('c') => {
                        stdout.suspend_raw_mode().unwrap();
                        return -1;
                    }
                    termion::event::Key::PageDown => {
                        lines_to_print = lines_to_print + window_height;
                    }
                    _ => {
                        lines_to_print = lines_to_print + 1;
                    }
                }
                stdout.flush().unwrap();
                break;
            }
            stdout.suspend_raw_mode().unwrap();
        }
    }
    if !no_first_indent {
        for _ in 0..indent {
            print!(" ");
        }
    }
    print!("{}\n", s);
    return lines_to_print - 1;
}

impl VM {
    /// Takes a value that can be stringified as its single argument,
    /// and prints that value to standard output.
    pub fn opcode_print(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("print requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
	let value_opt: Option<&str>;
	to_str!(value_rr, value_opt);

        match value_opt {
            Some(s) => {
                print!("{}", s);
                return 1;
            }
            _ => {
                self.print_error("print argument must be a string");
                return 0;
            }
        }
    }

    /// Takes a value that can be stringified as its single argument,
    /// and prints that value followed by newline to standard output.
    pub fn core_println(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("println requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
	let value_opt: Option<&str>;
	to_str!(value_rr, value_opt);

        match value_opt {
            Some(s) => {
                println!("{}", s);
                return 1;
            }
            _ => {
                self.print_error("println argument must be a string");
                return 0;
            }
        }
    }

    /// Used by print_stack to print a single stack value.  Takes a
    /// wrapped value, the current chunk, the instruction index, the
    /// map of global functions, the current indent, the window height
    /// (if run interactively), and the number of lines that can be
    /// printed without waiting for user input as its arguments.
    /// Prints the stack value to standard output, returning the new
    /// number of lines that can be printed without waiting for user
    /// input.
    fn print_stack_value<'a>(
        &mut self,
        value_rr: &Value,
        chunk: Rc<RefCell<Chunk>>,
        i: usize,
        indent: i32,
        no_first_indent: bool,
        window_height: i32,
        mut lines_to_print: i32,
    ) -> i32 {
        let mut is_generator = false;
        {
            match value_rr {
                Value::Ipv4(_) => {
                    let s = format!("{{IP}}");
                    lines_to_print =
                        psv_helper(&s, indent, no_first_indent, window_height, lines_to_print);
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                Value::Ipv6(_) => {
                    let s = format!("{{IP}}");
                    lines_to_print =
                        psv_helper(&s, indent, no_first_indent, window_height, lines_to_print);
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                Value::DateTimeNT(_) => {
                    let s = format!("{{DateTime}}");
                    lines_to_print =
                        psv_helper(&s, indent, no_first_indent, window_height, lines_to_print);
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                Value::DateTimeOT(_) => {
                    let s = format!("{{DateTime}}");
                    lines_to_print =
                        psv_helper(&s, indent, no_first_indent, window_height, lines_to_print);
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                // The way this works is less than ideal, what with it
                // being different from standard stringification, but
                // it may be that having separate representations is
                // useful for some reason.
                Value::CoreFunction(_) => {
                    let s = format!("{{CoreFunction}}");
                    lines_to_print =
                        psv_helper(&s, indent, no_first_indent, window_height, lines_to_print);
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                Value::NamedFunction(_) => {
                    let s = format!("{{NamedFunction}}");
                    lines_to_print =
                        psv_helper(&s, indent, no_first_indent, window_height, lines_to_print);
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                Value::Null => {
                    lines_to_print = psv_helper(
                        "{{Null}}",
                        indent,
                        no_first_indent,
                        window_height,
                        lines_to_print,
                    );
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                Value::Bool(b) => {
                    let s = if *b { ".t" } else { ".f" };
                    lines_to_print =
                        psv_helper(&s, indent, no_first_indent,
                            window_height, lines_to_print);
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                Value::Int(n) => {
                    let s = format!("{}", n);
                    lines_to_print =
                        psv_helper(&s, indent, no_first_indent, window_height, lines_to_print);
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                Value::BigInt(n) => {
                    let s = format!("{}", n);
                    lines_to_print =
                        psv_helper(&s, indent, no_first_indent, window_height, lines_to_print);
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                Value::String(sp) => {
                    let mut ss = unescape_string(&sp.borrow().s);
                    if sp.borrow().s.contains(char::is_whitespace) {
                        ss = format!("\"{}\"", ss);
                    } else if ss.len() == 0 {
                        ss = format!("\"\"");
                    } else if ss == ".t" {
                        ss = format!("\".t\"");
                    } else if ss == ".f" {
                        ss = format!("\".f\"");
                    } else {
                        ss = format!("{}", ss);
                    }
                    lines_to_print =
                        psv_helper(&ss, indent, no_first_indent, window_height, lines_to_print);
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                Value::Command(s) => {
                    let s = format!("{{{}}}", s);
                    lines_to_print =
                        psv_helper(&s, indent, no_first_indent, window_height, lines_to_print);
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                Value::CommandUncaptured(s) => {
                    let s = format!("{{{}}}", s);
                    lines_to_print =
                        psv_helper(&s, indent, no_first_indent, window_height, lines_to_print);
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                Value::Float(f) => {
                    let s = format!("{}", f);
                    lines_to_print =
                        psv_helper(&s, indent, no_first_indent, window_height, lines_to_print);
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                Value::AnonymousFunction(_, _) => {
                    let s = format!("{{Function}}");
                    lines_to_print =
                        psv_helper(&s, indent, no_first_indent, window_height, lines_to_print);
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                Value::FileReader(_) => {
                    lines_to_print = psv_helper(
                        "{{FileReader}}",
                        indent,
                        no_first_indent,
                        window_height,
                        lines_to_print,
                    );
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                Value::FileWriter(_) => {
                    lines_to_print = psv_helper(
                        "{{FileWriter}}",
                        indent,
                        no_first_indent,
                        window_height,
                        lines_to_print,
                    );
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                Value::DirectoryHandle(_) => {
                    lines_to_print = psv_helper(
                        "{{DirectoryHandle}}",
                        indent,
                        no_first_indent,
                        window_height,
                        lines_to_print,
                    );
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                }
                Value::List(list) => {
                    if list.borrow().len() == 0 {
                        lines_to_print = psv_helper(
                            "()",
                            indent,
                            no_first_indent,
                            window_height,
                            lines_to_print,
                        );
                        if lines_to_print == -1 {
                            return lines_to_print;
                        }
                    } else {
                        lines_to_print =
                            psv_helper("(", indent, no_first_indent, window_height, lines_to_print);
                        if lines_to_print == -1 {
                            return lines_to_print;
                        }
                        let new_indent = indent + 4;
                        for element in list.borrow().iter() {
                            lines_to_print = self.print_stack_value(
                                element,
                                chunk.clone(),
                                i,
                                new_indent,
                                false,
                                window_height,
                                lines_to_print,
                            );
                            if lines_to_print == -1 {
                                return lines_to_print;
                            }
                        }
                        lines_to_print =
                            psv_helper(")", indent, false, window_height, lines_to_print);
                        if lines_to_print == -1 {
                            return lines_to_print;
                        }
                    }
                }
                Value::Hash(map) => {
                    if map.borrow().len() == 0 {
                        lines_to_print = psv_helper(
                            "h()",
                            indent,
                            no_first_indent,
                            window_height,
                            lines_to_print,
                        );
                        if lines_to_print == -1 {
                            return lines_to_print;
                        }
                    } else {
                        lines_to_print = psv_helper(
                            "h(",
                            indent,
                            no_first_indent,
                            window_height,
                            lines_to_print,
                        );
                        if lines_to_print == -1 {
                            return lines_to_print;
                        }

                        let mut key_maxlen = 0;
                        for (k, _) in map.borrow().iter() {
                            let key_len = k.len();
                            if key_len > key_maxlen {
                                key_maxlen = key_len;
                            }
                        }

                        let new_indent = indent + 4;
                        for (k, v) in map.borrow().iter() {
                            for _ in 0..new_indent {
                                print!(" ");
                            }
                            print!("\"{}\": ", k);
                            let extra_spaces = key_maxlen - k.len();
                            for _ in 0..extra_spaces {
                                print!(" ");
                            }

                            lines_to_print = self.print_stack_value(
                                v,
                                chunk.clone(),
                                i,
                                new_indent,
                                true,
                                window_height,
                                lines_to_print,
                            );
                            if lines_to_print == -1 {
                                return lines_to_print;
                            }
                        }
                        lines_to_print =
                            psv_helper(")", indent, false, window_height, lines_to_print);
                        if lines_to_print == -1 {
                            return lines_to_print;
                        }
                    }
                }
                Value::Generator(_) => {
                    is_generator = true;
                }
                Value::CommandGenerator(_) => {
                    is_generator = true;
                }
                Value::KeysGenerator(_) => {
                    is_generator = true;
                }
                Value::ValuesGenerator(_) => {
                    is_generator = true;
                }
                Value::EachGenerator(_) => {
                    is_generator = true;
                }
            }
        }
        if is_generator {
            let mut has_elements = false;
            self.stack.push(value_rr.clone());
            loop {
                let dup_res = self.opcode_dup();
                if dup_res == 0 {
                    return lines_to_print;
                }
                let shift_res = self.opcode_shift();
                if shift_res == 0 {
                    self.stack.pop();
                    return lines_to_print;
                }
                if self.stack.len() == 0 {
                    break;
                }
                let is_null;
                let value_rr = self.stack.pop().unwrap();
                {
                    match value_rr {
                        Value::Null => {
                            is_null = true;
                        }
                        _ => {
                            is_null = false;
                        }
                    }
                }
                if !is_null {
                    if !has_elements {
                        lines_to_print =
                            psv_helper("(", indent, no_first_indent, window_height, lines_to_print);
                        if lines_to_print == -1 {
                            return lines_to_print;
                        }
                        has_elements = true;
                    }
                    lines_to_print = self.print_stack_value(
                        &value_rr,
                        chunk.clone(),
                        i,
                        indent + 4,
                        false,
                        window_height,
                        lines_to_print,
                    );
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                } else {
                    break;
                }
            }
            self.stack.pop();
            if !has_elements {
                lines_to_print =
                    psv_helper("()", indent, no_first_indent, window_height, lines_to_print);
            } else {
                lines_to_print = psv_helper(")", indent, false, window_height, lines_to_print);
            }
            if lines_to_print == -1 {
                return lines_to_print;
            }
        }
        return lines_to_print;
    }

    /// Takes the current chunk, the instruction index, the map of
    /// global functions, and a boolean indicating whether the stack
    /// needs to be cleared after the stack is printed.  Prints the
    /// stack to standard output.
    pub fn print_stack<'a>(
        &mut self,
        chunk: Rc<RefCell<Chunk>>,
        i: usize,
        no_remove: bool,
    ) {
        let mut window_height: i32 = 0;
        let dim_opt = term_size::dimensions();
        match dim_opt {
            Some((_, h)) => {
                window_height = h.try_into().unwrap();
            }
            _ => {}
        }
        let mut lines_to_print = window_height - 1;

        let mut stack_backup = Vec::new();
        while self.stack.len() > 0 {
            let value_rr = self.stack.remove(0);
            lines_to_print = self.print_stack_value(
                &value_rr,
                chunk.clone(),
                i,
                0,
                false,
                window_height,
                lines_to_print,
            );
            if lines_to_print == -1 {
                if !no_remove {
                    self.stack.clear();
                }
                return;
            }
            stack_backup.push(value_rr);
        }
        if no_remove {
            self.stack = stack_backup;
        }
    }
}

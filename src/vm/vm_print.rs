use std::convert::TryInto;
use std::io;
use std::io::Write;
use std::str;

use atty::Stream;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use unicode_segmentation::UnicodeSegmentation;

use chunk::{Chunk, Value};
use vm::*;

/// Helper function for paging once the line limit has been reached.
fn pager_input(window_height: i32,
               mut lines_to_print: i32) -> i32 {
    if (window_height <= 0)
            || !atty::is(Stream::Stdout) {
        return 1;
    }
    if lines_to_print > 0 {
        return lines_to_print;
    }

    let mut stdout = io::stdout().into_raw_mode().unwrap();
    let stdin = std::io::stdin();
    for c in stdin.keys() {
        match c {
            Ok(termion::event::Key::Char('q')) => {
                stdout.suspend_raw_mode().unwrap();
                return -1;
            }
            Ok(termion::event::Key::Ctrl('c')) => {
                stdout.suspend_raw_mode().unwrap();
                return -1;
            }
            Ok(termion::event::Key::PageDown) => {
                lines_to_print += window_height - 1;
            }
            Ok(termion::event::Key::End) => {
                /* todo: a bit of a hack.  It would be better
                 * if there were some way of indicating that
                 * there's no need to wait on input if End is
                 * pressed. */
                lines_to_print = i32::MAX;
            }
            /* The default behaviour for these two might be
             * confusing, so make them no-ops. */
            Ok(termion::event::Key::Home) => {
                continue;
            }
            Ok(termion::event::Key::PageUp) => {
                continue;
            }
            Ok(_) => {
                lines_to_print += 1;
            }
            _ => {
                continue;
            }
        }
        stdout.flush().unwrap();
        break;
    }
    stdout.suspend_raw_mode().unwrap();

    return lines_to_print;
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
    window_width: i32,
    mut lines_to_print: i32,
    index: Option<i32>,
) -> i32 {
    if !atty::is(Stream::Stdout) || (window_width == 0) {
        if !no_first_indent {
            for _ in 0..indent {
                print!(" ");
            }
        }
        if let Some(n) = index {
            print!("{}: ", n);
        }
        println!("{}", s);
        return lines_to_print - 1;
    }

    lines_to_print = pager_input(window_height, lines_to_print);
    if lines_to_print == -1 {
        return -1;
    }

    let mut str_offset = 0;
    if !no_first_indent {
        str_offset += indent;
        for _ in 0..indent {
            print!(" ");
        }
    }
    if let Some(n) = index {
        str_offset += n.to_string().len() as i32;
        str_offset += 2;
        print!("{}: ", n);
    }

    let mut str_finish =
        (window_width - str_offset) as usize;

    let graphemes: Vec<&str> = s.graphemes(true).collect();

    let slen = graphemes.len();
    if slen < str_finish {
        println!("{}", s);
        return lines_to_print - 1;
    }
    let mut str_start = 0;
    while str_finish < slen {
        let joined_str = graphemes[str_start..str_finish].join("");
        println!("{}", joined_str);
        str_start = str_finish;
        str_finish += window_width as usize;
        lines_to_print -= 1;
        lines_to_print = pager_input(window_height, lines_to_print);
        if lines_to_print == -1 {
            return -1;
        }
    }
    if str_start <= slen {
        let joined_str = graphemes[str_start..slen].join("");
        println!("{}", joined_str);
        lines_to_print -= 1;
    }

    return lines_to_print;
}

impl VM {
    /// Takes a value that can be stringified as its single argument,
    /// and prints that value to standard output.
    pub fn opcode_print(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("print requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let value_opt: Option<&str>;
        to_str!(value_rr, value_opt);

        match value_opt {
            Some(s) => {
                print!("{}", s);
                1
            }
            _ => {
                self.print_error("print argument must be a string");
                0
            }
        }
    }

    /// Takes a value that can be stringified as its single argument,
    /// and prints that value followed by newline to standard output.
    pub fn core_println(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("println requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let value_opt: Option<&str>;
        to_str!(value_rr, value_opt);

        match value_opt {
            Some(s) => {
                println!("{}", s);
                1
            }
            _ => {
                self.print_error("println argument must be a string");
                0
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
    #[allow(clippy::too_many_arguments)]
    fn print_stack_value(
        &mut self,
        value_rr: &Value,
        chunk: Rc<RefCell<Chunk>>,
        i: usize,
        indent: i32,
        no_first_indent: bool,
        window_height: i32,
        window_width: i32,
        mut lines_to_print: i32,
        index: Option<i32>,
    ) -> i32 {
        let mut is_generator = false;
        let type_string = value_rr.type_string();
        {
            match value_rr {
                Value::Ipv4(_) | Value::Ipv4Range(_) | Value::Ipv6(_) | Value::Ipv6Range(_) => {
                    let s = format!("v[{} {}]", &type_string, value_rr.to_string().unwrap());
                    lines_to_print = psv_helper(
                        &s,
                        indent,
                        no_first_indent,
                        window_height,
                        window_width,
                        lines_to_print,
                        index,
                    );
                }
                Value::DateTimeNT(dt) => {
                    let s = format!("v[{} {}]", &type_string, dt.format("%F %T %Z"));
                    lines_to_print = psv_helper(
                        &s,
                        indent,
                        no_first_indent,
                        window_height,
                        window_width,
                        lines_to_print,
                        index,
                    );
                }
                Value::DateTimeOT(dt) => {
                    let s = format!("v[{} {}]", &type_string, dt.format("%F %T %Z"));
                    lines_to_print = psv_helper(
                        &s,
                        indent,
                        no_first_indent,
                        window_height,
                        window_width,
                        lines_to_print,
                        index,
                    );
                }
                /* The way this works is less than ideal, what with it
                 * being different from standard stringification, but
                 * it may be that having separate representations is
                 * useful for some reason. */
                Value::CoreFunction(_) | Value::NamedFunction(_) => {
                    let s = format!("v[{}]", &type_string);
                    lines_to_print = psv_helper(
                        &s,
                        indent,
                        no_first_indent,
                        window_height,
                        window_width,
                        lines_to_print,
                        index,
                    );
                }
                Value::Null => {
                    lines_to_print = psv_helper(
                        "null",
                        indent,
                        no_first_indent,
                        window_height,
                        window_width,
                        lines_to_print,
                        index,
                    );
                }
                Value::Bool(b) => {
                    let s = if *b { ".t" } else { ".f" };
                    lines_to_print = psv_helper(
                        s,
                        indent,
                        no_first_indent,
                        window_height,
                        window_width,
                        lines_to_print,
                        index,
                    );
                }
                Value::Byte(b) => {
                    let s = format!("{:#04x}", b);
                    lines_to_print = psv_helper(
                        &s,
                        indent,
                        no_first_indent,
                        window_height,
                        window_width,
                        lines_to_print,
                        index,
                    );
                }
                Value::Int(n) => {
                    let s = format!("{}", n);
                    lines_to_print = psv_helper(
                        &s,
                        indent,
                        no_first_indent,
                        window_height,
                        window_width,
                        lines_to_print,
                        index,
                    );
                }
                Value::BigInt(n) => {
                    let s = format!("{}", n);
                    lines_to_print = psv_helper(
                        &s,
                        indent,
                        no_first_indent,
                        window_height,
                        window_width,
                        lines_to_print,
                        index,
                    );
                }
                Value::Float(f) => {
                    let s = format!("{}", f);
                    lines_to_print = psv_helper(
                        &s,
                        indent,
                        no_first_indent,
                        window_height,
                        window_width,
                        lines_to_print,
                        index,
                    );
                }
                Value::String(st) => {
                    let mut ss = st.borrow().escaped_string.clone();
                    if st.borrow().string.contains(char::is_whitespace) {
                        ss = format!("\"{}\"", ss);
                    } else if ss.is_empty() {
                        ss = "\"\"".to_string();
                    } else if ss == ".t" {
                        ss = "\".t\"".to_string();
                    } else if ss == ".f" {
                        ss = "\".f\"".to_string();
                    }
                    lines_to_print = psv_helper(
                        &ss,
                        indent,
                        no_first_indent,
                        window_height,
                        window_width,
                        lines_to_print,
                        index,
                    );
                }
                Value::Command(s, _) => {
                    let s = format!("v[{} {}]", &type_string, s);
                    lines_to_print = psv_helper(
                        &s,
                        indent,
                        no_first_indent,
                        window_height,
                        window_width,
                        lines_to_print,
                        index,
                    );
                }
                Value::CommandUncaptured(s) => {
                    let s = format!("v[{} {}]", &type_string, s);
                    lines_to_print = psv_helper(
                        &s,
                        indent,
                        no_first_indent,
                        window_height,
                        window_width,
                        lines_to_print,
                        index,
                    );
                }
                Value::FileWriter(_) | Value::FileReader(_) | Value::DirectoryHandle(_) => {
                    let s = format!("v[{}]", &type_string);
                    lines_to_print = psv_helper(
                        &s,
                        indent,
                        no_first_indent,
                        window_height,
                        window_width,
                        lines_to_print,
                        index,
                    );
                }
                Value::AnonymousFunction(_, _) => {
                    let s = format!("v[{}]", &(value_rr.type_string()));
                    lines_to_print = psv_helper(
                        &s,
                        indent,
                        no_first_indent,
                        window_height,
                        window_width,
                        lines_to_print,
                        index,
                    );
                }
                Value::List(list) => {
                    if list.borrow().len() == 0 {
                        lines_to_print = psv_helper(
                            "()",
                            indent,
                            no_first_indent,
                            window_height,
                            window_width,
                            lines_to_print,
                            index,
                        );
                    } else {
                        lines_to_print = psv_helper(
                            "(",
                            indent,
                            no_first_indent,
                            window_height,
                            window_width,
                            lines_to_print,
                            index,
                        );
                        if lines_to_print == -1 {
                            return lines_to_print;
                        }
                        let new_indent = indent + 4;
                        for (index, element) in list.borrow().iter().enumerate() {
                            lines_to_print = self.print_stack_value(
                                element,
                                chunk.clone(),
                                i,
                                new_indent,
                                false,
                                window_height,
                                window_width,
                                lines_to_print,
                                Some(index.try_into().unwrap()),
                            );
                            if lines_to_print == -1 {
                                return lines_to_print;
                            }
                        }
                        lines_to_print =
                            psv_helper(")", indent, false, window_height,
                                       window_width, lines_to_print, None);
                    }
                }
                Value::Hash(map) => {
                    if map.borrow().len() == 0 {
                        lines_to_print = psv_helper(
                            "h()",
                            indent,
                            no_first_indent,
                            window_height,
                            window_width,
                            lines_to_print,
                            index,
                        );
                    } else {
                        lines_to_print = psv_helper(
                            "h(",
                            indent,
                            no_first_indent,
                            window_height,
                            window_width,
                            lines_to_print,
                            index,
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
                                window_width,
                                lines_to_print,
                                None,
                            );
                            if lines_to_print == -1 {
                                return lines_to_print;
                            }
                        }
                        lines_to_print =
                            psv_helper(")", indent, false, window_height,
                                       window_width, lines_to_print, None);
                    }
                }
                Value::Set(map) => {
                    if map.borrow().len() == 0 {
                        lines_to_print = psv_helper(
                            "s()",
                            indent,
                            no_first_indent,
                            window_height,
                            window_width,
                            lines_to_print,
                            index,
                        );
                    } else {
                        lines_to_print = psv_helper(
                            "s(",
                            indent,
                            no_first_indent,
                            window_height,
                            window_width,
                            lines_to_print,
                            index,
                        );
                        if lines_to_print == -1 {
                            return lines_to_print;
                        }

                        let new_indent = indent + 4;
                        for (_, v) in map.borrow().iter() {
                            lines_to_print = self.print_stack_value(
                                v,
                                chunk.clone(),
                                i,
                                new_indent,
                                false,
                                window_height,
                                window_width,
                                lines_to_print,
                                index,
                            );
                            if lines_to_print == -1 {
                                return lines_to_print;
                            }
                        }
                        lines_to_print =
                            psv_helper(")", indent, false, window_height,
                                       window_width, lines_to_print, None);
                    }
                }
                Value::Generator(_)
                | Value::CommandGenerator(_)
                | Value::KeysGenerator(_)
                | Value::ValuesGenerator(_)
                | Value::EachGenerator(_)
                | Value::MultiGenerator(_)
                | Value::HistoryGenerator(_)
                | Value::IpSet(_) => {
                    is_generator = true;
                }
            }
        }
        if is_generator {
            let mut has_elements = false;
            self.stack.push(value_rr.clone());
            let mut element_index = 0;
            loop {
                let dup_res = self.opcode_dup();
                if dup_res == 0 {
                    return -1;
                }
                let shift_res = self.opcode_shift();
                if shift_res == 0 {
                    self.stack.pop();
                    return -1;
                }
                if self.stack.is_empty() {
                    break;
                }
                let is_null;
                let element_rr = self.stack.pop().unwrap();
                {
                    match element_rr {
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
                        let new_str = format!("v[{} (", &type_string);
                        lines_to_print = psv_helper(
                            &new_str,
                            indent,
                            no_first_indent,
                            window_height,
                            window_width,
                            lines_to_print,
                            index,
                        );
                        if lines_to_print == -1 {
                            return lines_to_print;
                        }
                        has_elements = true;
                    }
                    lines_to_print = self.print_stack_value(
                        &element_rr,
                        chunk.clone(),
                        i,
                        indent + 4,
                        false,
                        window_height,
                        window_width,
                        lines_to_print,
                        Some(element_index),
                    );
                    element_index += 1;
                    if lines_to_print == -1 {
                        return lines_to_print;
                    }
                } else {
                    break;
                }
            }
            self.stack.pop();
            if !has_elements {
                let new_str = format!("v[{}]", &type_string);
                lines_to_print = psv_helper(
                    &new_str,
                    indent,
                    no_first_indent,
                    window_height,
                    window_width,
                    lines_to_print,
                    index,
                );
            } else {
                lines_to_print =
                    psv_helper(")]", indent, false, window_height,
                               window_width, lines_to_print, None);
            }
        }
        if lines_to_print == -1 {
            return lines_to_print;
        }

        lines_to_print
    }

    /// Takes the current chunk, the instruction index, the map of
    /// global functions, and a boolean indicating whether the stack
    /// needs to be cleared after the stack is printed.  Prints the
    /// stack to standard output.
    pub fn print_stack(&mut self, chunk: Rc<RefCell<Chunk>>, i: usize, no_remove: bool) -> bool {
        if self.printing_stack {
            self.print_error("cannot call .s recursively");
            return false;
        }
        self.printing_stack = true;

        let mut window_width:  i32 = 0;
        let mut window_height: i32 = 0;
        let dim_opt = term_size::dimensions();
        if let Some((w, h)) = dim_opt {
            window_width  = w.try_into().unwrap();
            window_height = h.try_into().unwrap();
        }
        let mut lines_to_print = window_height - 1;

        let mut stack_backup = Vec::new();
        while !self.stack.is_empty() {
            let value_rr = self.stack.remove(0);
            lines_to_print = self.print_stack_value(
                &value_rr,
                chunk.clone(),
                i,
                0,
                false,
                window_height,
                window_width,
                lines_to_print,
                None,
            );
            if lines_to_print == -1 {
                if !no_remove {
                    self.stack.clear();
                }
                self.printing_stack = false;
                return true;
            }
            stack_backup.push(value_rr);
        }
        if no_remove {
            self.stack = stack_backup;
        }
        self.printing_stack = false;
        true
    }
}

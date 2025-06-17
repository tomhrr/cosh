use std::borrow::Cow::{self, Borrowed};
use std::cell::RefCell;
use std::collections::HashMap;
use std::env::current_dir;
use std::fs;
use std::path::{self, Path};
use std::rc::Rc;

use ansi_term::Colour::{Blue, Purple, Red};
use dirs::home_dir;
use memchr::memchr;
use rustyline::completion::{escape, unescape, Candidate, Completer, Pair, Quote};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Result};
use rustyline_derive::Helper;
use searchpath::search_path;

use crate::chunk::{Chunk, Variable};
use crate::vm::{LIB_FORMS, SIMPLE_FORMS};

// Most of the code through to 'impl Completer for ShellCompleter' is
// taken from kkawakam/rustyline#574 as at 3a41ee9, with some small
// changes.  Licence text from that repository:
//
// The MIT License (MIT)
//
// Copyright (c) 2015 Katsu Kawakami & Rustyline authors
//
// Permission is hereby granted, free of charge, to any person
// obtaining a copy of this software and associated documentation
// files (the "Software"), to deal in the Software without
// restriction, including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense, and/or sell copies
// of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS
// BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
// ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

const ESCAPE_CHAR: Option<char> = Some('\\');
const DOUBLE_QUOTES_ESCAPE_CHAR: Option<char> = Some('\\');
const DEFAULT_BREAK_CHARS: [u8; 18] = [
    b' ', b'\t', b'\n', b'"', b'\\', b'\'', b'`', b'@', b'$', b'>', b'<', b'=', b';', b'|', b'&',
    b'{', b'(', b'\0',
];
const DOUBLE_QUOTES_SPECIAL_CHARS: [u8; 4] = [b'"', b'$', b'\\', b'`'];

#[derive(PartialEq)]
enum ScanMode {
    DoubleQuote,
    Escape,
    EscapeInDoubleQuote,
    Normal,
    SingleQuote,
}

fn normalize(s: &str) -> Cow<str> {
    Cow::Borrowed(s)
}

/// try to find an unclosed single/double quote in `s`.
/// Return `None` if no unclosed quote is found.
/// Return the unclosed quote position and if it is a double quote.
fn find_unclosed_quote(s: &str) -> Option<(usize, Quote)> {
    let char_indices = s.char_indices();
    let mut mode = ScanMode::Normal;
    let mut quote_index = 0;
    for (index, char) in char_indices {
        match mode {
            ScanMode::DoubleQuote => {
                if char == '"' {
                    mode = ScanMode::Normal;
                } else if char == '\\' {
                    mode = ScanMode::EscapeInDoubleQuote;
                }
            }
            ScanMode::Escape => {
                mode = ScanMode::Normal;
            }
            ScanMode::EscapeInDoubleQuote => {
                mode = ScanMode::DoubleQuote;
            }
            ScanMode::Normal => {
                if char == '"' {
                    mode = ScanMode::DoubleQuote;
                    quote_index = index;
                } else if char == '\\' {
                    mode = ScanMode::Escape;
                } else if char == '\'' {
                    mode = ScanMode::SingleQuote;
                    quote_index = index;
                }
            }
            ScanMode::SingleQuote => {
                if char == '\'' {
                    mode = ScanMode::Normal;
                } // no escape in single quotes
            }
        };
    }
    if ScanMode::DoubleQuote == mode || ScanMode::EscapeInDoubleQuote == mode {
        return Some((quote_index, Quote::Double));
    } else if ScanMode::SingleQuote == mode {
        return Some((quote_index, Quote::Single));
    }
    None
}

/// Given a `line` and a cursor `pos`ition,
/// try to find backward the start of a word.
/// Return (0, `line[..pos]`) if no break char has been found.
/// Return the word and its start position (idx, `line[idx..pos]`) otherwise.
pub fn extract_word<'l>(
    line: &'l str,
    pos: usize,
    esc_char: Option<char>,
    break_chars: &[u8],
) -> (usize, &'l str) {
    let line = &line[..pos];
    if line.is_empty() {
        return (0, line);
    }
    let mut start = None;
    for (i, c) in line.char_indices().rev() {
        if let (Some(esc_char), true) = (esc_char, start.is_some()) {
            if esc_char == c {
                // escaped break char
                start = None;
                continue;
            } else {
                break;
            }
        }
        if c.is_ascii() && memchr(c as u8, break_chars).is_some() {
            start = Some(i + c.len_utf8());
            if esc_char.is_none() {
                break;
            } // else maybe escaped...
        }
    }

    match start {
        Some(start) => (start, &line[start..]),
        None => (0, line),
    }
}

fn filename_complete(
    path: &str,
    esc_char: Option<char>,
    break_chars: &[u8],
    quote: Quote,
) -> Vec<Pair> {
    let sep = path::MAIN_SEPARATOR;
    let (dir_name, file_name) = match path.rfind(sep) {
        Some(idx) => path.split_at(idx + sep.len_utf8()),
        None => ("", path),
    };

    let dir_path = Path::new(dir_name);
    let dir = if dir_path.starts_with("~") {
        if let Some(home) = home_dir() {
            match dir_path.strip_prefix("~") {
                Ok(rel_path) => home.join(rel_path),
                _ => home,
            }
        } else {
            dir_path.to_path_buf()
        }
    } else if dir_path.is_relative() {
        if let Ok(cwd) = current_dir() {
            cwd.join(dir_path)
        } else {
            dir_path.to_path_buf()
        }
    } else {
        dir_path.to_path_buf()
    };

    let mut entries: Vec<Pair> = Vec::new();

    // if dir doesn't exist, then don't offer any completions
    if !dir.exists() {
        return entries;
    }

    // if any of the below IO operations have errors, just ignore them
    if let Ok(read_dir) = dir.read_dir() {
        let file_name = normalize(file_name);
        for entry in read_dir.flatten() {
            if let Some(s) = entry.file_name().to_str() {
                let ns = normalize(s);
                if ns.starts_with(file_name.as_ref()) {
                    if let Ok(metadata) = fs::metadata(entry.path()) {
                        let mut path = String::from(dir_name) + s;
                        if metadata.is_dir() {
                            path.push(sep);
                        }
                        entries.push(Pair {
                            display: String::from(s),
                            replacement: escape(path, esc_char, break_chars, quote),
                        });
                    } // else ignore PermissionDenied
                }
            }
        }
    }
    entries
}

fn internal_complete(
    path: &str,
    esc_char: Option<char>,
    break_chars: &[u8],
    quote: Quote,
    global_functions: Rc<RefCell<HashMap<String, Rc<RefCell<Chunk>>>>>,
    global_vars: Rc<RefCell<HashMap<String, Variable>>>,
) -> Vec<Pair> {
    let mut entries: Vec<Pair> = Vec::new();

    for k in SIMPLE_FORMS.keys() {
        if k.starts_with(path) {
            entries.push(Pair {
                display: Red.paint(*k).to_string(),
                replacement: escape((*k).to_string(), esc_char, break_chars, quote),
            });
        }
    }

    for k in LIB_FORMS.iter() {
        if k.starts_with(path) {
            entries.push(Pair {
                display: Red.paint(*k).to_string(),
                replacement: escape((*k).to_string(), esc_char, break_chars, quote),
            });
        }
    }

    for k in global_functions.borrow().keys() {
        if k.starts_with(path) && !LIB_FORMS.contains::<str>(k) {
            entries.push(Pair {
                display: Blue.paint(k).to_string(),
                replacement: escape(k.to_string(), esc_char, break_chars, quote),
            });
        }
    }

    for k in global_vars.borrow().keys() {
        if k.starts_with(path) {
            entries.push(Pair {
                display: Purple.paint(k).to_string(),
                replacement: escape(k.to_string(), esc_char, break_chars, quote),
            });
        }
    }

    entries
}

fn bin_complete(path: &str, esc_char: Option<char>, break_chars: &[u8], quote: Quote) -> Vec<Pair> {
    let mut entries: Vec<Pair> = Vec::new();
    for file in search_path(path, std::env::var_os("PATH").as_deref(), None) {
        entries.push(Pair {
            display: file.clone(),
            replacement: escape(file, esc_char, break_chars, quote),
        });
    }

    entries
}

pub struct ShellCompleter {
    break_chars: &'static [u8],
    double_quotes_special_chars: &'static [u8],
    global_functions: Rc<RefCell<HashMap<String, Rc<RefCell<Chunk>>>>>,
    global_vars: Rc<RefCell<HashMap<String, Variable>>>,
}

fn should_complete_executable(path: &str, line: &str, start: usize) -> bool {
    // If the string prior to path comprises whitespace, then
    // executable completion should be used (unless the path is
    // qualified).
    let before = &line[0..start];
    if !before.is_empty()
        && before.chars().all(char::is_whitespace)
        && !path.contains(char::is_whitespace)
    {
        return !(path.starts_with("./") || path.starts_with('/'));
    }

    // If the string prior to path includes a $ or { or ( character,
    // followed by (optional) whitespace, and then the path, then
    // executable completion should be used (unless the path is
    // qualified).
    let mut index_opt = before.rfind('$');
    if index_opt.is_none() {
        index_opt = before.rfind('{');
    }
    if index_opt.is_none() {
        index_opt = before.rfind('(');
    }
    if index_opt.is_none() {
        return false;
    }
    let index = index_opt.unwrap();
    let before2_chars = &mut before[index + 1..start].chars();
    let mut hit_char = false;
    loop {
        let c_opt = before2_chars.next();
        if c_opt.is_none() {
            break;
        }
        let c = c_opt.unwrap();
        if c.is_whitespace() {
            if hit_char {
                return false;
            }
        } else {
            hit_char = true;
        }
    }
    !(path.starts_with("./") || path.starts_with('/'))
}

impl ShellCompleter {
    /// Constructor
    pub fn new(
        global_functions: Rc<RefCell<HashMap<String, Rc<RefCell<Chunk>>>>>,
        global_vars: Rc<RefCell<HashMap<String, Variable>>>,
    ) -> Self {
        Self {
            break_chars: &DEFAULT_BREAK_CHARS,
            double_quotes_special_chars: &DOUBLE_QUOTES_SPECIAL_CHARS,
            global_functions,
            global_vars,
        }
    }

    /// Takes the currently edited `line` with the cursor `pos`ition and
    /// returns the start position and the completion candidates for the
    /// partial path to be completed.
    pub fn complete_path(&self, line: &str, pos: usize) -> Result<(usize, Vec<Pair>)> {
        let (start, path, _, esc_char, break_chars, quote) =
            if let Some((idx, quote)) = find_unclosed_quote(&line[..pos]) {
                let start = idx + 1;
                if quote == Quote::Double {
                    (
                        start,
                        unescape(&line[start..pos], DOUBLE_QUOTES_ESCAPE_CHAR),
                        Borrowed(&line[..pos]),
                        DOUBLE_QUOTES_ESCAPE_CHAR,
                        &self.double_quotes_special_chars,
                        quote,
                    )
                } else {
                    (
                        start,
                        Borrowed(&line[start..pos]),
                        Borrowed(&line[..pos]),
                        None,
                        &self.break_chars,
                        quote,
                    )
                }
            } else {
                let (start, path) = extract_word(line, pos, ESCAPE_CHAR, self.break_chars);
                (
                    start,
                    unescape(path, ESCAPE_CHAR),
                    Borrowed(path),
                    ESCAPE_CHAR,
                    &self.break_chars,
                    Quote::None,
                )
            };

        let mut matches = if should_complete_executable(&path, line, start) {
            bin_complete(&path, esc_char, break_chars, quote)
        } else {
            filename_complete(&path, esc_char, break_chars, quote)
        };

        let mut internal_matches = internal_complete(
            &path,
            esc_char,
            break_chars,
            quote,
            self.global_functions.clone(),
            self.global_vars.clone(),
        );

        #[allow(clippy::unnecessary_sort_by)]
        matches.append(&mut internal_matches);
        matches.sort_by(|a, b| a.display().cmp(b.display()));
        Ok((start, matches))
    }
}

impl Default for ShellCompleter {
    fn default() -> Self {
        Self::new(
            Rc::new(RefCell::new(HashMap::new())),
            Rc::new(RefCell::new(HashMap::new())),
        )
    }
}

impl Completer for ShellCompleter {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Result<(usize, Vec<Pair>)> {
        self.complete_path(line, pos)
    }
}

#[derive(Helper)]
pub struct RLHelper {
    pub completer: ShellCompleter,
}

impl Completer for RLHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Result<(usize, Vec<Pair>)> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for RLHelper {}

impl Highlighter for RLHelper {}

impl Validator for RLHelper {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_should_complete_executable_with_parentheses() {
        // Test list start
        assert!(should_complete_executable("ls", "(", 1));
        
        // Test hash start  
        assert!(should_complete_executable("ls", "h(", 2));
        
        // Test set start
        assert!(should_complete_executable("ls", "s(", 2));
        
        // Test with whitespace before parentheses
        assert!(should_complete_executable("ls", " (", 2));
        assert!(should_complete_executable("ls", " h(", 3));
        
        // Test that other contexts still work
        assert!(should_complete_executable("ls", " ", 1));
        assert!(should_complete_executable("ls", "${", 2));
        assert!(should_complete_executable("ls", "{", 1));
    }
    
    #[test]
    fn test_should_complete_executable_qualified_paths() {
        // Qualified paths should return false even with parentheses
        assert!(!should_complete_executable("./ls", "(", 1));
        assert!(!should_complete_executable("/bin/ls", "h(", 2));
    }
    
    #[test]
    fn test_complete_path_with_lists_and_hashes() {
        let global_functions = Rc::new(RefCell::new(HashMap::new()));
        let global_vars = Rc::new(RefCell::new(HashMap::new()));
        let completer = ShellCompleter::new(global_functions, global_vars);
        
        // Test completion at the start of a list
        let result = completer.complete_path("(", 1);
        assert!(result.is_ok());
        let (start, matches) = result.unwrap();
        assert_eq!(start, 1);
        assert!(!matches.is_empty()); // Should have some completions
        
        // Test completion at the start of a hash
        let result = completer.complete_path("h(", 2);
        assert!(result.is_ok());
        let (start, matches) = result.unwrap();
        assert_eq!(start, 2);
        assert!(!matches.is_empty()); // Should have some completions
        
        // Test completion at the start of a set
        let result = completer.complete_path("s(", 2);
        assert!(result.is_ok());
        let (start, matches) = result.unwrap();
        assert_eq!(start, 2);
        assert!(!matches.is_empty()); // Should have some completions
    }
    
    #[test]
    fn test_edge_cases_with_parentheses() {
        // Test nested parentheses - inner completion should work
        assert!(should_complete_executable("ls", "((", 2));
        assert!(should_complete_executable("ls", "h(s(", 4));
        
        // Test with mixed whitespace
        assert!(should_complete_executable("ls", " \t(", 3));
        assert!(should_complete_executable("ls", "\n h(", 4));
        
        // Test that empty path after parentheses still works
        assert!(should_complete_executable("", "(", 1));
        assert!(should_complete_executable("", "h(", 2));
    }
}

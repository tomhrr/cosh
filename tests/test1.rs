extern crate cosh;
extern crate assert_cmd;
extern crate tempfile;

use std::io::{Write};
use std::fs;
use assert_cmd::Command;
use tempfile::NamedTempFile;

#[test]
fn add_file() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "1 2 +").unwrap();

    let mut cmd = Command::cargo_bin("cosh").unwrap();
    let path = file.path();
    let assert = cmd.arg(path).assert();
    assert.success().stdout("3\n");
}

fn basic_test(input: &str, output: &str) {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "{}", input).unwrap();

    let mut cmd = Command::cargo_bin("cosh").unwrap();
    let path = file.path();
    let assert = cmd.arg(path).assert();
    let output2 = format!("{}\n", output);
    assert.success().stdout(output2.to_owned());
}

fn basic_error_test(input: &str, output: &str) {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "{}", input).unwrap();

    let mut cmd = Command::cargo_bin("cosh").unwrap();
    let path = file.path();
    let assert = cmd.arg(path).assert();
    assert.success().stderr(output.to_owned() + "\n");
}

#[test]
fn add() {
    basic_test("1 2 +;", "3");
}

#[test]
fn subtract() {
    basic_test("1 2 -;", "-1");
}

#[test]
fn divide() {
    basic_test("10 5 /;", "2");
}

#[test]
fn multiply() {
    basic_test("10 5 *;", "50");
}

#[test]
fn combination() {
    basic_test("10 5 *; 10 +; 20 -; 10 /", "4");
}

#[test]
fn fn_call() {
    basic_test(": asdf 1 2 + ; :: asdf ;", "3");
}

#[test]
fn vars_basic() {
    basic_test("x var; 10 x !; x @; 5 +;", "15");
}

#[test]
fn vars_scoped() {
    basic_test(concat!("x var; 10 x !; ",
                               ": asdf x var; 15 x !; ",
                                 ": qwer x @; x var; 20 x !; x @; :: ",
                                 "qwer; x @; :: ",
                             "asdf; x @;"), "15\n20\n15\n10");
}

#[test]
fn if_then() {
    basic_test("1 if; 2 then; 0 if; 3 then;", "2");
}

#[test]
fn if_then_multi() {
    basic_test("1 if; 2 3 4 then; 0 if; 3 then;", "2\n3\n4");
}

#[test]
fn if_then_else_multi() {
    basic_test("1 if; 1 2 3 else; 4 5 6 then; 0 if; 1 2 3 else; 4 5 6 then;", "1\n2\n3\n4\n5\n6");
}

#[test]
fn if_then_else_nested() {
    basic_test("1 if; 0 if; 3 else; 2 then; else; 3 then;", "2");
    basic_test("1 if; 2 0 if; 3 then; else; 4 then;", "2");
}

#[test]
fn begin_until() {
    basic_test("x var; 5 x !; begin; x @; println; x @; 1 -; x !; x @; 0 =; until", "5\n4\n3\n2\n1");
    basic_test("x var; 5 x !; begin; x @; println; leave; x @; 1 -; x !; x @; 0 =; until", "5");
}

#[test]
fn begin_until_nested() {
    basic_test("
x var; 3 x !;
begin;
    y var; 2 y !;
    begin;
        y @; println;
        y @; 1 -; y !;
        y @; 0 =;
        until;
    x @; println;
    x @; 1 -; x !;
    x @; 0 =;
    until;", "2\n1\n3\n2\n1\n2\n2\n1\n1");
}

#[test]
fn top_level_functions() {
    basic_test(": asdf 1 2 3 :: : qwer asdf; 1 2 3 :: qwer;", "1\n2\n3\n1\n2\n3");
}

#[test]
fn fn_name_error() {
    basic_error_test(": 1234 ::", "1:3: expected name token");
}

#[test]
fn then_error() {
    basic_error_test("then;", "1:1: 'then' without 'if'");
}

#[test]
fn else_error() {
    basic_error_test(" else;", "1:2: 'else' without 'if'");
}

#[test]
fn leave_error() {
    basic_error_test("  leave;", "1:3: 'leave' without 'begin'");
}

#[test]
fn until_error() {
    basic_error_test("  until;", "1:3: 'until' without 'begin'");
}

#[test]
fn add_error() {
    basic_error_test("1 t +;", "1:5: + requires two numbers");
}

#[test]
fn subtract_error() {
    basic_error_test("1 t -;", "1:5: - requires two numbers");
}

#[test]
fn multiply_error() {
    basic_error_test("1 t *;", "1:5: * requires two numbers");
}

#[test]
fn divide_error() {
    basic_error_test("1 t /;", "1:5: / requires two numbers");
}

#[test]
fn equals_error() {
    basic_test("1 t =;", "0");
}

#[test]
fn function_not_found_error() {
    basic_error_test("asdf;", "1:1: function not found");
}

#[test]
fn var_must_be_string_error() {
    basic_error_test("1 var;", "1:3: variable name must be a string");
}

#[test]
fn could_not_find_var_error() {
    basic_error_test("x @;", "1:3: could not find variable");
}

#[test]
fn var_must_be_string_2_error() {
    basic_error_test("1 @;", "1:3: variable name must be a string");
}

#[test]
fn var_must_be_string_in_fn_error() {
    basic_error_test(": m 100 asdf dup; var; !; ::", "1:19: variable name must precede var");
}

#[test]
fn set_must_be_string_in_fn_error() {
    basic_error_test(": m asdf var; asdf dup; !; ::", "1:25: variable name must precede !");
}

#[test]
fn get_must_be_string_in_fn_error() {
    basic_error_test(": m asdf var; 100 asdf !; asdf dup; @; ::", "1:37: variable name must precede @");
}

#[test]
fn map_test_with_result() {
    basic_test("(1 2 3) [2 +] map", "(\n    3\n    4\n    5\n)");
}

#[test]
fn grep_test() {
    basic_test("(1 2 3) [2 =] grep", "(\n    2\n)");
}

#[test]
fn foldl_test() {
    basic_test("(1 2 3) 0 [+] foldl", "6");
}

#[test]
fn for_test() {
    basic_test("(1 2 3) [println] for", "1\n2\n3");
}

#[test]
fn generator_basic_test() {
    basic_test(":~ gen 0 0 drop; 1 yield; 2 yield; 3 yield; :: gen; dup; shift; println; dup; shift; println; shift; println;", "1\n2\n3");
}

#[test]
fn generator_var_test() {
    basic_test("
:~ gen 0 0
    drop;
    n var;
    0 n !;
    begin;
        n @; yield;
        n @; 1 +; n !;
        n @; 3 >;
        until; ::
n var; 100 n !;
gen; dup; shift; println; dup; shift; println; shift; println;
n @; println;", "0\n1\n2\n100");
}

#[test]
fn clear_test() {
    basic_test("1 2 clear; 3 4", "3\n4");
}

#[test]
fn dup_test() {
    basic_test("1 dup; dup;", "1\n1\n1");
}

#[test]
fn swap_test() {
    basic_test("1 2 swap;", "2\n1");
}

#[test]
fn rot_test() {
    basic_test("1 2 3 rot;", "2\n3\n1");
}

#[test]
fn over_test() {
    basic_test("1 2 3 over;", "1\n2\n3\n2");
}

#[test]
fn depth_test() {
    basic_test("1 depth; 2 depth; 3 depth;", "1\n1\n2\n3\n3\n5");
}

#[test]
fn le_test() {
    basic_test("1 5 <=;", "1");
    basic_test("1 1 <=;", "1");
    basic_test("1 0 <=;", "0");
}

#[test]
fn ge_test() {
    basic_test("1 5 >=;", "0");
    basic_test("1 1 >=;", "1");
    basic_test("1 0 >=;", "1");
}

#[test]
fn is_null_test() {
    basic_test(":~ nullgen 0 0 drop; :: nullgen; dup; shift; is-null; nip;", "1");
}

#[test]
fn is_list_test() {
    basic_test("(1 2 3) is-list;", "1");
}

#[test]
fn read_file_test() {
    basic_test("
: rl dup; readline; print; ::
test-data/readfile r open; rl; rl; rl; rl; rl; drop;
", "1\n2\n3\n4\n5");
}

#[test]
fn write_file_test() {
    basic_test("
test w open;
    dup; \"asdf\\n\" writeline;
    dup; \"qwer\\n\" writeline;
    close;
test r open;
    dup; readline; print;
    dup; readline; print;
    close;
", "asdf\nqwer");
    fs::remove_file("test").unwrap();
}

#[test]
fn lsr_test() {
    basic_test(". lsr; begin; dup; shift; is-null; if; leave; then; 0 until;", "()");
}

#[test]
fn implicit_generator_test() {
    basic_test("lsr; begin; 1 take; drop; ()", "()");
}

#[test]
fn regex_borrow_problem() {
    basic_test("((\"asdf\") (\"asdf\")) [[asdf m] grep] map", "(\n    (\n        asdf\n    )\n    (\n        asdf\n    )\n)");
}

#[test]
fn import_test() {
    basic_test("(1 2 3) sum;", "6");
}

#[test]
fn push_test() {
    basic_test("(1 2 3) 5 push;", "(\n    1\n    2\n    3\n    5\n)");
}

#[test]
fn pop_test() {
    basic_test("(1 2 3) pop;", "3");
}

#[test]
fn unshift_test() {
    basic_test("(1 2 3) 5 unshift;", "(\n    5\n    1\n    2\n    3\n)");
}

#[test]
fn shift_test() {
    basic_test("(1 2 3) shift;", "1");
}

#[test]
fn file_copy_test() {
    basic_test("README.md f<; output swap; f>; ()", "()");
    let md1 = fs::metadata("README.md").unwrap();
    let md2 = fs::metadata("output").unwrap();
    assert_eq!(md1.len(), md2.len());
    fs::remove_file("output").unwrap();
}

#[test]
fn single_quote_test() {
    basic_test("'asdf asdf' println;", "asdf asdf");
}

#[test]
fn regex_tests() {
    basic_test("'asdf asdf' asdf m;", "1");
    basic_test("'asdf asdf' asdf qwer s;", "\"qwer qwer\"");
    basic_test("'12341234' \\d\\d\\d\\d c;", "(\n    1234\n    1234\n)");
}

#[test]
fn nth_test() {
    basic_test("(1 2 3) 1 nth", "2");
    basic_test("(1 2 3) 1 100 nth!", "(\n    1\n    100\n    3\n)");
}

#[test]
fn take_test() {
    basic_test("(1 2 3) 2 take", "(\n    1\n    2\n)");
    basic_test("(1 2 3) take-all", "(\n    1\n    2\n    3\n)");
    basic_test("README.md f<; 1 take", "(\n    \"## cosh\\n\"\n)");
}

#[test]
fn grep_test_generator() {
    basic_test("10 range; [5 <] grep; take-all", "(\n    0\n    1\n    2\n    3\n    4\n)");
    basic_test("10 range; take-all; [5 <] grep", "(\n    0\n    1\n    2\n    3\n    4\n)");
}

#[test]
fn map_test_generator() {
    basic_test("5 range; [2 *] map; take-all", "(\n    0\n    2\n    4\n    6\n    8\n)");
    basic_test("5 range; take-all; [2 *] map", "(\n    0\n    2\n    4\n    6\n    8\n)");
}

#[test]
fn split_test() {
    basic_test("test-data/split f<; take-all; 0 nth; , split",
               "(\n    asdf\n    qwer\n    \"asdf asdf\"\n    asdf,asdf\n    \"\"\n    \"\"\n    \"\"\n    \"qwer\\n\"\n)");
}

#[test]
fn join_test() {
    basic_test("(a b c) , join", "a,b,c");
    basic_test("('a,b' c d) , join", "\\\"a,b\\\",c,d");
    basic_test("(a,b c d) , join", "\\\"a,b\\\",c,d");
    basic_test("('a\"b' c d) , join", "\\\"a\\\\\"b\\\",c,d");
}

#[test]
fn append_test() {
    basic_test("a b append", "ab");
}

#[test]
fn coerce_to_int_test() {
    basic_test("test-data/csv f<; [chomp] map; [, split] map; [0 [+] foldl] map; take-all;",
               "(\n    10\n    26\n    42\n)");
}

#[test]
fn coerce_to_string_test() {
    basic_test("(1 2 3 4 5 6) '' [append] foldl;",
               "123456");
}

#[test]
fn commands_test() {
    basic_test("{ls}; {sort} |; take-all; [o.toml m] grep; chomp map;",
               "(\n    Cargo.toml\n)");
    basic_test(". -type f {find {2} -maxdepth 1 {1} {0}}; {sort} |; take-all; [o.toml m] grep; chomp map; nip; nip; nip;",
               "(\n    ./Cargo.toml\n)");
    basic_test("3 2 1 {dc -e \"{2} {0} + {1} + p\"}; shift; chomp; nip; nip; nip;",
               "6");
    basic_test("{ls}; -r {sort {}} |; take-all; [o.toml m] grep; chomp map;",
               "(\n    Cargo.toml\n)");
}

#[test]
fn hash_test() {
    basic_test("h(1 2 3 4) 1 at;", "2");
    basic_test("h(1 2 3 4) 1 5 at!; 1 at;", "5");
    basic_test("h(1 2 3 4) keys; take-all;", "(\n    3\n    1\n)");
    basic_test("h(1 2 3 4) values; take-all;", "(\n    4\n    2\n)");
    basic_test("h(1 2 3 4) each; take-all;", "(\n    (\n        3\n        4\n    )\n    (\n        1\n        2\n    )\n)");
}

#[test]
fn json_test() {
    basic_test("'{\"3\":4,\"1\":2}' from-json; 3 at", "4");
    basic_test("h(1 2 3 4) to-json", "{\\\"3\\\":4,\\\"1\\\":2}");
}

#[test]
fn json_file_test() {
    basic_test("test-data/json1 f<; \"\" join; from-json;", "h(\n    \"asdf\": 1\n)");
    basic_test("test-data/json2 f<; \"\" join; from-json;", "h(\n    \"asdf\": 1\n    \"qwer\": 2\n    \"tyui\": h(\n        \"asdf\": 5\n    )\n    \"zxcv\": (\n        3\n        4\n    )\n)");
}

#[test]
fn xml_test() {
    basic_test("\"<e a='b'>one<a>two</a>three</e>\" from-xml; to-xml;",
        "\"<e a=\\\"b\\\">one<a>two</a>three</e>\"");
}

#[test]
fn external_command_test() {
    basic_test("$ls tests",
        "test1.rs");
}

#[test]
fn bigint_test_add() {
    basic_test("1000000000000000000 1 +;", "1000000000000000001");
}

#[test]
fn float_test_add() {
    basic_test("1.5 2.4 +;", "3.9");
}

#[test]
fn bigint_test_subtract() {
    basic_test("1000000000000000000 1000000000000000001 -;", "-1");
}

#[test]
fn float_test_subtract() {
    basic_test("5.5 2.5 -;", "3");
}

#[test]
fn bigint_test_multiply() {
    basic_test("1000000000000000000 1000000000000000001 *;", "1000000000000000001000000000000000000");
}

#[test]
fn float_test_multiply() {
    basic_test("5.5 2.5 *;", "13.75");
}

#[test]
fn local_var_is_zero() {
    basic_test(": mfn x var; x @; :: mfn;", "0");
}

#[test]
fn global_var_is_zero() {
    basic_test("x var; x @;", "0");
}

#[test]
fn nested_function_vars() {
    basic_test(": ff n var; 10 n !; f var; [n @; 1 +; n !] f !; f @; funcall; f @; funcall; n @; :: ff;", "12");
}

#[test]
fn grep_not_iterated_n_is_the_same() {
    basic_test("n var; 10 n !; README.md f<; [n @; 1 +; n !; eeeee m] grep; n @;", "()\n10");
}

#[test]
fn regex_numbers() {
    basic_test("((asdf asdf)) [[243 m] grep] map", "(\n    ()\n)");
}

#[test]
fn shift_all() {
    basic_test("(1 2 3) shift-all", "1\n2\n3");
}

#[test]
fn negative_numbers() {
    basic_test("-5 4 +; -6.5 3.2 +;", "-1\n-3.3");
}

#[test]
fn any() {
    basic_test("(1 2 3) [3 =] any", "1");
    basic_test("(1 2 3) [4 =] any", "0");
}

#[test]
fn return_test() {
    basic_test(": f ding println; return; ding println; :: f;", "ding");
}

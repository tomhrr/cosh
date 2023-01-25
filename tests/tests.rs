extern crate assert_cmd;
extern crate cosh;
extern crate tempfile;

use assert_cmd::Command;
use std::fs;
use std::io::Write;
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
    assert.success().stdout(output2);
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
    basic_test(": asdf 1 2 + ; ,, asdf ;", "3");
}

#[test]
fn vars_basic() {
    basic_test("x var; 10 x !; x @; 5 +;", "15");
}

#[test]
fn vars_scoped() {
    basic_test(
        concat!(
            "x var; 10 x !; ",
            ": asdf x var; 15 x !; ",
            ": qwer x @; x var; 20 x !; x @; ,, ",
            "qwer; x @; ,, ",
            "asdf; x @;"
        ),
        "15\n20\n15\n10",
    );
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
    basic_test(
        "1 if; 1 2 3 else; 4 5 6 then; 0 if; 1 2 3 else; 4 5 6 then;",
        "1\n2\n3\n4\n5\n6",
    );
}

#[test]
fn if_then_else_nested() {
    basic_test("1 if; 0 if; 3 else; 2 then; else; 3 then;", "2");
    basic_test("1 if; 2 0 if; 3 then; else; 4 then;", "2");
}

#[test]
fn begin_until() {
    basic_test(
        "x var; 5 x !; begin; x @; println; x @; 1 -; x !; x @; 0 =; until",
        "5\n4\n3\n2\n1",
    );
    basic_test(
        "x var; 5 x !; begin; x @; println; leave; x @; 1 -; x !; x @; 0 =; until",
        "5",
    );
}

#[test]
fn begin_until_nested() {
    basic_test(
        "
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
    until;",
        "2\n1\n3\n2\n1\n2\n2\n1\n1",
    );
}

#[test]
fn top_level_functions() {
    basic_test(
        ": asdf 1 2 3 ,, : qwer asdf; 1 2 3 ,, qwer;",
        "1\n2\n3\n1\n2\n3",
    );
}

#[test]
fn fn_name_error() {
    basic_error_test(": 1234 ,,", "1:3: expected name token");
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
    basic_test("1 t =;", ".f");
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
    basic_error_test(
        ": m 100 asdf dup; var; !; ,,",
        "1:19: variable name must precede var",
    );
}

#[test]
fn set_must_be_string_in_fn_error() {
    basic_error_test(
        ": m asdf var; asdf dup; !; ,,",
        "1:25: variable name must precede !",
    );
}

#[test]
fn get_must_be_string_in_fn_error() {
    basic_error_test(
        ": m asdf var; 100 asdf !; asdf dup; @; ,,",
        "1:37: variable name must precede @",
    );
}

#[test]
fn map_test_with_result() {
    basic_test("(1 2 3) [2 +] map", "(\n    0: 3\n    1: 4\n    2: 5\n)");
}

#[test]
fn grep_test() {
    basic_test("(1 2 3) [2 =] grep", "(\n    0: 2\n)");
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
    basic_test(":~ gen 0 0 drop; 1 yield; 2 yield; 3 yield; ,, gen; dup; shift; println; dup; shift; println; shift; println;", "1\n2\n3");
}

#[test]
fn generator_var_test() {
    basic_test(
        "
:~ gen 0 0
    drop;
    n var;
    0 n !;
    begin;
        n @; yield;
        n @; 1 +; n !;
        n @; 3 >;
        until; ,,
n var; 100 n !;
gen; dup; shift; println; dup; shift; println; shift; println;
n @; println;",
        "0\n1\n2\n100",
    );
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
    basic_test("1 5 <=;", ".t");
    basic_test("1 1 <=;", ".t");
    basic_test("1 0 <=;", ".f");
}

#[test]
fn ge_test() {
    basic_test("1 5 >=;", ".f");
    basic_test("1 1 >=;", ".t");
    basic_test("1 0 >=;", ".t");
}

#[test]
fn is_null_test() {
    basic_test(
        ":~ nullgen 0 0 drop; ,, nullgen; dup; shift; is-null; nip;",
        ".t",
    );
}

#[test]
fn is_list_test() {
    basic_test("(1 2 3) is-list;", ".t");
}

#[test]
fn read_file_test() {
    basic_test(
        "
: rl dup; readline; print; ,,
test-data/readfile r open; rl; rl; rl; rl; rl; drop;
",
        "1\n2\n3\n4\n5",
    );
}

#[test]
fn write_file_test() {
    basic_test(
        "
test w open;
    dup; \"asdf\\n\" writeline;
    dup; \"qwer\\n\" writeline;
    close;
test r open;
    dup; readline; print;
    dup; readline; print;
    close;
",
        "asdf\nqwer",
    );
    fs::remove_file("test").unwrap();
}

#[test]
fn lsr_test() {
    basic_test(
        ". lsr; begin; dup; shift; is-null; if; leave; then; 0 until;",
        "v[gen]",
    );
}

#[test]
fn implicit_generator_test() {
    basic_test("lsr; begin; 1 take; drop; ()", "()");
}

#[test]
fn regex_borrow_problem() {
    basic_test(
        "((\"asdf\") (\"asdf\")) [[asdf m] grep] map",
        "(\n    0: (\n        0: asdf\n    )\n    1: (\n        0: asdf\n    )\n)",
    );
}

#[test]
fn import_test() {
    basic_test("(1 2 3) sum;", "6");
}

#[test]
fn push_test() {
    basic_test(
        "(1 2 3) 5 push;",
        "(\n    0: 1\n    1: 2\n    2: 3\n    3: 5\n)",
    );
}

#[test]
fn pop_test() {
    basic_test("(1 2 3) pop;", "3");
}

#[test]
fn unshift_test() {
    basic_test(
        "(1 2 3) 5 unshift;",
        "(\n    0: 5\n    1: 1\n    2: 2\n    3: 3\n)",
    );
}

#[test]
fn shift_test() {
    basic_test("(1 2 3) shift;", "1");
}

#[test]
fn file_copy_test() {
    basic_test("README.md f<; output f>; ()", "()");
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
    basic_test("'asdf asdf' asdf m;", ".t");
    basic_test("'asdf asdf' asdf/g qwer s;", "\"qwer qwer\"");
    basic_test(
        "'12341234' \\d\\d\\d\\d/g c;",
        "(\n    0: 1234\n    1: 1234\n)",
    );
}

#[test]
fn get_test() {
    basic_test("(1 2 3) 1 get", "2");
    basic_test("(1 2 3) 1 100 set", "(\n    0: 1\n    1: 100\n    2: 3\n)");
}

#[test]
fn take_test() {
    basic_test("(1 2 3) 2 take", "(\n    0: 1\n    1: 2\n)");
    basic_test("(1 2 3) take-all", "(\n    0: 1\n    1: 2\n    2: 3\n)");
    basic_test("README.md f<; 1 take", "(\n    0: \"## cosh\\n\"\n)");
}

#[test]
fn grep_test_generator() {
    basic_test(
        "10 range; [5 <] grep; take-all",
        "(\n    0: 0\n    1: 1\n    2: 2\n    3: 3\n    4: 4\n)",
    );
    basic_test(
        "10 range; take-all; [5 <] grep",
        "(\n    0: 0\n    1: 1\n    2: 2\n    3: 3\n    4: 4\n)",
    );
}

#[test]
fn map_test_generator() {
    basic_test(
        "5 range; [2 *] map; take-all",
        "(\n    0: 0\n    1: 2\n    2: 4\n    3: 6\n    4: 8\n)",
    );
    basic_test(
        "5 range; take-all; [2 *] map",
        "(\n    0: 0\n    1: 2\n    2: 4\n    3: 6\n    4: 8\n)",
    );
}

#[test]
fn split_test() {
    basic_test("test-data/split f<; take-all; 0 get; , split",
               "(\n    0: asdf\n    1: qwer\n    2: \"asdf asdf\"\n    3: asdf,asdf\n    4: \"\"\n    5: \"\"\n    6: \"\"\n    7: \"qwer\\n\"\n)");

    basic_test("asdf:asdf:asdf \":\" split; \":\" join", "asdf:asdf:asdf");
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
    basic_test("a b ++", "ab");
    basic_test(
        "3 range; take-all; 3 range; take-all; ++",
        "(\n    0: 0\n    1: 1\n    2: 2\n    3: 0\n    4: 1\n    5: 2\n)",
    );
    basic_test("h(1 2) h(3 4) ++; keys; sort; '-' join", "1-3");
}

#[test]
fn coerce_to_int_test() {
    basic_test(
        "test-data/csv f<; [chomp] map; [, split] map; [0 [+] foldl] map; take-all;",
        "(\n    0: 10\n    1: 26\n    2: 42\n)",
    );
}

#[test]
fn coerce_to_string_test() {
    basic_test("(1 2 3 4 5 6) '' [++] foldl;", "123456");
}

#[test]
fn commands_test() {
    basic_test(
        "{ls}; {sort} |; take-all; [o.toml m] grep; chomp map;",
        "(\n    0: Cargo.toml\n)",
    );
    basic_test(". -type f {find {2} -maxdepth 1 {1} {0}}; {sort} |; take-all; [o.toml m] grep; chomp map; nip; nip; nip;",
               "(\n    0: ./Cargo.toml\n)");
    basic_test(
        "3 2 1 {dc -e \"{2} {0} + {1} + p\"}; shift; chomp; nip; nip; nip;",
        "6",
    );
    basic_test(
        "{ls}; -r {sort {}} |; take-all; [o.toml m] grep; chomp map;",
        "(\n    0: Cargo.toml\n)",
    );
}

#[test]
fn hash_test() {
    basic_test("h(1 2 3 4) 1 get;", "2");
    basic_test("h(1 2 3 4) 1 5 set; 1 get;", "5");
    basic_test("h(1 2 3 4) keys; take-all;", "(\n    0: 3\n    1: 1\n)");
    basic_test("h(1 2 3 4) values; take-all;", "(\n    0: 4\n    1: 2\n)");
    basic_test(
        "h(1 2 3 4) each; take-all;",
        "(\n    0: (\n        0: 3\n        1: 4\n    )\n    1: (\n        0: 1\n        1: 2\n    )\n)",
    );
}

#[test]
fn json_test() {
    basic_test("'{\"3\":4,\"1\":2}' from-json; 3 get", "4");
    basic_test("h(1 2 3 4) to-json", "{\\\"3\\\":4,\\\"1\\\":2}");
    basic_test("test-data/json-bigint f<; \"\" join; from-json;",
        "h(\n    \"num1\": 0\n    \"num2\": 100\n    \"num3\": 123.456\n    \"num4\": -123456789123\n    \"num5\": 123456789123\n)");
    basic_test("test-data/json-bigint f<; from-json;",
        "h(\n    \"num1\": 0\n    \"num2\": 100\n    \"num3\": 123.456\n    \"num4\": -123456789123\n    \"num5\": 123456789123\n)");
}

#[test]
fn json_file_test() {
    basic_test(
        "test-data/json1 f<; \"\" join; from-json;",
        "h(\n    \"asdf\": 1\n)",
    );
    basic_test("test-data/json2 f<; \"\" join; from-json;", "h(\n    \"asdf\": 1\n    \"qwer\": 2\n    \"tyui\": h(\n        \"asdf\": 5\n    )\n    \"zxcv\": (\n        0: 3\n        1: 4\n    )\n)");
}

#[test]
fn xml_test() {
    basic_test(
        "\"<e a='b'>one<a>two</a>three</e>\" from-xml; to-xml;",
        "\"<e a=\\\"b\\\">one<a>two</a>three</e>\"",
    );
}

#[test]
fn external_command_test() {
    basic_test("$ls tests", "tests.rs");
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
    basic_test(
        "1000000000000000000 1000000000000000001 *;",
        "1000000000000000001000000000000000000",
    );
}

#[test]
fn float_test_multiply() {
    basic_test("5.5 2.5 *;", "13.75");
}

#[test]
fn local_var_is_zero() {
    basic_test(": mfn x var; x @; ,, mfn;", "0");
}

#[test]
fn global_var_is_zero() {
    basic_test("x var; x @;", "0");
}

#[test]
fn nested_function_vars() {
    basic_test(
        ": ff n var; 10 n !; f var; [n @; 1 +; n !] f !; f @; funcall; f @; funcall; n @; ,, ff;",
        "12",
    );
}

#[test]
fn grep_not_iterated_n_is_the_same() {
    basic_test(
        "n var; 10 n !; README.md f<; [n @; 1 +; n !; eeeee m] grep; n @;",
        "v[gen]\n10",
    );
}

#[test]
fn regex_numbers() {
    basic_test("((asdf asdf)) [[243 m] grep] map", "(\n    0: ()\n)");
}

#[test]
fn negative_numbers() {
    basic_test("-5 4 +; -6.5 3.2 +;", "-1\n-3.3");
}

#[test]
fn misc_lst_fns() {
    basic_test("(1 2 3) [3 =] any", ".t");
    basic_test("(1 2 3) [4 =] any", ".f");
    basic_test("(1 2 3) [0 >] all", ".t");
    basic_test("(1 2 3) [100 >] all", ".f");
    basic_test("(1 2 3) [0 >] none", ".f");
    basic_test("(1 2 3) [100 >] none", ".t");
    basic_test("(1 2 3) [0 >] notall", ".f");
    basic_test("(1 2 3) [100 >] notall", ".t");
    basic_test("(1 2 3) [2 >] first", "3");
    basic_test("(1 2 3) [100 >] first", "null");
    basic_test("4 range; dup; shift; drop; product", "6");
    basic_test(
        "(1 2 5 1 2 5 3 6) uniq",
        "v[gen (\n    0: 1\n    1: 2\n    2: 5\n    3: 3\n    4: 6\n)]",
    );
    basic_test(
        "(a b 1 b 2) uniq",
        "v[gen (\n    0: a\n    1: b\n    2: 1\n    3: 2\n)]",
    );
}

#[test]
fn return_test() {
    basic_test(": f ding println; return; ding println; ,, f;", "ding");
}

#[test]
fn sort_test() {
    basic_test(
        "(5 2 3 4 1) sort;",
        "(\n    0: 1\n    1: 2\n    2: 3\n    3: 4\n    4: 5\n)",
    );
    basic_test(
        "(5 2 3 4 1) <=> sortp;",
        "(\n    0: 1\n    1: 2\n    2: 3\n    3: 4\n    4: 5\n)",
    );
}

#[test]
fn conv_test() {
    basic_test("5 int; \"10\" int;", "5\n10");
    basic_test("5 str; \"10\" str;", "5\n10");
    basic_test("5 float; \"10\" float;", "5\n10");
}

#[test]
fn search_replace_test() {
    basic_test("asdf \"(as)(df)\" as\\2\\1df s;", "asdfasdf");
}

#[test]
fn eq_test() {
    basic_test("asdf asdf =", ".t");
}

#[test]
fn nth_bounds_test1() {
    basic_test(
        "10 range; take-all; 15 get",
        "null"
    );
}

#[test]
fn nth_bounds_test2() {
    basic_error_test(
        "10 range; take-all; 10 15 set",
        "1:27: second set argument must fall within list bounds",
    );
}

#[test]
fn anon_fn_var_test() {
    basic_test(
        "3 range; [drop; x var; 3 x !; x @] map;",
        "v[gen (\n    0: 3\n    1: 3\n    2: 3\n)]",
    );
}

#[test]
fn generator_closure_test() {
    basic_test(
        "
: f
    x var;
    10 x !;
    : e
        z var;
        20 z !;
        : q x @; z @; +; 5 +; ,,
        :~ gen 0 0 drop;
            y var; 30 y !;
            begin; y @; q; +; y !; y @; yield; 0 until; ,,
        gen;
    ,,
    e; ,,

f;
dup; shift; println;
dup; shift; println;
dup; shift; println;
drop;
    ",
        "65\n100\n135",
    );
}

#[test]
fn anon_fn_test() {
    basic_error_test(
        "
: f
    x var;
    10 x !;
    [x @; 20 +;] ,,

f;
funcall;
    ",
        "5:6: anonymous function environment has gone out of scope",
    );
}

#[test]
fn bool_test() {
    basic_test(".t if; 1 else; 2 then;", "1");
    basic_test(".f if; 1 else; 2 then;", "2");
}

#[test]
fn json_bool_test() {
    basic_test("\"[true, false]\" from-json; to-json", "[true,false]");
}

#[test]
fn comment_test() {
    basic_test(
        "
# A function.
: f 100 ,,
f;
",
        "100",
    );
    basic_test("mystring#allgood", "mystring#allgood");
}

#[test]
fn clone_test() {
    basic_test(
        "3 range; take-all; dup; clone; shift;",
        "(\n    0: 0\n    1: 1\n    2: 2\n)\n0",
    );
    basic_test(
        "3 range; dup; clone; take-all; swap; take-all; ++; '-' join;",
        "0-1-2-0-1-2",
    );
    basic_test("h(1 2) keys; dup; clone; 0 get; swap; 0 get; ++", "11");
    basic_test("h(1 2) values; dup; clone; 0 get; swap; 0 get; ++", "22");
}

#[test]
fn date_test() {
    basic_test("now; now; =", ".f");
    basic_test("lcnow; lcnow; =", ".f");
    basic_test("now; now; <", ".t");
    basic_test("now; now; >", ".f");
    basic_test("now; to-epoch; \\d+ m;", ".t");
    basic_test(
        "now; dup; '%F %T' strftime; swap; to-epoch; from-epoch; '%F %T' strftime; =",
        ".t",
    );
    basic_test(
        "1664280627 from-epoch; '%F %T' strftime",
        "\"2022-09-27 12:10:27\"",
    );
    basic_test("now; dup; '%F %T' strftime; swap; Australia/Brisbane set-tz; UTC set-tz; '%F %T' strftime; =",
               ".t");
    basic_test(
        "'2022-09-27 12:10:27' '%F %T' strptime; to-epoch;",
        "1664280627",
    );
    basic_test(
        "'2022-09-27 22:10:27' '%F %T' Australia/Brisbane strptimez; to-epoch;",
        "1664280627",
    );
    basic_test(
        "'2022' '%Y' Australia/Brisbane strptimez; '%F %T %z' strftime;",
        "\"2022-01-01 00:00:00 +1000\"",
    );

    basic_test("'2022-09-27' '%F' strptime; '%F' strftime;", "2022-09-27");
    basic_test(
        "'2022-09-27' '%F' strptime; '%F %T' strftime;",
        "\"2022-09-27 00:00:00\"",
    );
    basic_test(
        "'02' '%H' strptime; '%F %T' strftime;",
        "\"1970-01-01 02:00:00\"",
    );
    basic_test(
        "'02 +10:00' '%H %z' strptime; '%F %T %z' strftime;",
        "\"1970-01-01 02:00:00 +1000\"",
    );

    basic_test("\"2000-01-01 00:00:00\" \"%F %T\" Asia/Vladivostok strptimez; \"2000-01-01 00:00:00 +1000\" \"%F %T %z\" strptime; =",
               ".t");
    basic_test("\"2000-01-01 00:00:00\" \"%F %T\" Asia/Vladivostok strptimez; \"2000-01-01 00:00:00 +1000\" \"%F %T %z\" strptime; <",
               ".f");
    basic_test("\"2000-02-01 00:00:00\" \"%F %T\" Asia/Vladivostok strptimez; \"2000-01-01 00:00:00 +1000\" \"%F %T %z\" strptime; >",
               ".t");
}

#[test]
fn ip_test() {
    basic_test("1.0.0.0/24 ip", "v[ip 1.0.0.0/24]");
    basic_test("16777216 4 ip.from-int; str", "1.0.0.0");
    basic_test("1.0.0.0/24 ip; ip.addr", "1.0.0.0");
    basic_test("3.1.0.0/16 ip; ip.len", "16");
    basic_test("0.0.0.0/0 ip; ip.addr-int", "0");
    basic_test("16.0.0.0/7 ip; ip.last-addr", "17.255.255.255");
    basic_test("16.0.0.0/7 ip; ip.last-addr-int", "301989887");
    basic_test("1.0.0.0/24 ip; ip.size", "256");
    basic_test("1.0.0.0/24 ip; ip.version", "4");
    basic_test("1.0.0.0/24 ip; str", "1.0.0.0/24");

    basic_test("::/128 ip", "v[ip ::]");
    basic_test("10000000000 6 ip.from-int; str", "::2:540b:e400");
    basic_test("31CC::/64 ip; ip.addr", "31cc::");
    basic_test("305F:305F::/32 ip; ip.len", "32");
    basic_test("::2:540b:e400 ip; ip.addr-int", "10000000000");
    basic_test(
        "3000::/16 ip; ip.last-addr",
        "3000:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
    );
    basic_test(
        "3000::/16 ip; ip.last-addr-int",
        "63808136094534496727011269389785759743",
    );
    basic_test("::/112 ip; ip.size", "65536");
    basic_test(":: ip; ip.version", "6");
    basic_test("ABCD::/32 ip; str", "abcd::/32");

    basic_test("1.0.0.0-1.0.0.255 ip", "v[ip 1.0.0.0-1.0.0.255]");
    basic_test("1.0.0.0-1.0.0.255 ip; ip.addr", "1.0.0.0");
    basic_test("3.1.0.0-3.1.255.255 ip; ip.len", "16");
    basic_test("0.0.0.0-255.255.255.255 ip; ip.addr-int", "0");
    basic_test("16.0.0.0-17.255.255.255 ip; ip.last-addr", "17.255.255.255");
    basic_test("16.0.0.0-17.255.255.255 ip; ip.last-addr-int", "301989887");
    basic_test("1.0.0.0-1.0.0.255 ip; ip.size", "256");
    basic_test("1.0.0.0-1.0.0.255 ip; ip.version", "4");
    basic_test("1.0.0.0-1.0.0.255 ip; str", "1.0.0.0-1.0.0.255");

    basic_test("31CC::-31CC::ffff:ffff:ffff:ffff ip; ip.addr", "31cc::");
    basic_test(
        "305F:305F::-305F:305F:ffff:ffff:ffff:ffff:ffff:ffff ip; ip.len",
        "32",
    );
    basic_test("::2:540b:e400 ip; ip.addr-int", "10000000000");
    basic_test(
        "3000::-3000:ffff:ffff:ffff:ffff:ffff:ffff:ffff ip; ip.last-addr",
        "3000:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
    );
    basic_test(
        "3000::-3000:ffff:ffff:ffff:ffff:ffff:ffff:ffff ip; ip.last-addr-int",
        "63808136094534496727011269389785759743",
    );
    basic_test("::/112 ip; ip.size", "65536");
    basic_test(":: ip; ip.version", "6");
    basic_test(
        "ABCD::-ABCD:0000:ffff:ffff:ffff:ffff:ffff:ffff ip; str",
        "abcd::-abcd:0:ffff:ffff:ffff:ffff:ffff:ffff",
    );
}

#[test]
fn ipset_test() {
    basic_test(
        "0.0.0.0-1.0.0.0 ip; ip.prefixes; str map;",
        "(\n    0: 0.0.0.0/8\n    1: 1.0.0.0\n)",
    );
    basic_test(
        "0.0.0.0-1.0.0.0 ips; take-all; str map;",
        "(\n    0: 0.0.0.0/8\n    1: 1.0.0.0\n)",
    );
    basic_test("::-FFFF:: ip; ip.prefixes; str map;", "(\n    0: ::/1\n    1: 8000::/2\n    2: c000::/3\n    3: e000::/4\n    4: f000::/5\n    5: f800::/6\n    6: fc00::/7\n    7: fe00::/8\n    8: ff00::/9\n    9: ff80::/10\n    10: ffc0::/11\n    11: ffe0::/12\n    12: fff0::/13\n    13: fff8::/14\n    14: fffc::/15\n    15: fffe::/16\n    16: ffff::\n)");
    basic_test("::-FFFF:: ips; take-all; str map;", "(\n    0: ::/1\n    1: 8000::/2\n    2: c000::/3\n    3: e000::/4\n    4: f000::/5\n    5: f800::/6\n    6: fc00::/7\n    7: fe00::/8\n    8: ff00::/9\n    9: ff80::/10\n    10: ffc0::/11\n    11: ffe0::/12\n    12: fff0::/13\n    13: fff8::/14\n    14: fffc::/15\n    15: fffe::/16\n    16: ffff::\n)");

    basic_test(
        "1.0.0.0/8 ip; ip.prefixes; str map;",
        "(\n    0: 1.0.0.0/8\n)",
    );
    basic_test("0.0.0.251-0.0.5.16 ip; ip.prefixes; str map;",
               "(\n    0: 0.0.0.251\n    1: 0.0.0.252/30\n    2: 0.0.1.0/24\n    3: 0.0.2.0/23\n    4: 0.0.4.0/24\n    5: 0.0.5.0/28\n    6: 0.0.5.16\n)");
    basic_test("::/120 ip; ip.prefixes; str map;", "(\n    0: ::/120\n)");
    basic_test("1:0:0:0:0:0:0:1-1:0:0:0:0:0:0:8000 ip; ip.prefixes; str map;",
               "(\n    0: 1::1\n    1: 1::2/127\n    2: 1::4/126\n    3: 1::8/125\n    4: 1::10/124\n    5: 1::20/123\n    6: 1::40/122\n    7: 1::80/121\n    8: 1::100/120\n    9: 1::200/119\n    10: 1::400/118\n    11: 1::800/117\n    12: 1::1000/116\n    13: 1::2000/115\n    14: 1::4000/114\n    15: 1::8000\n)");

    basic_test("(0.0.0.0/8 1.0.0.0/8) ips; str", "0.0.0.0/7");
    basic_test("(:: ::1) ips; str", "::/127");
    basic_test("(::) ips; ::1 ips; union; str", "::/127");
    basic_test(
        "1.0.0.0-1.255.255.255 ips; 1.128.0.0-2.255.255.255 ips; isect; str",
        "1.128.0.0/9",
    );
    basic_test(
        "1.0.0.0-1.255.255.255 ips; 1.128.0.0-2.255.255.255 ips; diff; str",
        "1.0.0.0/9",
    );
    basic_test(
        "1.0.0.0-1.255.255.255 ips; 1.128.0.0-2.255.255.255 ips; symdiff; str",
        "1.0.0.0/9,2.0.0.0/8",
    );
    basic_test(
        "1.0.0.0-1.255.255.255 ips; take-all; str map",
        "(\n    0: 1.0.0.0/8\n)",
    );
    basic_test("1.0.0.0-1.255.255.255 ips; dup; =;", ".t");
    basic_test(
        "1.0.0.0-255.255.255.255 ips; take-all; shift; str;",
        "1.0.0.0/8",
    );
}

#[test]
fn set_test() {
    basic_test("s(1 2 3) 4 push;", "s(\n    1\n    2\n    3\n    4\n)");
    basic_test(
        "s(1 2 3) s(2 3 4) union;",
        "s(\n    1\n    2\n    3\n    4\n)",
    );
    basic_test("s(1 2 3) s(2 3 4) isect;", "s(\n    2\n    3\n)");
    basic_test("s(1 2 3) s(2 3 4) diff;", "s(\n    1\n)");
    basic_test("s(1 2 3) s(2 3 4) symdiff;", "s(\n    1\n    4\n)");
    basic_test("s(1 2 3) dup; shift;", "s(\n    2\n    3\n)\n1");
}

#[test]
fn predicate_test() {
    basic_test(".t is-bool;", ".t");
    basic_test(".f is-bool;", ".t");
    basic_test("100 is-bool;", ".f");

    basic_test("1000 is-int;", ".t");
    basic_test("0 is-int;", ".t");
    basic_test("10.0 is-int;", ".f");
    basic_test("10000000000000000000000000000000000 is-int;", ".f");

    basic_test("1000 is-bigint;", ".f");
    basic_test("10000000000000000000000000000000000 is-bigint;", ".t");

    basic_test("1000 is-str;", ".f");
    basic_test("\"1000\" is-str;", ".t");
    basic_test("asdf is-str;", ".t");
    basic_test("s(1 2 3) is-str;", ".f");

    basic_test("1 is-float;", ".f");
    basic_test("1.0 is-float;", ".t");
    basic_test("asdf is-float;", ".f");
    basic_test("\"1.0\" is-float;", ".f");
}

#[test]
fn bigint_conversion_test() {
    basic_test("1 bigint;", "1");
    basic_test(
        "1000000000000000000000000 bigint;",
        "1000000000000000000000000",
    );
    basic_test("asdf bigint;", "null");
}

#[test]
fn chr_test() {
    basic_test("100 chr;", "d");
    basic_test("100 bigint; chr;", "d");
    basic_error_test("-100 bigint; chr", "1:14: chr argument must be u32 integer");
}

#[test]
fn ord_test() {
    basic_test("d ord;", "100");
    basic_test("千 ord;", "21315");
    basic_error_test(
        "asdf ord;",
        "1:6: ord argument must be one character in length",
    );
}

#[test]
fn hex_test() {
    basic_test("5353 hex;", "21331");
    basic_test("0x5353 hex;", "21331");
    basic_test("0x5353535353535353 hex;", "6004234345560363859");
    basic_error_test("asdf hex;", "1:6: hex argument must be hexadecimal string");
}

#[test]
fn oct_test() {
    basic_test("777 oct;", "511");
}

#[test]
fn lc_test() {
    basic_test("AsDf lc;", "asdf");
    basic_error_test("[] lc;", "1:4: lc argument must be string");
}

#[test]
fn lcfirst_test() {
    basic_test("'' lcfirst;", "\"\"");
    basic_test("AsDf lcfirst;", "asDf");
    basic_error_test("[] lcfirst;", "1:4: lcfirst argument must be string");
}

#[test]
fn uc_test() {
    basic_test("AsDf uc;", "ASDF");
    basic_error_test("[] uc;", "1:4: uc argument must be string");
}

#[test]
fn ucfirst_test() {
    basic_test("'' ucfirst;", "\"\"");
    basic_test("asDf ucfirst;", "AsDf");
    basic_error_test("[] ucfirst;", "1:4: ucfirst argument must be string");
}

#[test]
fn reverse_test() {
    basic_test("(1 2 3) reverse;", "(\n    0: 3\n    1: 2\n    2: 1\n)");
    basic_test("asdf reverse;", "fdsa");
}

#[test]
fn sqrt_test() {
    basic_test("100 sqrt;", "10");
    basic_test("100.0 sqrt;", "10");
    basic_test("1 sqrt;", "1");
}

#[test]
fn exp_test() {
    basic_test("2 2 **;", "4");
    basic_test("100000000000000 50 **;", "10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
    basic_test("\"100\" \"2.0\" **;", "10000");
}

#[test]
fn abs_test() {
    basic_test("-100 abs;", "100");
    basic_test("-100.50 abs;", "100.5");
    basic_test("-10000000000000 abs;", "10000000000000");
}

#[test]
fn len_test() {
    basic_test("asdf len;", "4");
    basic_test("(1 2 3) len;", "3");
    basic_test("s(1 2 3) len;", "3");
    basic_test("h(1 2 3 4) len;", "2");
    basic_test("10 range; len;", "10");

    basic_test("\"\" empty;", ".t");
    basic_test("(1 2 3) empty;", ".f");
    basic_test("s() empty;", ".t");
    basic_test("h(1 2 3 4) empty;", ".f");
    basic_test("10 range; empty;", ".f");
}

#[test]
fn delete_test() {
    basic_test("h(1 2 3 4) dup; 1 delete;", "h(\n    \"3\": 4\n)");
}

#[test]
fn exists_test() {
    basic_test("s(1 2 3 4) 2 exists;", ".t");
}

#[cfg(target_os = "macos")]
#[test]
fn chmod_test() {
    basic_test(
        "() asdf f>; asdf 700 oct; chmod; {stat -f '%p' asdf}; shift; chomp; 700 m; asdf rm",
        ".t",
    );
}

#[cfg(not(target_os = "macos"))]
#[test]
fn chmod_test() {
    basic_test(
        "() asdf f>; asdf 700 oct; chmod; {stat -c '%a' asdf}; shift; chomp; 700 m; asdf rm",
        ".t",
    );
}

#[test]
fn stat_test() {
    basic_test("{rm -f asdf}; take-all; drop; {rm -f temp}; take-all; drop; Cargo.toml temp cp; {ln -s temp asdf}; take-all; drop; asdf stat; size get; 500 >; asdf lstat; size get; 100 <; and; {rm -f asdf}; take-all; drop; {rm -f temp}; take-all; drop;", ".t");
}

#[test]
fn mv_test() {
    basic_test("mvtest touch; mvtest mvtest2 rename; mvtest2 mvtest mv; mvtest stat; size get; 0 =; {rm -f mvtest}; take-all; {rm -f mvtest2}; take-all; drop; drop;", ".t");
}

#[test]
fn dir_test() {
    basic_test("dirtest mkdir; dirtest rmdir; .t", ".t");
}

#[test]
fn lsft_rsft_tests() {
    basic_test("1 1 <<;", "2");
    basic_test("1 31 <<;", "2147483648");
    basic_test("1 32 <<;", "4294967296");
    basic_test("1 33 <<;", "8589934592");
    basic_test("200 2 <<;", "800");
    basic_test("11111111111111 11 <<;", "22755555555555328");

    basic_test("1 1 >>;", "0");
    basic_test("500 3 >>;", "62");
}

#[test]
fn bitwise_tests() {
    basic_test("99 50 ||;", "115");
    basic_test("10 10 ||;", "10");
    basic_test("99 50 ^;", "81");
    basic_test("10 10 ^;", "0");
    basic_test("99 50 &;", "34");
    basic_test("10 10 &;", "10");
}

/* todo: Comment out temporarily, because it's failing in Github
 * Actions for some reason. */
/*
#[test]
fn stdout_stderr_tests() {
    basic_test("{perl test-misc/test.pl}; len; 3 =", ".t");

    basic_test("{perl test-misc/test.pl}/o; len; 3 =", ".t");
    basic_test("{perl test-misc/test.pl}/o; [output m] all", ".t");
    basic_test("{perl test-misc/test.pl}/o; [error m; not] all", ".t");

    basic_test("{perl test-misc/test.pl}/e; len; 3 =", ".t");
    basic_test("{perl test-misc/test.pl}/e; [output m; not] all", ".t");
    basic_test("{perl test-misc/test.pl}/e; [error m] all", ".t");

    basic_test("{perl test-misc/test.pl}/oe; len; 6 =", ".t");
    basic_test("{perl test-misc/test.pl}/oe; [output m] grep; len", "3");
    basic_test("{perl test-misc/test.pl}/oe; [error m] grep; len", "3");

    basic_test("{perl test-misc/test.pl}/c; [0 nth; 1 =] grep; len", "3");
    basic_test("{perl test-misc/test.pl}/c; [0 nth; 2 =] grep; len", "3");
    basic_test("{perl test-misc/test.pl}/c; len;", "6");
    basic_test("{perl test-misc/test.pl}/c; [0 nth; 1 =] grep; [1 nth] map; [output m] all", ".t");
    basic_test("{perl test-misc/test.pl}/c; [0 nth; 2 =] grep; [1 nth] map; [error  m] all", ".t");
}
*/

#[test]
fn append_generator_tests() {
    basic_test("2 range; 2 range; ++; 2 range; ++; '' join", "010101");
}

#[test]
fn env_tests() {
    basic_test("cosh_key cosh_value setenv; cosh_key getenv", "cosh_value");
    basic_test(
        "cosh_key cosh_value setenv; env; cosh_key get",
        "cosh_value",
    );
}

#[test]
fn regex_modifier_tests() {
    basic_test("asdf asdf m", ".t");
    basic_test("asdf asdf/i m", ".t");
    basic_test("asdF asdf/i m", ".t");
    basic_test("asdF asdf m", ".f");

    basic_test("\"asdf\\nasdf\" asdf.asdf m", ".f");
    basic_test("\"asdf\\nasdf\" asdf.asdf/s m", ".t");

    basic_test("\"asdf\\nasdf\" ^asdf.asdf$/s m", ".t");
    basic_test("\"asdf\\nasdf\" ^asdf$.^asdf$/s m", ".f");
    basic_test("\"asdf\\nasdf\" ^asdf$.^asdf$/sm m", ".t");

    basic_test("asdf_asdf_asdf asdf c", "(\n    0: asdf\n)");
    basic_test(
        "asdf_asdf_asdf asdf/g c",
        "(\n    0: asdf\n    1: asdf\n    2: asdf\n)",
    );
    basic_test(
        "asDf_aSdf_asdF asdf/ig c",
        "(\n    0: asDf\n    1: aSdf\n    2: asdF\n)",
    );
}

#[test]
fn regex_escape_tests() {
    basic_test("asdf asdf m", ".t");
    basic_test("asdf asdf\\/asdf m", ".f");
    basic_test("asdf/asdf asdf\\/asdf m", ".t");
    basic_test("asdf/asdF asdf\\/asdf m", ".f");
    basic_test("asdf/asdF asdf\\/asdf/i m", ".t");
}

#[test]
fn xml_ns_test() {
    basic_test(
        "test-misc/test.xml f<; '' join; from-xml; namespaces get; 0 get; name get;",
        "myns",
    );
    basic_test(
        "test-misc/test.xml f<; '' join; from-xml; key get;",
        "myns:top",
    );
    basic_test(
        "test-misc/test.xml f<; '' join; from-xml; value get; 1 get; namespaces get",
        "null",
    );
    basic_test(
        "test-misc/test.xml f<; '' join; from-xml; to-xml; xmlns:myns m",
        ".t",
    );
    basic_test(
        "test-misc/test.xml f<; '' join; from-xml; to-xml; myns:middle m",
        ".t",
    );
    basic_test(
        "test-misc/test.xml f<; '' join; from-xml; to-xml; myns:top m",
        ".t",
    );
    basic_test(
        "test-misc/test.xml f<; '' join; from-xml; to-xml; from-xml; to-xml; xmlns:myns m",
        ".t",
    );
    basic_test(
        "test-misc/test-default.xml f<; '' join; from-xml; to-xml; <middle> m",
        ".t",
    );
    basic_test(
        "test-misc/test-default.xml f<; from-xml; to-xml; <middle> m",
        ".t",
    );
}

#[test]
fn ips_gen_test() {
    basic_test(
        "test-misc/ipv4-data f<; chomp map; ips; str",
        "1.0.0.0/24,2.0.0.0/24",
    );
}

#[test]
fn tab_test() {
    basic_test(
        "asdf\\tqwer\\tzxcv \\t split",
        "(\n    0: asdf\n    1: qwer\n    2: zxcv\n)",
    );
}

#[test]
fn regex_escaping_test() {
    basic_test("\\\\n \\\\ p s", "pn");
    basic_test("10 \\d/g 3 s", "33");
    basic_test("\\\\d \\\\d 5 s", "5");
}

#[test]
fn cmp_test() {
    basic_test("100 150 <=>", "-1");
    basic_test("100 100 <=>", "0");
    basic_test("150 100 <=>", "1");
}

#[test]
fn fmt_test() {
    basic_test("1 2 \"{} {}\" fmt", "\"2 1\"");
    basic_test("1 2 \"{0} {1}\" fmt; nip; nip;", "\"2 1\"");
}

#[test]
fn pairwise_test() {
    basic_test(
        "3 range; 3 range; + pairwise; take-all;",
        "(\n    0: 0\n    1: 2\n    2: 4\n)",
    );
}

#[test]
fn slide_test() {
    basic_test(
        "4 range; ++ slide; take-all;",
        "(\n    0: 01\n    1: 12\n    2: 23\n)",
    );
}

#[test]
fn before_test() {
    basic_test(
        "5 range; [2 >] before; take-all;",
        "(\n    0: 0\n    1: 1\n    2: 2\n)",
    );
}

#[test]
fn after_test() {
    basic_test("5 range; [2 >] after; take-all;", "(\n    0: 4\n)");
}

#[test]
fn newline_command_test() {
    basic_test(
        "{perl test-misc/newline.pl}; len",
        "2"
    );
    basic_test(
        "{perl test-misc/newline.pl}/e; len",
        "2"
    );
}

#[test]
fn rpsl_parsem_with_trailing_blank() {
    basic_test(
        "
'rpsl' import;
test_input_with_blank.txt f<; rpsl.parsem; shift-all;
        ",
        "(
    0: (
        0: field1
        1: value1
    )
    1: (
        0: field2
        1: value2
    )
)
(
    0: (
        0: field3
        1: value3
    )
    1: (
        0: field4
        1: value4
    )
)"
    );
}

#[test]
fn rpsl_parsem_without_trailing_blank() {
    basic_test(
        "
'rpsl' import;
test_input_without_blank.txt f<; rpsl.parsem; shift-all;
        ",
        "(
    0: (
        0: field1
        1: value1
    )
    1: (
        0: field2
        1: value2
    )
)
(
    0: (
        0: field3
        1: value3
    )
    1: (
        0: field4
        1: value4
    )
)"
    );
}
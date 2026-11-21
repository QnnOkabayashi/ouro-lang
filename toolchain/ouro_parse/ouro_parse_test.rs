use ouro_parse::*;
use ouro_tokenize::tokenize;

fn pprint_parse_tree(input: &str) -> String {
    let tokenize = tokenize(input);
    let parse = parse(&tokenize.tokens);
    assert!(parse.ok.is_ok());

    let mut out = String::new();
    for (node, &node_kind) in parse.nodes.nodes.iter_enumerated() {
        use std::fmt::Write as _;
        write!(out, "{node_kind:?}").unwrap();

        if node_kind.has_token() {
            let span = tokenize.spans.to_span(parse.nodes.tokens[node]);
            let text = span.lookup(input);
            write!(out, " {text:?} {span:?}").unwrap();
        }
        writeln!(out).unwrap();
    }
    out
}

macro_rules! case {
    ($($tt:tt)*) => {
        pprint_parse_tree(stringify!($($tt)*))
    };
}

#[test]
fn test_parse() {
    insta::assert_snapshot!(case! {
        fn a() { 1 + 2 * 3 }
        struct Foo {
            struct Bar {
                fn foobar() {
                    let a = 4;
                    0b11_11
                }
            }
        }
        struct Baz {}
    });
}

#[test]
fn test_call_call() {
    insta::assert_snapshot!(case! {
        const CallCall = foo()();
    });
}

#[test]
fn test_str() {
    insta::assert_snapshot!(case! {
        const Name = "Quinn";
    });
}

#[test]
fn test_builtin() {
    insta::assert_snapshot!(case! {
        const Name = @This();
    });
}

#[test]
fn test_i32_and_type_keywords() {
    insta::assert_snapshot!(case! {
        fn Vec(T: type, len: i32) {
            //
        }
    });
}

#[test]
fn test_unary() {
    insta::assert_snapshot!(case! {
        fn a() { -5 + !x * --y }
    });
}

#[test]
fn test_fn_return_type() {
    insta::assert_snapshot!(case! {
        fn id(T: type, x: T) T {
            x
        }
    });
}

#[test]
fn test_if_else() {
    insta::assert_snapshot!(case! {
        fn f(k: bool) i32 { if k { 1 } else { 2 } }
    });
}

#[test]
fn test_if_without_else() {
    insta::assert_snapshot!(case! {
        fn f(k: bool) { if k { g() } }
    });
}

#[test]
fn test_else_if_needs_braces() {
    let input = "fn f(a: bool, b: bool) i32 { if a { 1 } else if b { 2 } else { 3 } }";
    let parse = parse(&tokenize(input).tokens);
    assert!(parse.ok.is_err());
}

#[test]
fn test_if_in_an_expression() {
    insta::assert_snapshot!(case! {
        const N = 1 + if k { 2 } else { 3 } + 4;
    });
}

#[test]
fn test_comparisons() {
    insta::assert_snapshot!(case! {
        const A = 1 + 2 < 3 * 4;
        const B = a > b;
        const C = a <= b;
        const D = a >= b;
        const E = a == b;
    });
}

#[test]
fn test_comparison_in_if() {
    insta::assert_snapshot!(case! {
        fn f(i: i32, n: i32) i32 { if i < n { i } else { n } }
    });
}

#[test]
fn test_comparisons_do_not_chain() {
    let input = "const A = a < b < c;";
    let parse = parse(&tokenize(input).tokens);
    assert!(parse.ok.is_err());
}

#[test]
fn test_struct_fields() {
    insta::assert_snapshot!(case! {
        struct Shape { kind: bool, data: i32 }
        struct Trailing { kind: bool, data: i32, }
        struct WithFn { len: i32, fn get() i32 { 0 } }
    });
}

#[test]
fn test_struct_fields_need_commas_between() {
    let input = "struct S { a: i32 b: i32 }";
    let parse = parse(&tokenize(input).tokens);
    assert!(parse.ok.is_err());
}

#[test]
fn test_named_call_args() {
    insta::assert_snapshot!(case! {
        const A = get(b, 3, p: trivial);
        const B = Shape(kind: true, data: 5);
        const C = Empty();
        fn make(k: bool) Shape { Shape(kind: k, data: Box(5)) }
    });
}

#[test]
fn test_named_call_arg_in_if_condition() {
    insta::assert_snapshot!(case! {
        fn f() i32 { if g(S(a: 1)) { 1 } else { 2 } }
    });
}

#[test]
fn test_positional_after_named_call_arg() {
    for input in ["const A = f(a: 1, 2);", "const A = f(a: 1, b);"] {
        let parse = parse(&tokenize(input).tokens);
        assert!(parse.ok.is_err(), "{input}");
    }
}

#[test]
fn test_top_level_field_at_eof() {
    insta::assert_snapshot!(case! { x: i32 });
}

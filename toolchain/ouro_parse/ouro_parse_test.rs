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
fn test_else_if() {
    insta::assert_snapshot!(case! {
        fn f(a: bool, b: bool) i32 { if a { 1 } else if b { 2 } else { 3 } }
    });
}

#[test]
fn test_if_in_an_expression() {
    insta::assert_snapshot!(case! {
        const N = 1 + if k { 2 } else { 3 } + 4;
    });
}

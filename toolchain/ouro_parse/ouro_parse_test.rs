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
        const Foo = struct {
            const Bar = struct {
                fn foobar() {
                    let a = 4;
                    0b11_11
                }
            };
        };
        const Baz = struct {};
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

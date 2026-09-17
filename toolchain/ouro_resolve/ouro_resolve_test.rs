use ouro_parse::parse;
use ouro_parse_types::{ExprKind, NodeKind};
use ouro_resolve::*;
use ouro_tokenize::tokenize;

fn pprint_name_resolve(input: &str) -> String {
    use std::fmt::Write as _;

    let tokenize = tokenize(input);
    let parse = parse(&tokenize.tokens);
    parse.ok.as_ref().unwrap();
    let resolve = resolve(&parse.nodes, &tokenize.spans, input);
    let mut iter = resolve.refs.into_iter();

    let mut out = String::new();
    for (node, &node_kind) in parse.nodes.nodes.iter_enumerated() {
        write!(out, "{node:?} {node_kind:?}").unwrap();
        if node_kind.has_token() {
            let string = tokenize
                .spans
                .to_span(parse.nodes.tokens[node])
                .lookup(input);
            write!(out, " {string:?}").unwrap();
        }

        if let NodeKind::Expr(ExprKind::Ident) = node_kind {
            let opt_def = iter.next().expect("should align");
            write!(out, " -> {opt_def:?}").unwrap();
        }
        writeln!(out).unwrap();
    }
    assert!(iter.next().is_none(), "didn't consume all synrefs?");
    out
}

macro_rules! case {
    ($($tt:tt)*) => {
        pprint_name_resolve(stringify!($($tt)*))
    };
}

#[test]
fn test_resolve() {
    insta::assert_snapshot!(case! {
        fn foo(a: i32, b: i32) {
            let c = add(a * a, b);
            c
        }

        fn add(a: i32, b: i32) {
            a + b + c
        }
    });
}

#[test]
fn test_struct_visible_in_its_body() {
    insta::assert_snapshot!(case! {
        const Thing = struct {
            fn new(a: 1) {
                Thing
            }
        };
    });
}

#[test]
fn test_params_can_refer_to_prior_params() {
    insta::assert_snapshot!(case! {
        fn foo(T: type, n: T) {}
    });
}

#[test]
fn test_hello() {
    insta::assert_snapshot!(case! {
        fn Vec(T: type) {
            struct {
                t: T,
            }
        }

        fn main() {
            // stuff
        }
    });
}

#[test]
fn debug() {
    insta::assert_snapshot!(case! {
        const Thing = struct {
            const Self = @This();

            fn new(self: Self) {
                self + i32
            }
        };

        fn VecImpl(T: type) {
            // When we enter a struct body, everything in here is runtime dependent.
            // Even types in type signatures.
            struct {
                // Runtime dep on this
                t: T,
                y: foo(),
            }
        }

        fn Vec(T: type) {
            VecImpl(T)
        }

        fn foo() {}

        // Since Vec depends on VecImpl, we need to ensure that we're below that.
        // Just ensure that the decl we're in is below the last decl that Vec uses
        fn print_ints(ints: Vec(i32)) {
            // stuff
        }

        fn print(s: string) {
            struct {
                fn fmt(io: Io) {
                    // print the stuff
                }
            }.fmt
        }

        pub fn main(io: Io) {
            print("hello")(io)
        }
    });
}

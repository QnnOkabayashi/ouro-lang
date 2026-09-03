use ouro_parse::parse;
use ouro_parse_node::{ExprKind, NodeKind};
use ouro_pprint::pprint;
use ouro_resolve::*;
use ouro_tokenize::tokenize;

fn pprint_name_resolve(input: &str) -> String {
    use std::fmt::Write as _;

    let tokenize = tokenize(input);
    let parse = parse(&tokenize.tokens);
    parse.ok.as_ref().unwrap();
    let resolve = resolve(&parse.nodes, &tokenize.ends, input);
    let mut iter = resolve.ref_to_referent.into_iter();

    let out = pprint(&parse, |node, out| {
        let node_impl = &parse.nodes[node];
        let string = ouro_tokenize::span(node_impl.token, &tokenize.ends).lookup(input);
        write!(out, "{:?} {:?} {:?}", node, node_impl.kind, string).unwrap();

        if let NodeKind::Expr(ExprKind::Ident) = node_impl.kind {
            let opt_def = iter.next().expect("should align");
            write!(out, " -> {opt_def:?}").unwrap();
        }
    });
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

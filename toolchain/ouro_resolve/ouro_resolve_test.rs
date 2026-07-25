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
    let resolve = resolve(&parse, &tokenize.ends, input);

    let mut output = pprint(&parse, |node, out| {
        let node_impl = &parse.nodes[node];
        let text = ouro_tokenize::span(node_impl.token, &tokenize.ends).lookup(input);

        write!(out, "{node:?} {:?} {text:?}", node_impl.kind).unwrap();
        if let NodeKind::Expr(ExprKind::Ident(syn_ref)) = parse.nodes[node].kind {
            let def_node = resolve.ref_to_referent[syn_ref];
            write!(out, " -> {def_node:?}").unwrap();
        }
    });
    if !resolve.errors.is_empty() {
        write!(&mut output, "Errors: {:?}", resolve.errors).unwrap();
    }
    output
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
            // each function is its own node in a dep graph.
            //
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
        // const Thing = struct {
        //     const Self = @This();

        //     fn new(self: *Self) Self {
        //         Self {}
        //     }
        // };

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

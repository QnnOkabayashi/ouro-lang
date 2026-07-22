use ouro_parse::*;
use ouro_pprint::pprint;
use ouro_tokenize::tokenize;

fn pprint_parse_tree(input: &str) -> String {
    let tokenize = tokenize(input);
    let parse = parse(&tokenize.tokens);
    assert!(parse.ok.is_ok());

    pprint(&parse.nodes, |node, out| {
        let node_impl = &parse.nodes[node];
        use std::fmt::Write as _;

        let span = ouro_tokenize::span(node_impl.token, &tokenize.ends);
        let text = span.lookup(input);
        write!(out, "{:?} {text:?} {span:?}", node_impl.kind).unwrap();
    })
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

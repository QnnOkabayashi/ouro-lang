use ouro_parse::parse;
use ouro_parse_types::NodeKind;
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
        write!(out, "{} {node_kind:?}", node.raw()).unwrap();
        if node_kind.has_token() {
            let string = tokenize
                .spans
                .to_span(parse.nodes.tokens[node])
                .lookup(input);
            write!(out, " {string:?}").unwrap();
        }

        if let NodeKind::Ident = node_kind {
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
        struct Thing {
            fn new(a: 1) {
                Thing
            }
        }
    });
}

#[test]
fn test_params_can_refer_to_prior_params() {
    insta::assert_snapshot!(case! {
        fn foo(T: type, n: T) {}
    });
}

#[test]
fn test_duplicate_definitions() {
    let resolve_src = |input: &str| {
        let tokenize = tokenize(input);
        let parse = parse(&tokenize.tokens);
        parse.ok.as_ref().unwrap();
        resolve(&parse.nodes, &tokenize.spans, input)
    };

    assert_eq!(resolve_src("fn foo() {} fn foo() {}").duplicates.len(), 1);
    assert_eq!(
        resolve_src("fn foo() {} fn foo() {} fn bar() { foo() }")
            .duplicates
            .len(),
        1
    );
    assert!(
        resolve_src("struct A { fn foo() {} } fn foo() {}")
            .duplicates
            .is_empty()
    );
    assert!(
        resolve_src("struct A { fn foo() {} } struct B { fn foo() {} }")
            .duplicates
            .is_empty()
    );
}

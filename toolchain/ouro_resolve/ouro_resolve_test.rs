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
    let node_source = parse.nodes.source(&tokenize.spans, input);
    for (node, &node_kind) in parse.nodes.nodes.iter_enumerated() {
        write!(out, "{} {node_kind:?}", node.raw()).unwrap();
        if node_kind.has_token() {
            let string = node_source.text(node);
            write!(out, " {string:?}").unwrap();
        }

        if let NodeKind::Ident = node_kind {
            let opt_def = iter.next().expect("should align");
            write!(out, " -> {:?}", opt_def.target).unwrap();
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

#[test]
fn test_let_shadowing() {
    insta::assert_snapshot!(case! {
        fn f(x: i32) {
            let x = x + 1;
            let x = { let a = 1; x };
            x
        }
    });
}

#[test]
fn test_ref_rings() {
    let input = "fn f(x: i32) { let y = x + x; let x = x; x + y }";
    let tokenize = tokenize(input);
    let parse = parse(&tokenize.tokens);
    parse.ok.as_ref().unwrap();
    let res = resolve(&parse.nodes, &tokenize.spans, input);

    let ring = |start: Ref| {
        let mut out = vec![start.raw()];
        let mut cur = res.refs[start].next.into_non_max().unwrap();
        while cur != start {
            out.push(cur.raw());
            cur = res.refs[cur].next.into_non_max().unwrap();
        }
        out
    };

    // Refs in order: 0=i32, 1=x, 2=x, 3=x (all param x), 4=x (let x), 5=y (let y)
    assert_eq!(ring(Ref::from_raw(1)), vec![1, 3, 2]);
    assert_eq!(ring(Ref::from_raw(4)), vec![4]);
    assert_eq!(ring(Ref::from_raw(5)), vec![5]);
}

#[test]
fn test_de_bruijn_across_params_blocks_and_shadowing() {
    insta::assert_snapshot!(case! {
        fn id(T: type, x: T) T {
            { let a = 1; x } + { let x = x + 1; x }
        }
    });
}

use ouro_span::{Byte, Span};
use ouro_tokenize::tokenize;

fn pprint_tokenize(input: &str) -> String {
    let tokenize = tokenize(input);
    let mut prev = Byte::new(0);
    let mut output = String::new();

    for (token, end) in tokenize.tokens.iter().zip(tokenize.ends.iter()) {
        use std::fmt::Write;
        writeln!(
            &mut output,
            "{:?} {:?} ",
            token,
            Span {
                start: prev,
                end: *end
            }
        )
        .expect("writing to a String cannot fail");
        prev = *end;
    }

    output
}

#[test]
fn t() {
    insta::assert_snapshot!(pprint_tokenize("fn f(x) { 4 + 3 * x } "));
    insta::assert_snapshot!(pprint_tokenize("a *= b;"));
    insta::assert_snapshot!(pprint_tokenize("a *"));
    assert_eq!('😎'.len_utf8(), 4);
    insta::assert_snapshot!(pprint_tokenize("a * 😎"));
    insta::assert_snapshot!(pprint_tokenize("a 1_000_000 123 _123"));
    insta::assert_snapshot!(pprint_tokenize("123😎"));
}

#[test]
fn valid_binary_numbers() {
    insta::assert_snapshot!(pprint_tokenize("0b1010"));
    insta::assert_snapshot!(pprint_tokenize("0b1010_1010"));
    insta::assert_snapshot!(pprint_tokenize("0b0"));
    insta::assert_snapshot!(pprint_tokenize("0b_0"));
}

#[test]
fn invalid_binary_numbers() {
    insta::assert_snapshot!(pprint_tokenize("0b"));
    insta::assert_snapshot!(pprint_tokenize("0b_"));
    insta::assert_snapshot!(pprint_tokenize("0b2"));
    insta::assert_snapshot!(pprint_tokenize("0b_2"));
    insta::assert_snapshot!(pprint_tokenize("0b101a"));
}

#[test]
fn newline_terminated_strings() {
    insta::assert_snapshot!(pprint_tokenize(r"\\Hello, world!"));
    insta::assert_snapshot!(pprint_tokenize(
        r"
        \\Hello, world!
        \\123 123\\
        "
    ));
}

#[test]
fn keywords() {
    insta::assert_snapshot!(pprint_tokenize("let fn comptime"));
    insta::assert_snapshot!(pprint_tokenize("letter fn_ comptime_"));
}

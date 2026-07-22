use ouro_span::Utf16Char;
use ouro_token_sum_tree::*;
use ouro_tokenize::{Token, TokenImpl, Tokenize, tokenize};

const CAP: usize = 64;

fn build(source: &str) -> (Tokenize, TokenSourceMap) {
    let tokenize_result = tokenize(source);
    let map = TokenSourceMap::new(&tokenize_result, source);
    (tokenize_result, map)
}

fn pos(row: u32, column: u32) -> RowColDelta<Utf16Char> {
    RowColDelta {
        row,
        column: Utf16Char::from_raw(column),
    }
}

#[test]
fn first_token_starts_at_origin() {
    let (_tokenize_result, map) = build("hello world");
    assert_eq!(map.token_to_position(Token::from(0)), pos(0, 0));
    assert_eq!(map.token_to_position(Token::from(1)), pos(0, 5));
    assert_eq!(map.token_to_position(Token::from(2)), pos(0, 6));
}

/// The single most important test here: for every token in the source,
/// mapping it to a position and back should return the same token.
/// This is exactly the test that would have caught the off-by-one bug
/// in `position_to_token` from last time.
#[test]
fn round_trip_every_token() {
    let source = "foo bar\nbaz qux\n\nlast line here";
    let (tokenize_result, map) = build(source);

    for token in tokenize_result.tokens.indices() {
        let position = map.token_to_position(token);
        let found = map.position_to_token(position);
        assert_eq!(
            found,
            Some(token),
            "round trip failed for token {token:?} at position {position:?}"
        );
    }
}

#[test]
fn newline_advances_row_and_resets_column() {
    let source = "abc\ndef";
    let (tokenize_result, map) = build(source);

    let newline_idx = tokenize_result
        .tokens
        .iter_enumerated()
        .find_map(|(token, token_impl)| matches!(token_impl, TokenImpl::Newline).then_some(token))
        .expect("expected a newline token in this source");

    let after_newline = Token::from(newline_idx.index() + 1);
    let position = map.token_to_position(after_newline);

    assert_eq!(position.row, 1);
    assert_eq!(position.column, Utf16Char::from(0));
}

/// A position sitting exactly on the boundary between two tokens should
/// resolve to the token that *starts* there, not the one that ends
/// there. This pins down the `Bias::Right` behavior in
/// `position_to_token` explicitly, rather than leaving it implicit.
#[test]
fn position_at_boundary_resolves_to_later_token() {
    let source = "ab cd";
    let (_tokenize_result, map) = build(source);

    let second_token_start = map.token_to_position(Token::from(1));
    assert_eq!(
        map.position_to_token(second_token_start),
        Some(Token::from(1))
    );
}

#[test]
fn position_past_end_of_source_returns_none() {
    let (_tokenize_result, map) = build("short");
    assert_eq!(map.position_to_token(pos(1_000, 0)), None);
}

#[test]
#[should_panic]
fn token_out_of_range_panics() {
    let (tokenize_result, map) = build("a b c");
    let out_of_range = Token::from(tokenize_result.tokens.len() + 10);
    map.token_to_position(out_of_range);
}

/// Force at least one seek across a chunk boundary (CAP = 64) so the
/// tree-level traversal actually gets exercised, not just in-chunk math.
#[test]
fn round_trip_across_chunk_boundary() {
    let source = "word ".repeat(100);
    let (tokenize_result, map) = build(&source);
    assert!(
        tokenize_result.tokens.len() as usize > CAP,
        "test input needs to produce more than {CAP} tokens to be meaningful"
    );

    for token in tokenize_result.tokens.indices() {
        let position = map.token_to_position(token);
        assert_eq!(map.position_to_token(position), Some(token));
    }
}

use std::ops::{Add, AddAssign};

use arrayvec::ArrayVec;
use ouro_span::{Unit, Utf16Char};
use ouro_tokenize::{Token, TokenImpl, Tokenize};
use zed_sum_tree::{Bias, Dimension, Dimensions, Item, SumTree, Summary};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct RowColDelta<U: Unit> {
    // Note: row must come first to make it have higher precedence in derived Ord impl.
    pub row: u32,
    pub column: U,
}

impl<U: Unit> Add for RowColDelta<U> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        if rhs.row == 0 {
            RowColDelta {
                row: self.row,
                column: self.column + rhs.column,
            }
        } else {
            RowColDelta {
                row: self.row + rhs.row,
                column: rhs.column,
            }
        }
    }
}

impl<U: Unit> AddAssign for RowColDelta<U> {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<'a> Dimension<'a, TokenSummary> for Token {
    fn zero(_cx: ()) -> Self {
        Token::from(0)
    }

    fn add_summary(&mut self, summary: &'a TokenSummary, _cx: ()) {
        *self += Token::from_raw(summary.token_count);
    }
}

impl<'a> Dimension<'a, TokenSummary> for RowColDelta<Utf16Char> {
    fn zero(_cx: ()) -> Self {
        RowColDelta::default()
    }
    fn add_summary(&mut self, summary: &'a TokenSummary, _cx: ()) {
        *self += summary.lines;
    }
}

#[derive(Clone, Debug, Default)]
struct TokenSummary {
    token_count: u32,
    lines: RowColDelta<Utf16Char>,
}

impl Summary for TokenSummary {
    type Context<'a> = ();

    fn zero<'a>(_cx: ()) -> Self {
        TokenSummary::default()
    }

    fn add_summary<'a>(&mut self, summary: &Self, _cx: ()) {
        self.token_count += summary.token_count;
        self.lines += summary.lines;
    }
}

const CAP: usize = 64;

#[derive(Clone, Debug)]
struct Chunk {
    // Each one corresponds with a token.
    row_col_deltas: ArrayVec<RowColDelta<Utf16Char>, CAP>,
}

impl Item for Chunk {
    type Summary = TokenSummary;

    fn summary(&self, _cx: ()) -> TokenSummary {
        let mut summary = TokenSummary::default();
        for &rcd in &self.row_col_deltas {
            summary.lines += rcd
        }
        summary.token_count += self.row_col_deltas.len() as u32;
        summary
    }
}

struct ArrayVecChunks<I, const CAP: usize> {
    iter: I,
}

impl<I: Iterator, const CAP: usize> ArrayVecChunks<I, CAP> {
    pub fn new(iter: I) -> Self {
        ArrayVecChunks { iter }
    }
}

impl<I: Iterator, const CAP: usize> Iterator for ArrayVecChunks<I, CAP> {
    type Item = ArrayVec<<I as Iterator>::Item, CAP>;

    fn next(&mut self) -> Option<Self::Item> {
        let array_vec: ArrayVec<<I as Iterator>::Item, CAP> =
            self.iter.by_ref().take(CAP).collect();
        if array_vec.is_empty() {
            return None;
        }

        Some(array_vec)
    }
}

#[derive(Debug)]
pub struct TokenSourceMap {
    sum_tree: SumTree<Chunk>,
}

impl TokenSourceMap {
    pub fn new(tokenize: &Tokenize, source: &str) -> Self {
        let iter = ArrayVecChunks::<_, CAP>::new(tokenize.tokens.iter_enumerated().map(
            |(token, &token_impl)| {
                if token_impl == TokenImpl::Newline {
                    RowColDelta {
                        row: 1,
                        column: Utf16Char::from(0),
                    }
                } else {
                    RowColDelta {
                        row: 0,
                        column: Utf16Char::len(
                            ouro_tokenize::span(token, &tokenize.ends).lookup(source),
                        ),
                    }
                }
            },
        ));
        TokenSourceMap {
            sum_tree: SumTree::from_iter(iter.map(|row_col_deltas| Chunk { row_col_deltas }), ()),
        }
    }

    /// Token -> (line, column)
    pub fn token_to_position(&self, token: Token) -> RowColDelta<Utf16Char> {
        let mut cursor = self
            .sum_tree
            .cursor::<Dimensions<Token, RowColDelta<Utf16Char>>>(());
        cursor.seek(&token, Bias::Left);

        let &Dimensions(start_token, mut delta, ()) = cursor.start();
        let len = token.index() - start_token.index();
        // we're in a chunk now.
        // go from start_token to token (accumulating delta as we go) until we're at token
        // then return delta.
        let chunk = cursor
            .item()
            .expect("token should be in range if it came from same Tokenize");
        for &row_col_delta in &chunk.row_col_deltas[..len] {
            delta += row_col_delta;
        }
        delta
    }

    /// (line, column) -> Token at that position
    pub fn position_to_token(&self, position: RowColDelta<Utf16Char>) -> Option<Token> {
        let mut cursor = self
            .sum_tree
            .cursor::<Dimensions<RowColDelta<Utf16Char>, Token>>(());
        cursor.seek(&position, Bias::Right);

        let chunk = cursor.item()?;
        let &Dimensions(mut cur, mut token, ()) = cursor.start();
        for &row_col_delta in &chunk.row_col_deltas[..] {
            cur += row_col_delta;
            if position < cur {
                return Some(token);
            }
            token += 1;
        }
        None
    }
}

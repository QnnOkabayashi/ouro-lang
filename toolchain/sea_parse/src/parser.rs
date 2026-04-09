use crate::node::{
    Counter, Node, NodeImpl, NodeKind, Nominal, SemRef, SubtreeSize, SynDef, SynRef,
};
use crate::tokenize::{Token, TokenImpl};
use index_vec::{IndexSlice, IndexVec};

struct ParseTree {
    nodes: IndexVec<Node, NodeImpl>,
    subtree_sizes: Vec<u32>,
}

impl ParseTree {
    fn with_capacity(capacity: usize) -> Self {
        ParseTree {
            nodes: IndexVec::with_capacity(capacity),
            // We start off with an entry, which tracks the number of nodes in the whole file.
            subtree_sizes: vec![0],
        }
    }

    fn increment_current_subtree_size(&mut self) {
        *self.subtree_sizes.last_mut().unwrap() += 1;
    }

    fn push(&mut self, token: Token, kind: NodeKind) {
        self.increment_current_subtree_size();
        self.nodes.push(NodeImpl { token, kind });
    }

    fn push_introducer(&mut self, token: Token, kind: NodeKind) {
        self.subtree_sizes.push(1);
        self.nodes.push(NodeImpl { token, kind });
    }

    fn push_terminator(&mut self, token: Token, kind: fn(SubtreeSize) -> NodeKind) {
        let subtree_size = self.subtree_sizes.pop().expect("Unmatched terminator");
        self.increment_current_subtree_size();
        self.nodes.push(NodeImpl {
            token,
            kind: kind(SubtreeSize(subtree_size)),
        });
    }
}

#[derive(Clone, Debug)]
pub struct Error {
    pub expected: Expected,
    pub actual: Option<TokenImpl>,
}

#[derive(Copy, Clone, Debug)]
pub enum Expected {
    Just(TokenImpl),
    OneOf(&'static [TokenImpl]),
}

struct Cursor<'a> {
    tokens: &'a IndexSlice<Token, [TokenImpl]>,
    index: Token,
}

impl<'a> Cursor<'a> {
    fn new(tokens: &'a IndexSlice<Token, [TokenImpl]>) -> Self {
        Cursor {
            tokens,
            index: Token::new(0),
        }
    }

    fn eat(&mut self, token: TokenImpl) -> Result<Token, Error> {
        if self.peek() == Some(token) {
            Ok(self.advance_1())
        } else {
            Err(Error {
                expected: Expected::Just(token),
                actual: self.peek(),
            })
        }
    }

    fn peek(&self) -> Option<TokenImpl> {
        self.tokens.get(self.index).copied()
    }

    fn advance_1(&mut self) -> Token {
        let index = self.index;
        self.index += 1;
        self.skip_whitespace();
        index
    }

    fn skip_whitespace(&mut self) {
        while let Some(TokenImpl::Comment | TokenImpl::Whitespace | TokenImpl::Newline) =
            self.peek()
        {
            self.index += 1;
        }
    }
}

struct Parser<'a> {
    cursor: Cursor<'a>,
    parse_tree: ParseTree,
    syn_defs: Counter<SynDef>,
    syn_refs: Counter<SynRef>,
    sem_refs: Counter<SemRef>,
    nominals: Counter<Nominal>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a IndexSlice<Token, [TokenImpl]>) -> Self {
        Parser {
            cursor: Cursor::new(tokens),
            parse_tree: ParseTree::with_capacity(tokens.len()),
            syn_defs: Counter::new(),
            syn_refs: Counter::new(),
            sem_refs: Counter::new(),
            nominals: Counter::new(),
        }
    }

    fn parse_items(&mut self) -> Result<(), Error> {
        self.cursor.skip_whitespace();
        loop {
            match self.cursor.peek() {
                Some(TokenImpl::Fn) => self.parse_fn()?,
                Some(TokenImpl::Struct) => self.parse_struct()?,
                Some(TokenImpl::Ident) => self.parse_field_decl()?,
                _ => return Ok(()),
            }
        }
    }

    fn parse_struct(&mut self) -> Result<(), Error> {
        self.parse_tree.push_introducer(
            self.cursor.eat(TokenImpl::Struct)?,
            NodeKind::Struct(self.nominals.next()),
        );
        self.parse_tree.push(
            self.cursor.eat(TokenImpl::Ident)?,
            NodeKind::StructIdent(self.syn_defs.next()),
        );
        self.parse_tree
            .push(self.cursor.eat(TokenImpl::OpenBrace)?, NodeKind::StructBody);
        self.parse_items()?;
        self.parse_tree.push_terminator(
            self.cursor.eat(TokenImpl::CloseBrace)?,
            NodeKind::StructBodyEnd,
        );
        Ok(())
    }

    fn parse_fn(&mut self) -> Result<(), Error> {
        self.parse_tree
            .push_introducer(self.cursor.eat(TokenImpl::Fn)?, NodeKind::Fn);
        self.parse_tree.push(
            self.cursor.eat(TokenImpl::Ident)?,
            NodeKind::FnIdent(self.syn_defs.next()),
        );
        self.parse_fn_params()?;
        self.parse_tree
            .push(self.cursor.eat(TokenImpl::OpenBrace)?, NodeKind::FnBody);
        self.parse_body()?;
        self.parse_tree
            .push_terminator(self.cursor.eat(TokenImpl::CloseBrace)?, NodeKind::FnBodyEnd);
        Ok(())
    }

    fn parse_fn_params(&mut self) -> Result<(), Error> {
        self.parse_tree
            .push_introducer(self.cursor.eat(TokenImpl::OpenParen)?, NodeKind::FnParams);
        let mut accept_comma = false;
        loop {
            match self.cursor.peek() {
                Some(TokenImpl::CloseParen) => {
                    self.parse_tree
                        .push_terminator(self.cursor.advance_1(), NodeKind::FnParamsEnd);
                    return Ok(());
                }
                Some(TokenImpl::Comma) if accept_comma => {
                    self.cursor.advance_1();
                    accept_comma = false;
                }
                _ => {
                    self.parse_fn_param()?;
                    accept_comma = true;
                }
            }
        }
    }

    fn parse_fn_param(&mut self) -> Result<(), Error> {
        let token = self.cursor.eat(TokenImpl::Ident)?;
        let kind = NodeKind::FnParamsParam(self.syn_defs.next());
        self.cursor.eat(TokenImpl::Colon)?;
        self.parse_expr()?;
        self.parse_tree.push(token, kind);
        Ok(())
    }

    fn parse_body(&mut self) -> Result<(), Error> {
        loop {
            match self.cursor.peek() {
                Some(TokenImpl::Fn) => self.parse_fn()?,
                Some(TokenImpl::Struct) => self.parse_struct()?,
                Some(TokenImpl::Let) => {
                    // Parse tree order: let expr = ident ;
                    // This simplifies name resolution because the ident shouldn't be in scope until after
                    // the expression.
                    self.parse_tree.push(self.cursor.advance_1(), NodeKind::Let);

                    let let_ident = self.cursor.eat(TokenImpl::Ident)?;
                    let let_eq = self.cursor.eat(TokenImpl::Eq)?;

                    self.parse_expr()?;

                    self.parse_tree.push(let_eq, NodeKind::LetEq);
                    self.parse_tree
                        .push(let_ident, NodeKind::LetIdent(self.syn_defs.next()));

                    self.parse_tree
                        .push(self.cursor.eat(TokenImpl::Semi)?, NodeKind::LetSemi);
                }
                Some(TokenImpl::CloseBrace) => return Ok(()),
                _ => return self.parse_expr(),
            }
        }
    }

    fn parse_expr(&mut self) -> Result<(), Error> {
        self.parse_term()?;
        loop {
            let kind = match self.cursor.peek() {
                Some(TokenImpl::Plus) => NodeKind::ExprAdd,
                Some(TokenImpl::Dash) => NodeKind::ExprSub,
                _ => {
                    return Ok(());
                }
            };
            let index = self.cursor.advance_1();
            self.parse_term()?;
            self.parse_tree.push(index, kind);
        }
    }

    fn parse_term(&mut self) -> Result<(), Error> {
        self.parse_factor()?;

        loop {
            let kind = match self.cursor.peek() {
                Some(TokenImpl::Star) => NodeKind::ExprMul,
                Some(TokenImpl::Slash) => NodeKind::ExprDiv,
                _ => {
                    return Ok(());
                }
            };
            let token = self.cursor.advance_1();
            self.parse_factor()?;
            self.parse_tree.push(token, kind);
        }
    }

    fn parse_factor(&mut self) -> Result<(), Error> {
        let kind = match self.cursor.peek() {
            Some(TokenImpl::Bang) => NodeKind::ExprNot,
            Some(TokenImpl::Dash) => NodeKind::ExprNeg,
            _ => return self.parse_atom(),
        };
        let token = self.cursor.advance_1();
        self.parse_factor()?;
        self.parse_tree.push(token, kind);
        self.parse_atom()
    }

    fn parse_atom(&mut self) -> Result<(), Error> {
        match self.cursor.peek() {
            Some(TokenImpl::OpenBrace) => {
                self.parse_tree
                    .push_introducer(self.cursor.advance_1(), NodeKind::ExprBlock);
                self.parse_body()?;
                self.parse_tree.push_terminator(
                    self.cursor.eat(TokenImpl::CloseBrace)?,
                    NodeKind::ExprBlockEnd,
                );
            }
            Some(TokenImpl::Ident) => {
                self.parse_tree.push(
                    self.cursor.advance_1(),
                    NodeKind::ExprIdent(self.syn_refs.next()),
                );
            }
            Some(TokenImpl::Int) => {
                self.parse_tree
                    .push(self.cursor.advance_1(), NodeKind::ExprInt);
            }
            actual => {
                return Err(Error {
                    expected: Expected::OneOf(&[
                        TokenImpl::OpenBrace,
                        TokenImpl::Ident,
                        TokenImpl::Int,
                    ]),
                    actual,
                });
            }
        }
        loop {
            match self.cursor.peek() {
                Some(TokenImpl::Dot) => {
                    self.parse_tree
                        .push(self.cursor.advance_1(), NodeKind::ExprDot);
                    self.parse_tree.push(
                        self.cursor.eat(TokenImpl::Ident)?,
                        NodeKind::ExprField(self.sem_refs.next()),
                    );
                }
                Some(TokenImpl::OpenParen) => {
                    self.parse_tree
                        .push_introducer(self.cursor.advance_1(), NodeKind::ExprCall);
                    let mut accept_comma = false;
                    loop {
                        match self.cursor.peek() {
                            Some(TokenImpl::CloseParen) => {
                                self.parse_tree.push_terminator(
                                    self.cursor.advance_1(),
                                    NodeKind::ExprCallEnd,
                                );
                                return Ok(());
                            }
                            Some(TokenImpl::Comma) if accept_comma => {
                                self.parse_tree
                                    .push(self.cursor.advance_1(), NodeKind::ExprCallComma);
                                accept_comma = false;
                            }
                            _ => {
                                self.parse_expr()?;
                                accept_comma = true;
                            }
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn parse_field_decl(&mut self) -> Result<(), Error> {
        self.parse_tree.push_introducer(
            self.cursor.eat(TokenImpl::Ident)?,
            NodeKind::StructFieldIdent,
        );
        self.parse_tree.push(
            self.cursor.eat(TokenImpl::Colon)?,
            NodeKind::StructFieldColon,
        );
        self.parse_expr()?;
        self.parse_tree.push_terminator(
            self.cursor.eat(TokenImpl::Comma)?,
            NodeKind::StructFieldComma,
        );
        Ok(())
    }
}

pub fn parse(tokens: &IndexSlice<Token, [TokenImpl]>) -> Parse {
    let mut parser = Parser::new(tokens);
    let ok = parser.parse_items();
    Parse {
        nodes: parser.parse_tree.nodes.into_boxed_slice(),
        syn_refs: parser.syn_refs,
        syn_defs: parser.syn_defs,
        ok,
    }
}

pub struct Parse {
    pub nodes: Box<IndexSlice<Node, [NodeImpl]>>,
    pub syn_refs: Counter<SynRef>,
    pub syn_defs: Counter<SynDef>,
    pub ok: Result<(), Error>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pprint::pprint;
    use crate::tokenize::tokenize;

    fn pprint_parse_tree(input: &str) -> String {
        let tokenize = tokenize(input);
        let parse = parse(&tokenize.tokens);
        assert!(parse.ok.is_ok());

        pprint(&parse.nodes, |node, out| {
            let node_impl = &parse.nodes[node];
            use std::fmt::Write as _;

            let span = crate::tokenize::span(node_impl.token, &tokenize.ends);
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
            struct Foo {
                struct Bar {
                    fn foobar() {
                        let a = 4;
                        0b11_11
                    }
                }
            }
            struct Baz {}
        });
    }
}

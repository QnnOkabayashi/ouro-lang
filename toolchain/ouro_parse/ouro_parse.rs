use ouro_index_vec::{Counter, IndexSlice, IndexVec};
use ouro_parse_node::{
    Node, NodeImpl, NodeKind, SemRef, SubtreeSize, SynDef, SynFn, SynRef, SynStruct,
};
use ouro_tokenize::{Token, TokenImpl};

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
    syn_structs: Counter<SynStruct>,
    syn_fns: Counter<SynFn>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a IndexSlice<Token, [TokenImpl]>) -> Self {
        Parser {
            cursor: Cursor::new(tokens),
            parse_tree: ParseTree::with_capacity(tokens.len()),
            syn_defs: Counter::new(),
            syn_refs: Counter::new(),
            sem_refs: Counter::new(),
            syn_structs: Counter::new(),
            syn_fns: Counter::new(),
        }
    }

    fn parse_struct_body(&mut self) -> Result<(), Error> {
        self.cursor.skip_whitespace();
        self.parse_visibility()?;
        loop {
            match self.cursor.peek() {
                Some(TokenImpl::Fn) => self.parse_fn()?,
                Some(TokenImpl::Ident) => self.parse_field_decl()?,
                Some(TokenImpl::Let) => self.parse_let()?,
                Some(TokenImpl::Const) => self.parse_const()?,
                _ => return Ok(()),
            }
        }
    }

    fn parse_visibility(&mut self) -> Result<(), Error> {
        if let Some(TokenImpl::Pub) = self.cursor.peek() {
            self.parse_tree.push(self.cursor.advance_1(), NodeKind::Pub);
        }
        Ok(())
    }

    fn parse_struct(&mut self) -> Result<(), Error> {
        self.parse_tree.push_introducer(
            self.cursor.eat(TokenImpl::Struct)?,
            NodeKind::Struct(self.syn_structs.next()),
        );
        self.parse_tree.push(
            self.cursor.eat(TokenImpl::OpenBrace)?,
            NodeKind::StructBodyBegin,
        );
        self.parse_struct_body()?;
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
        self.parse_tree.push(
            self.cursor.eat(TokenImpl::OpenBrace)?,
            NodeKind::FnBodyBegin,
        );
        self.parse_block_body()?;
        self.parse_tree
            .push_terminator(self.cursor.eat(TokenImpl::CloseBrace)?, NodeKind::FnBodyEnd);
        Ok(())
    }

    fn parse_fn_params(&mut self) -> Result<(), Error> {
        self.parse_tree.push_introducer(
            self.cursor.eat(TokenImpl::OpenParen)?,
            NodeKind::FnParams(self.syn_fns.next()),
        );
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
        self.parse_tree.push(
            self.cursor.eat(TokenImpl::Ident)?,
            NodeKind::FnParamsIdent(self.syn_defs.next()),
        );
        self.cursor.eat(TokenImpl::Colon)?;
        self.parse_expr()?;
        Ok(())
    }

    fn parse_block_body(&mut self) -> Result<(), Error> {
        loop {
            match self.cursor.peek() {
                Some(TokenImpl::Let) => self.parse_let()?,
                Some(TokenImpl::CloseBrace) => return Ok(()),
                _ => return self.parse_expr(),
            }
        }
    }

    fn parse_let(&mut self) -> Result<(), Error> {
        self.parse_tree.push(self.cursor.advance_1(), NodeKind::Let);
        // Parsed as `let expr = ident;` to make analysis easier.
        let ident = self.cursor.eat(TokenImpl::Ident)?;
        let eq = self.cursor.eat(TokenImpl::Eq)?;
        self.parse_expr()?;
        self.parse_tree.push(eq, NodeKind::LetEq);
        self.parse_tree
            .push(ident, NodeKind::LetIdent(self.syn_defs.next()));
        self.parse_tree
            .push(self.cursor.eat(TokenImpl::Semi)?, NodeKind::LetSemi);
        Ok(())
    }

    fn parse_const(&mut self) -> Result<(), Error> {
        self.parse_tree
            .push(self.cursor.advance_1(), NodeKind::Const);
        // Parsed as `const expr = ident;` to make analysis easier.
        let ident = self.cursor.eat(TokenImpl::Ident)?;
        let eq = self.cursor.eat(TokenImpl::Eq)?;
        self.parse_expr()?;
        self.parse_tree.push(eq, NodeKind::ConstEq);
        self.parse_tree
            .push(ident, NodeKind::ConstIdent(self.syn_defs.next()));
        self.parse_tree
            .push(self.cursor.eat(TokenImpl::Semi)?, NodeKind::ConstSemi);
        Ok(())
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
                self.parse_block_body()?;
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
            Some(TokenImpl::Struct) => self.parse_struct()?,
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
        self.cursor.eat(TokenImpl::Colon)?;
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
    let ok = parser.parse_struct_body();
    Parse {
        nodes: parser.parse_tree.nodes.into_boxed_slice(),
        syn_refs: parser.syn_refs,
        syn_defs: parser.syn_defs,
        syn_fns: parser.syn_fns,
        ok,
    }
}

#[derive(Debug)]
pub struct Parse {
    pub nodes: Box<IndexSlice<Node, [NodeImpl]>>,
    pub syn_refs: Counter<SynRef>,
    pub syn_defs: Counter<SynDef>,
    pub syn_fns: Counter<SynFn>,
    pub ok: Result<(), Error>,
}

use ouro_index_vec::{Delta, IndexSlice, IndexVec};
use ouro_parse_types::{NodeKind, Nodes};
use ouro_tokenize_types::{Token, TokenImpl};

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

#[derive(Copy, Clone)]
struct Cursor<'a> {
    tokens: &'a IndexSlice<Token, [TokenImpl]>,
    token: Token,
}

impl<'a> Cursor<'a> {
    fn new(tokens: &'a IndexSlice<Token, [TokenImpl]>) -> Self {
        Cursor {
            tokens,
            token: Token::from_raw(0),
        }
    }

    fn eat(&mut self, token: TokenImpl) -> Result<Token, Error> {
        self.check(token)?;
        Ok(self.advance_1())
    }

    fn peek(&self) -> Option<TokenImpl> {
        self.tokens.get(self.token).copied()
    }

    fn check(&self, token: TokenImpl) -> Result<(), Error> {
        if self.peek() == Some(token) {
            Ok(())
        } else {
            Err(Error {
                expected: Expected::Just(token),
                actual: self.peek(),
            })
        }
    }

    fn advance_1(&mut self) -> Token {
        let index = self.token;
        self.token += Delta::ONE;
        self.skip_whitespace();
        index
    }

    fn skip_whitespace(&mut self) {
        while let Some(TokenImpl::Comment | TokenImpl::Whitespace | TokenImpl::Newline) =
            self.peek()
        {
            self.token += Delta::ONE;
        }
    }
}

struct Parser<'a> {
    cursor: Cursor<'a>,
    nodes: Nodes,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a IndexSlice<Token, [TokenImpl]>) -> Self {
        Parser {
            cursor: Cursor::new(tokens),
            nodes: Nodes {
                nodes: IndexVec::with_capacity(tokens.len()),
                tokens: IndexVec::with_capacity(tokens.len()),
            },
        }
    }

    fn parse_file(&mut self) -> Result<(), Error> {
        self.nodes.push(self.cursor.token, NodeKind::FileBegin);
        self.parse_struct_body()?;
        self.nodes.push(self.cursor.token, NodeKind::FileEnd);
        Ok(())
    }

    fn parse_struct_body(&mut self) -> Result<(), Error> {
        loop {
            self.cursor.skip_whitespace();
            if let Some(TokenImpl::Pub) = self.cursor.peek() {
                self.nodes.push(self.cursor.advance_1(), NodeKind::Pub);
                self.cursor.skip_whitespace();
            }
            match self.cursor.peek() {
                Some(TokenImpl::Fn) => self.parse_fn()?,
                Some(TokenImpl::Ident) => self.parse_field_decl()?,
                Some(TokenImpl::Const) => self.parse_const()?,
                Some(TokenImpl::Struct) => self.parse_struct_decl()?,
                _ => return Ok(()),
            }
        }
    }

    fn parse_struct_decl(&mut self) -> Result<(), Error> {
        self.nodes
            .push(self.cursor.eat(TokenImpl::Struct)?, NodeKind::Struct);
        self.nodes
            .push(self.cursor.eat(TokenImpl::Ident)?, NodeKind::StructIdent);
        self.nodes.push(
            self.cursor.eat(TokenImpl::OpenBrace)?,
            NodeKind::StructBodyBegin,
        );
        self.parse_struct_body()?;
        self.nodes.push(
            self.cursor.eat(TokenImpl::CloseBrace)?,
            NodeKind::StructBodyEnd,
        );
        Ok(())
    }

    fn parse_fn(&mut self) -> Result<(), Error> {
        self.nodes
            .push(self.cursor.eat(TokenImpl::Fn)?, NodeKind::Fn);
        self.nodes
            .push(self.cursor.eat(TokenImpl::Ident)?, NodeKind::FnIdent);
        self.parse_fn_params()?;
        // Optional return type: `fn id(T: type, x: T) T { ... }`.
        if self.cursor.peek() != Some(TokenImpl::OpenBrace) {
            self.parse_expr()?;
        }
        self.nodes.push(
            self.cursor.eat(TokenImpl::OpenBrace)?,
            NodeKind::FnBodyBegin,
        );
        self.parse_block_body()?;
        self.nodes
            .push(self.cursor.eat(TokenImpl::CloseBrace)?, NodeKind::FnBodyEnd);
        Ok(())
    }

    fn parse_fn_params(&mut self) -> Result<(), Error> {
        self.nodes
            .push(self.cursor.eat(TokenImpl::OpenParen)?, NodeKind::FnParams);
        let mut accept_comma = false;
        loop {
            match self.cursor.peek() {
                Some(TokenImpl::CloseParen) => {
                    self.nodes
                        .push(self.cursor.advance_1(), NodeKind::FnParamsEnd);
                    return Ok(());
                }
                Some(TokenImpl::Comma) if accept_comma => {
                    let _ = self.cursor.advance_1();
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
        self.nodes
            .push(self.cursor.eat(TokenImpl::Ident)?, NodeKind::FnParamIdent);
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
        self.nodes.push(self.cursor.advance_1(), NodeKind::Let);
        self.nodes
            .push(self.cursor.eat(TokenImpl::Ident)?, NodeKind::LetIdent);
        self.nodes
            .push(self.cursor.eat(TokenImpl::Eq)?, NodeKind::LetEq);
        self.parse_expr()?;
        let _ = self.cursor.eat(TokenImpl::Semi)?;
        Ok(())
    }

    fn parse_const(&mut self) -> Result<(), Error> {
        self.nodes.push(self.cursor.advance_1(), NodeKind::Const);
        self.nodes
            .push(self.cursor.eat(TokenImpl::Ident)?, NodeKind::ConstIdent);
        self.nodes
            .push(self.cursor.eat(TokenImpl::Eq)?, NodeKind::ConstEq);
        self.parse_expr()?;
        let _ = self.cursor.eat(TokenImpl::Semi)?;
        Ok(())
    }

    fn parse_expr(&mut self) -> Result<(), Error> {
        self.parse_sum()?;
        let kind = match self.cursor.peek() {
            Some(TokenImpl::Lt) => Some(NodeKind::Lt),
            Some(TokenImpl::Gt) => Some(NodeKind::Gt),
            Some(TokenImpl::Le) => Some(NodeKind::Le),
            Some(TokenImpl::Ge) => Some(NodeKind::Ge),
            Some(TokenImpl::EqEq) => Some(NodeKind::EqEq),
            _ => None,
        };
        if let Some(kind) = kind {
            let token = self.cursor.advance_1();
            self.parse_sum()?;
            self.nodes.push(token, kind);
        }
        if self.cursor.peek().is_some() {
            self.nodes.push(self.cursor.token, NodeKind::EndOfExpr);
        }
        Ok(())
    }

    fn parse_sum(&mut self) -> Result<(), Error> {
        self.parse_term()?;
        loop {
            let kind = match self.cursor.peek() {
                Some(TokenImpl::Plus) => NodeKind::Add,
                Some(TokenImpl::Dash) => NodeKind::Sub,
                _ => return Ok(()),
            };
            let token = self.cursor.advance_1();
            self.parse_term()?;
            self.nodes.push(token, kind);
        }
    }

    fn parse_term(&mut self) -> Result<(), Error> {
        self.parse_factor()?;
        loop {
            let kind = match self.cursor.peek() {
                Some(TokenImpl::Star) => NodeKind::Mul,
                Some(TokenImpl::Slash) => NodeKind::Div,
                _ => return Ok(()),
            };
            let token = self.cursor.advance_1();
            self.parse_factor()?;
            self.nodes.push(token, kind);
        }
    }

    fn parse_factor(&mut self) -> Result<(), Error> {
        let kind = match self.cursor.peek() {
            Some(TokenImpl::Bang) => NodeKind::Not,
            Some(TokenImpl::Dash) => NodeKind::Neg,
            _ => return self.parse_atom(),
        };
        let token = self.cursor.advance_1();
        self.parse_factor()?;
        self.nodes.push(token, kind);
        Ok(())
    }

    fn parse_atom(&mut self) -> Result<(), Error> {
        match self.cursor.peek() {
            Some(TokenImpl::OpenBrace) => self.parse_block()?,
            Some(TokenImpl::If) => self.parse_if()?,
            Some(TokenImpl::Ident) => {
                self.nodes.push(self.cursor.advance_1(), NodeKind::Ident);
            }
            Some(TokenImpl::IntLit) => {
                self.nodes.push(self.cursor.advance_1(), NodeKind::IntLit);
            }
            Some(TokenImpl::StrLit) => {
                self.nodes.push(self.cursor.advance_1(), NodeKind::Str);
            }
            Some(TokenImpl::I32Keyword) => {
                self.nodes
                    .push(self.cursor.advance_1(), NodeKind::I32Keyword);
            }
            Some(TokenImpl::TypeKeyword) => {
                self.nodes
                    .push(self.cursor.advance_1(), NodeKind::TypeKeyword);
            }
            Some(TokenImpl::At) => self.parse_builtin()?,
            actual => {
                return Err(Error {
                    expected: Expected::OneOf(&[
                        TokenImpl::OpenBrace,
                        TokenImpl::If,
                        TokenImpl::Ident,
                        TokenImpl::IntLit,
                        TokenImpl::StrLit,
                        TokenImpl::I32Keyword,
                        TokenImpl::TypeKeyword,
                        TokenImpl::At,
                    ]),
                    actual,
                });
            }
        }
        loop {
            match self.cursor.peek() {
                Some(TokenImpl::Dot) => {
                    self.nodes.push(self.cursor.advance_1(), NodeKind::Dot);
                    self.nodes
                        .push(self.cursor.eat(TokenImpl::Ident)?, NodeKind::Field);
                }
                Some(TokenImpl::OpenParen) => self.parse_call_args()?,
                _ => return Ok(()),
            }
        }
    }

    fn parse_block(&mut self) -> Result<(), Error> {
        self.nodes
            .push(self.cursor.eat(TokenImpl::OpenBrace)?, NodeKind::Block);
        self.parse_block_body()?;
        self.nodes
            .push(self.cursor.eat(TokenImpl::CloseBrace)?, NodeKind::BlockEnd);
        Ok(())
    }

    fn parse_if(&mut self) -> Result<(), Error> {
        self.nodes
            .push(self.cursor.eat(TokenImpl::If)?, NodeKind::If);
        self.parse_expr()?;
        self.parse_block()?;
        if self.cursor.peek() == Some(TokenImpl::Else) {
            self.nodes.push(self.cursor.advance_1(), NodeKind::Else);
            self.parse_block()?;
        }
        Ok(())
    }

    fn parse_call_args(&mut self) -> Result<(), Error> {
        self.nodes.push(self.cursor.advance_1(), NodeKind::Call);
        let mut accept_comma = false;
        let mut in_kwargs = false;
        loop {
            match self.cursor.peek() {
                Some(TokenImpl::CloseParen) => {
                    self.nodes.push(self.cursor.advance_1(), NodeKind::CallEnd);
                    return Ok(());
                }
                Some(TokenImpl::Comma) if accept_comma => {
                    let _ = self.cursor.advance_1();
                    accept_comma = false;
                }
                peek => {
                    let mut lookahead = self.cursor;
                    let _ = lookahead.advance_1();
                    let second = lookahead.peek();
                    if peek == Some(TokenImpl::Ident) && second == Some(TokenImpl::Colon) {
                        self.nodes
                            .push(self.cursor.advance_1(), NodeKind::CallArgIdent);
                        let _colon = self.cursor.advance_1();
                        in_kwargs = true;
                    } else if in_kwargs {
                        return Err(match peek {
                            Some(TokenImpl::Ident) => Error {
                                expected: Expected::Just(TokenImpl::Colon),
                                actual: second,
                            },
                            actual => Error {
                                expected: Expected::Just(TokenImpl::Ident),
                                actual,
                            },
                        });
                    }
                    self.parse_expr()?;
                    accept_comma = true;
                }
            }
        }
    }

    fn parse_field_decl(&mut self) -> Result<(), Error> {
        self.nodes.push(
            self.cursor.eat(TokenImpl::Ident)?,
            NodeKind::StructFieldIdent,
        );
        self.cursor.eat(TokenImpl::Colon)?;
        self.parse_expr()?;
        match self.cursor.peek() {
            Some(TokenImpl::Comma) => {
                let _ = self.cursor.advance_1();
                Ok(())
            }
            Some(TokenImpl::CloseBrace) | None => Ok(()),
            actual => Err(Error {
                expected: Expected::OneOf(&[TokenImpl::Comma, TokenImpl::CloseBrace]),
                actual,
            }),
        }
    }

    fn parse_builtin(&mut self) -> Result<(), Error> {
        self.nodes
            .push(self.cursor.advance_1(), NodeKind::BuiltinAt);
        self.nodes
            .push(self.cursor.eat(TokenImpl::Ident)?, NodeKind::BuiltinIdent);
        self.cursor.skip_whitespace();
        self.cursor.check(TokenImpl::OpenParen)?;
        self.parse_call_args()
    }
}

pub fn parse(tokens: &IndexSlice<Token, [TokenImpl]>) -> Parse {
    let mut parser = Parser::new(tokens);
    let ok = parser.parse_file();
    Parse {
        nodes: parser.nodes,
        ok,
    }
}

#[derive(Debug)]
pub struct Parse {
    pub nodes: Nodes,
    pub ok: Result<(), Error>,
}

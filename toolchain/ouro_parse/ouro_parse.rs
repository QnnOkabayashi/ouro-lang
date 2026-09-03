use ouro_index_vec::{IndexSlice, IndexVec};
use ouro_parse_node::{ExprKind, Node, NodeImpl, NodeKind};
use ouro_tokenize::{Token, TokenImpl};

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
        self.check(token)?;
        Ok(self.advance_1())
    }

    fn peek(&self) -> Option<TokenImpl> {
        self.tokens.get(self.index).copied()
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
    nodes: IndexVec<Node, NodeImpl>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a IndexSlice<Token, [TokenImpl]>) -> Self {
        Parser {
            cursor: Cursor::new(tokens),
            nodes: IndexVec::with_capacity(tokens.len()),
        }
    }

    fn parse_struct_body(&mut self) -> Result<(), Error> {
        loop {
            self.cursor.skip_whitespace();
            if let Some(TokenImpl::Pub) = self.cursor.peek() {
                self.nodes.push(NodeImpl {
                    token: self.cursor.advance_1(),
                    kind: NodeKind::Pub,
                });
                self.cursor.skip_whitespace();
            }
            match self.cursor.peek() {
                Some(TokenImpl::Fn) => self.parse_fn()?,
                Some(TokenImpl::Ident) => self.parse_field_decl()?,
                Some(TokenImpl::Const) => self.parse_const()?,
                _ => return Ok(()),
            }
        }
    }

    fn parse_struct(&mut self) -> Result<(), Error> {
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::Struct)?,
            kind: NodeKind::Struct,
        });
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::OpenBrace)?,
            kind: NodeKind::StructBodyBegin,
        });
        self.parse_struct_body()?;
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::CloseBrace)?,
            kind: NodeKind::StructBodyEnd,
        });
        Ok(())
    }

    fn parse_fn(&mut self) -> Result<(), Error> {
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::Fn)?,
            kind: NodeKind::Fn,
        });
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::Ident)?,
            kind: NodeKind::FnIdent,
        });
        self.parse_fn_params()?;
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::OpenBrace)?,
            kind: NodeKind::FnBodyBegin,
        });
        self.parse_block_body()?;
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::CloseBrace)?,
            kind: NodeKind::FnBodyEnd,
        });
        Ok(())
    }

    fn parse_fn_params(&mut self) -> Result<(), Error> {
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::OpenParen)?,
            kind: NodeKind::FnParams,
        });
        let mut accept_comma = false;
        loop {
            match self.cursor.peek() {
                Some(TokenImpl::CloseParen) => {
                    self.nodes.push(NodeImpl {
                        token: self.cursor.advance_1(),
                        kind: NodeKind::FnParamsEnd,
                    });
                    return Ok(());
                }
                Some(TokenImpl::Comma) if accept_comma => {
                    self.nodes.push(NodeImpl {
                        token: self.cursor.advance_1(),
                        kind: NodeKind::FnParamsComma,
                    });
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
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::Ident)?,
            kind: NodeKind::FnParamsIdent,
        });
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
        self.nodes.push(NodeImpl {
            token: self.cursor.advance_1(),
            kind: NodeKind::Let,
        });
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::Ident)?,
            kind: NodeKind::LetIdent,
        });
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::Eq)?,
            kind: NodeKind::LetEq,
        });
        self.parse_expr()?;
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::Semi)?,
            kind: NodeKind::LetSemi,
        });
        Ok(())
    }

    fn parse_const(&mut self) -> Result<(), Error> {
        self.nodes.push(NodeImpl {
            token: self.cursor.advance_1(),
            kind: NodeKind::Const,
        });
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::Ident)?,
            kind: NodeKind::ConstIdent,
        });
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::Eq)?,
            kind: NodeKind::ConstEq,
        });
        self.parse_expr()?;
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::Semi)?,
            kind: NodeKind::ConstSemi,
        });
        Ok(())
    }

    fn parse_expr(&mut self) -> Result<(), Error> {
        self.parse_term()?;
        loop {
            let kind = match self.cursor.peek() {
                Some(TokenImpl::Plus) => NodeKind::Expr(ExprKind::Add),
                Some(TokenImpl::Dash) => NodeKind::Expr(ExprKind::Sub),
                _ => return Ok(()),
            };
            let token = self.cursor.advance_1();
            self.parse_term()?;
            self.nodes.push(NodeImpl { token, kind });
        }
    }

    fn parse_term(&mut self) -> Result<(), Error> {
        self.parse_factor()?;
        loop {
            let kind = match self.cursor.peek() {
                Some(TokenImpl::Star) => NodeKind::Expr(ExprKind::Mul),
                Some(TokenImpl::Slash) => NodeKind::Expr(ExprKind::Div),
                _ => return Ok(()),
            };
            let token = self.cursor.advance_1();
            self.parse_factor()?;
            self.nodes.push(NodeImpl { token, kind });
        }
    }

    fn parse_factor(&mut self) -> Result<(), Error> {
        let kind = match self.cursor.peek() {
            Some(TokenImpl::Bang) => NodeKind::Expr(ExprKind::Not),
            Some(TokenImpl::Dash) => NodeKind::Expr(ExprKind::Neg),
            _ => return self.parse_atom(),
        };
        let token = self.cursor.advance_1();
        self.parse_factor()?;
        self.nodes.push(NodeImpl { token, kind });
        self.parse_atom()
    }

    fn parse_atom(&mut self) -> Result<(), Error> {
        match self.cursor.peek() {
            Some(TokenImpl::OpenBrace) => {
                self.nodes.push(NodeImpl {
                    token: self.cursor.advance_1(),
                    kind: NodeKind::Expr(ExprKind::Block),
                });
                self.parse_block_body()?;
                self.nodes.push(NodeImpl {
                    token: self.cursor.eat(TokenImpl::CloseBrace)?,
                    kind: NodeKind::Expr(ExprKind::BlockEnd),
                });
            }
            Some(TokenImpl::Ident) => {
                self.nodes.push(NodeImpl {
                    token: self.cursor.advance_1(),
                    kind: NodeKind::Expr(ExprKind::Ident),
                });
            }
            Some(TokenImpl::IntLit) => {
                self.nodes.push(NodeImpl {
                    token: self.cursor.advance_1(),
                    kind: NodeKind::Expr(ExprKind::IntLit),
                });
            }
            Some(TokenImpl::Struct) => self.parse_struct()?,
            Some(TokenImpl::StrLit) => {
                self.nodes.push(NodeImpl {
                    token: self.cursor.advance_1(),
                    kind: NodeKind::Expr(ExprKind::Str),
                });
            }
            Some(TokenImpl::I32Keyword) => {
                self.nodes.push(NodeImpl {
                    token: self.cursor.advance_1(),
                    kind: NodeKind::Expr(ExprKind::I32Keyword),
                });
            }
            Some(TokenImpl::TypeKeyword) => {
                self.nodes.push(NodeImpl {
                    token: self.cursor.advance_1(),
                    kind: NodeKind::Expr(ExprKind::TypeKeyword),
                });
            }
            Some(TokenImpl::Ampersand) => self.parse_builtin()?,
            actual => {
                return Err(Error {
                    expected: Expected::OneOf(&[
                        TokenImpl::OpenBrace,
                        TokenImpl::Ident,
                        TokenImpl::IntLit,
                        TokenImpl::Struct,
                        TokenImpl::StrLit,
                        TokenImpl::I32Keyword,
                        TokenImpl::TypeKeyword,
                        TokenImpl::Ampersand,
                    ]),
                    actual,
                });
            }
        }
        loop {
            match self.cursor.peek() {
                Some(TokenImpl::Dot) => {
                    self.nodes.push(NodeImpl {
                        token: self.cursor.advance_1(),
                        kind: NodeKind::Expr(ExprKind::Dot),
                    });
                    self.nodes.push(NodeImpl {
                        token: self.cursor.eat(TokenImpl::Ident)?,
                        kind: NodeKind::Expr(ExprKind::Field),
                    });
                }
                Some(TokenImpl::OpenParen) => self.parse_call_args()?,
                _ => return Ok(()),
            }
        }
    }

    fn parse_call_args(&mut self) -> Result<(), Error> {
        self.nodes.push(NodeImpl {
            token: self.cursor.advance_1(),
            kind: NodeKind::Expr(ExprKind::Call),
        });
        let mut accept_comma = false;
        loop {
            match self.cursor.peek() {
                Some(TokenImpl::CloseParen) => {
                    self.nodes.push(NodeImpl {
                        token: self.cursor.advance_1(),
                        kind: NodeKind::Expr(ExprKind::CallEnd),
                    });
                    return Ok(());
                }
                Some(TokenImpl::Comma) if accept_comma => {
                    self.nodes.push(NodeImpl {
                        token: self.cursor.advance_1(),
                        kind: NodeKind::Expr(ExprKind::CallComma),
                    });
                    accept_comma = false;
                }
                _ => {
                    self.parse_expr()?;
                    accept_comma = true;
                }
            }
        }
    }

    fn parse_field_decl(&mut self) -> Result<(), Error> {
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::Ident)?,
            kind: NodeKind::StructFieldIdent,
        });
        self.cursor.eat(TokenImpl::Colon)?;
        self.parse_expr()?;
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::Comma)?,
            kind: NodeKind::StructFieldComma,
        });
        Ok(())
    }

    fn parse_builtin(&mut self) -> Result<(), Error> {
        self.nodes.push(NodeImpl {
            token: self.cursor.advance_1(),
            kind: NodeKind::BuiltinAmpersand,
        });
        self.nodes.push(NodeImpl {
            token: self.cursor.eat(TokenImpl::Ident)?,
            kind: NodeKind::BuiltinIdent,
        });
        self.cursor.skip_whitespace();
        self.cursor.check(TokenImpl::OpenParen)?;
        self.parse_call_args()
    }
}

pub fn parse(tokens: &IndexSlice<Token, [TokenImpl]>) -> Parse {
    let mut parser = Parser::new(tokens);
    let ok = parser.parse_struct_body();
    Parse {
        nodes: parser.nodes,
        ok,
    }
}

#[derive(Debug)]
pub struct Parse {
    pub nodes: IndexVec<Node, NodeImpl>,
    pub ok: Result<(), Error>,
}

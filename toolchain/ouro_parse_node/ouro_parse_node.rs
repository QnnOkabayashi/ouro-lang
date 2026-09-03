//! Nodes produced by the parser.

use ouro_tokenize::Token;

ouro_index_vec::define_index_type! {
    pub struct Node = u32;
}

#[derive(Copy, Clone, Debug)]
pub struct NodeImpl {
    pub token: Token,
    pub kind: NodeKind,
}

ouro_index_vec::define_index_type! {
    /// A syntactic reference.
    pub struct SynRef = u32;
    DEBUG_FORMAT = "SynRef({})";
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ExprKind {
    Add,
    Sub,
    Mul,
    Div,
    Not,
    Neg,
    Block,
    BlockEnd,
    Ident,
    IntLit,
    Dot,
    Field,
    Call,
    CallComma,
    CallEnd,
    Str,
    I32Keyword,
    TypeKeyword,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Pub,
    Struct,
    StructBodyBegin,
    StructFieldIdent,
    StructFieldComma,
    StructBodyEnd,
    Fn,
    FnIdent,
    FnParams,
    FnParamsIdent,
    FnParamsComma,
    FnParamsEnd,
    FnBodyBegin,
    FnBodyEnd,
    Let,
    LetIdent,
    LetEq,
    LetSemi,
    Const,
    ConstIdent,
    ConstEq,
    ConstSemi,
    Expr(ExprKind),
    BuiltinAmpersand,
    BuiltinIdent,
}

impl NodeKind {
    pub fn is_ident(self) -> bool {
        matches!(
            self,
            NodeKind::StructFieldIdent
                | NodeKind::FnIdent
                | NodeKind::FnParamsIdent
                | NodeKind::LetIdent
                | NodeKind::ConstIdent
                | NodeKind::Expr(ExprKind::Ident)
                | NodeKind::BuiltinIdent
        )
    }

    pub fn is_introducer(self) -> bool {
        matches!(
            self,
            NodeKind::Struct
                | NodeKind::StructFieldIdent
                | NodeKind::Fn
                | NodeKind::FnParams
                | NodeKind::Expr(ExprKind::Block)
                | NodeKind::Expr(ExprKind::Call)
        )
    }

    pub fn is_terminator(self) -> bool {
        matches!(
            self,
            NodeKind::StructBodyEnd
                | NodeKind::StructFieldComma
                | NodeKind::FnBodyEnd
                | NodeKind::FnParamsEnd
                | NodeKind::Expr(ExprKind::BlockEnd)
                | NodeKind::Expr(ExprKind::CallEnd)
        )
    }
}

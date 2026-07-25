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

#[derive(Copy, Clone, Debug)]
pub struct SubtreeSize(pub u32);

#[derive(Copy, Clone, Debug)]
pub enum NodeKind {
    Pub,
    Struct,
    StructBodyBegin,
    StructFieldIdent,
    StructFieldComma(SubtreeSize),
    StructBodyEnd(SubtreeSize),
    Fn,
    FnIdent,
    FnParams,
    FnParamsIdent,
    FnParamsEnd(SubtreeSize),
    FnBodyBegin,
    FnBodyEnd(SubtreeSize),
    Let,
    LetIdent,
    LetEq,
    LetSemi,
    Const,
    ConstIdent,
    ConstEq,
    ConstSemi,
    ExprAdd,
    ExprSub,
    ExprMul,
    ExprDiv,
    ExprNot,
    ExprNeg,
    ExprBlock,
    ExprBlockEnd(SubtreeSize),
    ExprIdent(SynRef),
    ExprInt,
    ExprDot,
    ExprField,
    ExprCall,
    ExprCallComma,
    ExprCallEnd(SubtreeSize),
    ExprStr,
}

impl NodeKind {
    pub fn subtree_size(self) -> Option<SubtreeSize> {
        match self {
            NodeKind::StructBodyEnd(subtree_size)
            | NodeKind::FnParamsEnd(subtree_size)
            | NodeKind::FnBodyEnd(subtree_size)
            | NodeKind::ExprBlockEnd(subtree_size)
            | NodeKind::ExprCallEnd(subtree_size) => Some(subtree_size),
            _ => None,
        }
    }
}

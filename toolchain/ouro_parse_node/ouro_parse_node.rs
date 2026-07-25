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
    pub fn has_subtree_size(self) -> bool {
        matches!(
            self,
            NodeKind::StructFieldComma(_)
                | NodeKind::StructBodyEnd(_)
                | NodeKind::FnParamsEnd(_)
                | NodeKind::FnBodyEnd(_)
                | NodeKind::ExprBlockEnd(_)
                | NodeKind::ExprCallEnd(_)
        )
    }

    pub fn is_introducer(self) -> bool {
        matches!(
            self,
            NodeKind::Struct
                | NodeKind::Fn
                | NodeKind::FnParams
                | NodeKind::ExprBlock
                | NodeKind::ExprCall
                | NodeKind::StructFieldIdent
        )
    }
}

use ouro_index_vec::{Counter, IndexSlice, index_box};
use ouro_parse::Parse;
use ouro_parse_node::{Node, NodeKind, SynRef};
use ouro_span::Byte;
use ouro_tokenize::Token;
use std::collections::HashMap;
use std::mem;

#[derive(Debug)]
pub struct Error {
    pub existing: Referent,
    pub conflicting_def: Node,
}

ouro_index_vec::define_index_type! {
    struct Symbol = u32;
}

ouro_index_vec::define_index_type! {
    pub struct ScopeId = u32;
}

#[derive(Copy, Clone, Debug)]
struct ScopeXor {
    a: ScopeId,
    b: ScopeId,
}

impl ScopeXor {
    fn new(a: ScopeId, b: ScopeId) -> Self {
        ScopeXor { a, b }
    }

    fn xor(self, a: ScopeId) -> ScopeId {
        if a == self.a {
            self.b
        } else if a == self.b {
            self.a
        } else {
            panic!("wasn't one of the original two")
        }
    }
}

#[derive(Copy, Clone)]
enum Inst {
    Def(Symbol, Node),
    Ref(Symbol, SynRef),
    /// Each Inst::ScopeTransition has exactly one matching pair with the same ScopeId.
    /// These two form the edges of a scope.
    ScopeTransition(ScopeXor),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Builtin {
    I32,
    Type,
}

impl Builtin {
    pub fn as_str(self) -> &'static str {
        match self {
            Builtin::I32 => "i32",
            Builtin::Type => "type",
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Referent {
    Local { node: Node, scope_id: ScopeId },
    Builtin(Builtin),
}

impl Referent {
    fn exists(self, open_scopes: &IndexSlice<ScopeId, [bool]>) -> bool {
        match self {
            Referent::Local { scope_id, .. } => open_scopes[scope_id],
            Referent::Builtin(_) => true,
        }
    }
}

#[derive(Debug)]
pub struct Resolve {
    pub ref_to_referent: Box<IndexSlice<SynRef, [Option<Referent>]>>,
    pub errors: Box<[Error]>,
    /// Points to the FnIdent of a top level main function, if any.
    pub top_level_main: Option<Node>,
}

#[derive(Default)]
struct Interner<'a> {
    dedup: HashMap<&'a str, Symbol>,
}

impl<'a> Interner<'a> {
    fn make_symbol(&mut self, s: &'a str) -> Symbol {
        use std::collections::hash_map::Entry::*;

        let len = self.dedup.len();
        match self.dedup.entry(s) {
            Occupied(occupied) => *occupied.get(),
            Vacant(vacant) => {
                let symbol = Symbol::new(len);
                vacant.insert(symbol);
                symbol
            }
        }
    }
}

pub fn resolve(parse: &Parse, ends: &IndexSlice<Token, [Byte]>, input: &str) -> Resolve {
    let mut next_scope_id = Counter::<ScopeId>::new();
    let root_scope = next_scope_id.next();
    let mut open_scopes = vec![];
    let mut curr_scope = root_scope;
    let mut interner = Interner::default();
    let builtins = [
        (interner.make_symbol("i32"), Referent::Builtin(Builtin::I32)),
        (
            interner.make_symbol("type"),
            Referent::Builtin(Builtin::Type),
        ),
    ];
    let sym_main = interner.make_symbol("main");

    let mut top_level_main = None;
    let mut insts = Vec::new();
    for (node, node_impl) in parse.nodes.iter_enumerated() {
        use NodeKind::*;
        match node_impl.kind {
            FnIdent(_) | FnParamsIdent(_) | LetIdent(_) | ConstIdent(_) => {
                let symbol =
                    interner.make_symbol(ouro_tokenize::span(node_impl.token, ends).lookup(input));

                // If this is a function that is top level and is named main
                if matches!(node_impl.kind, FnIdent(_))
                    && open_scopes.is_empty()
                    && symbol == sym_main
                {
                    top_level_main = Some(node);
                }
                insts.push(Inst::Def(symbol, node));
            }
            ExprIdent(syn_ref) => {
                insts.push(Inst::Ref(
                    interner.make_symbol(ouro_tokenize::span(node_impl.token, ends).lookup(input)),
                    syn_ref,
                ));
            }
            StructBodyBegin | FnParams | ExprBlock => {
                open_scopes.push(curr_scope);
                let prev_scope = curr_scope;
                curr_scope = next_scope_id.next();
                insts.push(Inst::ScopeTransition(ScopeXor::new(prev_scope, curr_scope)));
            }
            StructBodyEnd(_) | FnBodyEnd(_) | ExprBlockEnd(_) => {
                let prev_scope = curr_scope;
                curr_scope = open_scopes
                    .pop()
                    .expect("should be associated with a start of the scope");
                insts.push(Inst::ScopeTransition(ScopeXor::new(prev_scope, curr_scope)));
            }
            _ => {
                // The node isn't relevant for name resolution.
            }
        }
    }

    assert!(open_scopes.is_empty(), "should have exactly one left");

    let mut open_scopes_table: Box<IndexSlice<ScopeId, [bool]>> =
        index_box![false; next_scope_id.next.index()];
    open_scopes_table[root_scope] = true;

    let num_symbols = interner.dedup.len();
    let mut symbols: Box<IndexSlice<Symbol, [Option<Referent>]>> = index_box![None; num_symbols];

    for (symbol, def_slot) in builtins {
        symbols[symbol] = Some(def_slot);
    }

    let mut ref_to_referent: Box<IndexSlice<SynRef, [Option<Referent>]>> =
        index_box![None; parse.syn_refs.next.index()];

    let mut errors = Vec::new();
    let mut resolve_inst =
        |inst, symbol_to_referent: &mut IndexSlice<Symbol, [Option<Referent>]>| {
            match inst {
                Inst::Def(symbol, def) => {
                    if let Some(existing) = symbol_to_referent[symbol]
                        .filter(|referent| referent.exists(&open_scopes_table))
                    {
                        // This def shadows something else, report and error and do not write it to the
                        // table.
                        errors.push(Error {
                            existing,
                            conflicting_def: def,
                        });
                        return;
                    }
                    symbol_to_referent[symbol] = Some(Referent::Local {
                        node: def,
                        scope_id: curr_scope,
                    });
                }
                Inst::Ref(symbol, syn_ref) => {
                    // We found a node that is a ref.
                    let Some(referent) = symbol_to_referent[symbol]
                        .filter(|referent| referent.exists(&open_scopes_table))
                    else {
                        // Not referring to anything.
                        return;
                    };
                    let previous_def = mem::replace(&mut ref_to_referent[syn_ref], Some(referent));
                    assert!(
                        previous_def.is_none(),
                        "ambiguous defs should have been caught when def was added"
                    );
                }
                Inst::ScopeTransition(xor) => {
                    let prev = curr_scope;
                    curr_scope = xor.xor(prev);
                    let top_scope = prev.index().max(curr_scope.index());

                    open_scopes_table[top_scope] ^= true;
                }
            }
        };

    // Forward pass
    for &inst in insts.iter() {
        resolve_inst(inst, &mut symbols);
    }

    // Reset (zeroing the buffer)
    for def in &mut symbols[..] {
        *def = None;
    }

    // Backward pass
    for &inst in insts.iter().rev() {
        resolve_inst(inst, &mut symbols);
    }

    Resolve {
        ref_to_referent,
        errors: errors.into_boxed_slice(),
        top_level_main,
    }
}

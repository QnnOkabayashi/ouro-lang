use crate::node::{Capture, Node, NodeKind, SynDef, SynRef};
use crate::parser::Parse;
use crate::span::Offset;
use crate::tokenize::Token;
use index_vec::IndexSlice;
use std::collections::HashMap;
use std::mem;

#[derive(Debug)]
pub struct Error {
    pub shadowed: Shadowed,
    pub conflicting_def: Node,
}

#[derive(Debug)]
pub enum Shadowed {
    Builtin(Builtin),
    Local { def: Node },
}

index_vec::define_index_type! {
    struct Symbol = u32;
}

pub enum Referee {
    Parent(Capture),
    Local(SynDef),
}

// QUINN
// We will proceed with the existing algorithm, but with the following changes:
// We'll have a new stack that gets pushed to when a function scope begins, and popped from when it
// ends. The items on the stack will represent how many ScopeIds are we in deep right at the point
// of the function start. When checking to see if a def is in scope, we would previously scan the
// whole ScopeId stack. But now, we'll only scan the ones after the Function start. This ensures
// that we won't capture things from outside of the function scope. If we don't find a valid thing
// in scope, then we'll go to that function and say "hey you gotta capture this from ur parent
// because I need it".

#[derive(Copy, Clone)]
enum Inst {
    Def(Symbol, Node),
    Ref(Symbol, SynRef),
    /// Each Inst::ScopeTransition has exactly one matching pair with the same ScopeId.
    /// These two form the edges of a scope.
    ScopeTransition(ScopeId),
}

type ScopeId = u32;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Builtin {
    I32,
    Type,
}

#[derive(Copy, Clone)]
enum DefSlot {
    Local { def: Node, scope_id: ScopeId },
    NonLocal(Option<Builtin>),
}

impl DefSlot {
    /// Returns None if it's not shadowing anything.
    fn as_shadowed(self, scope_stack: &[ScopeId]) -> Option<Shadowed> {
        match self {
            DefSlot::Local { def, scope_id } if scope_stack.contains(&scope_id) => {
                Some(Shadowed::Local { def })
            }
            DefSlot::NonLocal(Some(builtin)) => Some(Shadowed::Builtin(builtin)),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Def {
    Local { def: Node },
    NonLocal(Option<Builtin>),
}

pub struct Resolve {
    pub defs: Box<IndexSlice<SynRef, [Def]>>,
    pub errors: Box<[Error]>,
}

pub fn resolve(parse: &Parse, ends: &IndexSlice<Token, [Offset]>, input: &str) -> Resolve {
    let mut insts = Vec::new();
    let mut scope_stack: Vec<ScopeId> = vec![1];
    let mut dedup = HashMap::new();
    let mut intern = |s| {
        use std::collections::hash_map::Entry::*;
        let len = dedup.len();
        match dedup.entry(s) {
            Occupied(occupied) => *occupied.get(),
            Vacant(vacant) => {
                let symbol = Symbol::new(len);
                vacant.insert(symbol);
                symbol
            }
        }
    };
    let sym_i32 = intern("i32");
    let sym_type = intern("type");

    let mut next_scope_id = 2;
    for (node, node_impl) in parse.nodes.iter_enumerated() {
        use NodeKind::*;
        match node_impl.kind {
            StructIdent(_) | FnIdent(_) | FnParamsParam(_) | LetIdent(_) => {
                insts.push(Inst::Def(
                    intern(crate::tokenize::span(node_impl.token, ends).lookup(input)),
                    node,
                ));
            }
            ExprIdent(syn_ref) => {
                insts.push(Inst::Ref(
                    intern(crate::tokenize::span(node_impl.token, ends).lookup(input)),
                    syn_ref,
                ));
            }
            StructBody | FnParams | ExprBlock => {
                insts.push(Inst::ScopeTransition(next_scope_id));
                scope_stack.push(next_scope_id);
                next_scope_id = next_scope_id
                    .checked_add(1)
                    .expect("shouldn't feasibly run out of scope IDs");
            }
            StructBodyEnd(_) | FnBodyEnd(_) | ExprBlockEnd(_) => {
                insts.push(Inst::ScopeTransition(
                    scope_stack
                        .pop()
                        .expect("should be associated with a start of the scope"),
                ));
            }
            _ => {
                // The node isn't relevant for name resolution.
            }
        }
    }

    assert_eq!(scope_stack.len(), 1, "should have exactly one left");

    let mut defs = index_vec::index_box![Def::NonLocal(None); parse.syn_refs.next.index()];

    let mut symbols: Box<IndexSlice<Symbol, [DefSlot]>> =
        index_vec::index_box![DefSlot::NonLocal(None); dedup.len()];
    symbols[sym_i32] = DefSlot::NonLocal(Some(Builtin::I32));
    symbols[sym_type] = DefSlot::NonLocal(Some(Builtin::Type));

    let mut errors = Vec::new();
    let mut resolve_inst = |inst, symbol_to_def: &mut IndexSlice<Symbol, [DefSlot]>| {
        match inst {
            Inst::Def(symbol, def) => {
                if let Some(shadowed) = symbol_to_def[symbol].as_shadowed(&scope_stack) {
                    // This def shadows something else, report and error and do not write it to the
                    // table.
                    errors.push(Error {
                        shadowed,
                        conflicting_def: def,
                    });
                    return;
                }
                symbol_to_def[symbol] = DefSlot::Local {
                    def,
                    scope_id: *scope_stack.last().unwrap(),
                };
            }
            Inst::Ref(symbol, syn_def) => {
                let live_def = match symbol_to_def[symbol] {
                    DefSlot::Local { def, scope_id } => {
                        if !scope_stack.contains(&scope_id) {
                            // Def is from a past scope and should be ignored.
                            return;
                        }
                        Def::Local { def }
                    }
                    DefSlot::NonLocal(Some(builtin)) => Def::NonLocal(Some(builtin)),
                    _ => {
                        // No def was found.
                        return;
                    }
                };
                let previous_def = mem::replace(&mut defs[syn_def], live_def);
                assert!(
                    matches!(previous_def, Def::NonLocal(None)),
                    "ambiguous defs should have been caught when def was added"
                );
            }
            Inst::ScopeTransition(scope_id) => {
                // TODO: use a bitset for this (or boolset idc)
                if *scope_stack.last().unwrap() == scope_id {
                    // Transitioning out of this scope
                    scope_stack.pop();
                } else {
                    // Transitioning into this scope
                    scope_stack.push(scope_id);
                }
            }
        }
    };

    // Forward pass
    for &inst in insts.iter() {
        resolve_inst(inst, &mut symbols);
    }

    // Reset (zeroing the buffer)
    for def in &mut symbols[..] {
        *def = DefSlot::NonLocal(None);
    }

    // Backward pass
    for &inst in insts.iter().rev() {
        resolve_inst(inst, &mut symbols);
    }

    Resolve {
        defs,
        errors: errors.into_boxed_slice(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::pprint::pprint;
    use crate::tokenize::tokenize;

    fn pprint_name_resolve(input: &str) -> String {
        use std::fmt::Write as _;

        let tokenize = tokenize(input);
        let parse = parse(&tokenize.tokens);
        parse.ok.as_ref().unwrap();
        let resolve = resolve(&parse, &tokenize.ends, input);

        let mut output = pprint(&parse.nodes, |node, out| {
            let node_impl = &parse.nodes[node];
            let text = crate::tokenize::span(node_impl.token, &tokenize.ends).lookup(input);

            if let NodeKind::ExprIdent(syn_ref) = parse.nodes[node].kind {
                let def_node = resolve.defs[syn_ref];
                write!(out, "{node:?} ExprIdent({def_node:?}) {text:?}").unwrap();
            } else {
                write!(out, "{node:?} {:?} {text:?}", node_impl.kind).unwrap();
            }
        });
        if !resolve.errors.is_empty() {
            write!(&mut output, "Errors: {:?}", resolve.errors).unwrap();
        }
        output
    }

    macro_rules! case {
        ($($tt:tt)*) => {
            pprint_name_resolve(stringify!($($tt)*))
        };
    }

    #[test]
    fn test_resolve() {
        insta::assert_snapshot!(case! {
            fn foo(a: i32, b: i32) {
                let c = add(a * a, b);
                c
            }

            fn add(a: i32, b: i32) {
                a + b + c
            }
        });
    }

    #[test]
    fn test_struct_visible_in_its_body() {
        insta::assert_snapshot!(case! {
            struct Thing {
                fn new(a: 1) {
                    Thing
                }
            }
        });
    }

    #[test]
    fn test_duplicate_defs_are_ignored() {
        insta::assert_snapshot!(case! {
            struct Thing {
                fn first() {
                    Thing
                }
                struct HidesDuplicate {
                    struct Thing {}
                    fn second() {
                        Thing
                    }
                }
                fn third() {
                    Thing
                }
            }
        });
    }

    #[test]
    fn test_params_can_refer_to_prior_params() {
        insta::assert_snapshot!(case! {
            fn foo(T: type, n: T) {}
        });
    }
}

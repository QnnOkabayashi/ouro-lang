use ouro_index_vec::{IndexSlice, IndexVec, MaxOr};
use ouro_parse_node::{ExprKind, Node, NodeImpl, NodeKind, SynRef};
use ouro_span::Byte;
use ouro_tokenize::Token;
use std::collections::HashMap;
use std::num::NonZero;

struct Text<'a> {
    ends: &'a IndexSlice<Token, [Byte]>,
    source: &'a str,
}

impl<'a> Text<'a> {
    fn as_str(&self, token: Token) -> &'a str {
        ouro_tokenize::span(token, self.ends).lookup(self.source)
    }
}

type ScopeId = NonZero<u32>;

struct ScopeFrame {
    scope_id: ScopeId,
    truncate_defs_to: usize,
}

pub struct Resolver<'source> {
    dedup: HashMap<&'source str, MaxOr<SynRef>>,
    text: Text<'source>,

    // TODO: next objective is to combine scope_stack and def_stack
    // Importantly: need a way to have nodes that do not correspond to tokens.
    // This goes up and down.
    scope_stack: Vec<ScopeFrame>,
    top_scope_id: ScopeId,

    // The defs that are queued up to trigger at the end of their scope.
    def_stack: Vec<Node>,

    // where to get a fresh scope_id for scope_stack
    next_scope_id: ScopeId,

    // The final buffer that we return: iterate through this and you'll get where everything points
    result: IndexVec<SynRef, Entry>,
}

// Instead of only storing ref to def, it would also be nice to store def to refs as well
// in the buffer

#[derive(Debug)]
pub enum Entry {
    Def(Node),
    Undef {
        scope_id: ScopeId,
        next: MaxOr<SynRef>,
    },
}
const _: () = assert!(size_of::<Entry>() == 8);

// TODO(okabayashi): this isn't used yet, but we should make the result buffer store
// refs AND defs and have them form cycles.
pub struct Entry2 {
    // for refs, these would point to either defs or their opening scope thingy
    // for defs, this points to its opening scope thingy
    node: Node,

    // these all join point in a circularly linked list
    next: SynRef,
}
const _: () = assert!(size_of::<Entry2>() == 8);

impl<'source> Resolver<'source> {
    fn scope_open(&mut self) {
        self.top_scope_id = self.next_scope_id;
        self.scope_stack.push(ScopeFrame {
            scope_id: self.next_scope_id,
            truncate_defs_to: self.def_stack.len(),
        });
        self.next_scope_id = self.next_scope_id.saturating_add(1);
    }

    fn scope_close(&mut self, nodes: &IndexSlice<Node, [NodeImpl]>) {
        let scope_frame = self
            .scope_stack
            .pop()
            .expect("matched by a preceeding push");
        self.top_scope_id = scope_frame.scope_id;

        for def in self.def_stack.drain(scope_frame.truncate_defs_to..).rev() {
            let string = self.text.as_str(nodes[def].token);
            let Some(slot) = self.dedup.get_mut(string) else {
                continue;
            };

            let mut max_or_entry = *slot;

            while let Some(synref) = max_or_entry.into_non_max() {
                let Entry::Undef {
                    scope_id: result_scope_id,
                    next,
                } = self.result[synref]
                else {
                    unreachable!();
                };
                if result_scope_id < self.top_scope_id {
                    break;
                }
                // We're on an entry that points to a slot that needs to be rewritten!
                self.result[synref] = Entry::Def(def);

                max_or_entry = next;
            }
            // Put what we finished on back
            *slot = max_or_entry;
        }
    }
}

#[derive(Debug)]
pub struct Resolve {
    pub ref_to_referent: IndexVec<SynRef, Entry>,
}

// 1{ def ref } 2{ ref def }
// go forward until you find a def, ensure def is visible from first ref, then keep going until that def goes out of scope
// { def { def } } -> "I won't look at anything before my ScopeId"
//
// { ref } { def } -> "I wont look at anything before my ScopeId"

pub fn resolve(
    nodes: &IndexSlice<Node, [NodeImpl]>,
    ends: &IndexSlice<Token, [Byte]>,
    source: &str,
) -> Resolve {
    let mut resolver = Resolver {
        dedup: HashMap::new(),
        text: Text { ends, source },
        scope_stack: Vec::new(),
        top_scope_id: const { NonZero::new(1).unwrap() },
        def_stack: Vec::new(),
        next_scope_id: const { NonZero::new(1).unwrap() },
        result: IndexVec::new(),
    };

    resolver.scope_open();
    for (node, node_impl) in nodes.iter_enumerated() {
        match node_impl.kind {
            NodeKind::FnIdent
            | NodeKind::FnParamsIdent
            | NodeKind::LetIdent
            | NodeKind::ConstIdent => {
                resolver.def_stack.push(node);
            }
            NodeKind::Expr(ExprKind::Ident) => {
                let slot = resolver
                    .dedup
                    .entry(resolver.text.as_str(node_impl.token))
                    .or_insert(MaxOr::max());
                let next = *slot;
                *slot = MaxOr::new(resolver.result.push(Entry::Undef {
                    scope_id: resolver.top_scope_id,
                    next,
                }));
            }
            NodeKind::StructBodyBegin | NodeKind::FnParams | NodeKind::Expr(ExprKind::Block) => {
                resolver.scope_open();
                // resolver.def_stack.push(node);
            }
            NodeKind::StructBodyEnd | NodeKind::FnBodyEnd | NodeKind::Expr(ExprKind::BlockEnd) => {
                resolver.scope_close(nodes);
            }
            _ => {}
        }
    }
    resolver.scope_close(nodes);

    Resolve {
        ref_to_referent: resolver.result,
    }
}

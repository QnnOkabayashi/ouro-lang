use ouro_index_vec::{Delta, IndexVec, MaxOr};
use ouro_parse_types::{Node, NodeKind, NodeSource, Nodes};
use ouro_tokenize_types::ByteSpans;
use std::collections::HashMap;

ouro_index_vec::define_index_type! {
    pub struct Ref = u32;
}

ouro_index_vec::define_index_type! {
    pub struct Item = u32;
}

#[derive(Copy, Clone, Debug)]
pub enum RefTarget {
    Unresolved { scope_start: Node, depth: u32 },
    Item { def_node: Node, item: Item },
    Local { def_node: Node, de_bruijn: u32 },
}

impl RefTarget {
    pub fn def_node(self) -> Option<Node> {
        match self {
            RefTarget::Item { def_node, .. } | RefTarget::Local { def_node, .. } => Some(def_node),
            RefTarget::Unresolved { .. } => None,
        }
    }
}

#[derive(Debug)]
pub struct RefImpl {
    pub target: RefTarget,
    pub next: MaxOr<Ref>,
}

enum StackEntry {
    Open { prev_scope_start: Node },
    Def { def_node: Node, kind: DefKind },
}

enum DefKind {
    Item(Item),
    Local { prev_scope_start: Node },
}

#[derive(Default)]
struct DedupEntry {
    head: MaxOr<Ref>,
    last_scope: MaxOr<Node>,
}

#[derive(Default)]
struct Resolver<'a> {
    next_node: Node,
    scope_start: Node,
    local_depth: u32,
    stack: Vec<StackEntry>,
    dedup: HashMap<&'a str, DedupEntry>,
    out: Resolve,
}

impl<'a> Resolver<'a> {
    fn step(&mut self, node_kind: NodeKind, source: NodeSource<'a>) {
        let node = self.next_node;
        self.next_node += Delta::ONE;
        match node_kind {
            NodeKind::StructIdent | NodeKind::FnIdent | NodeKind::ConstIdent => {
                self.stack.push(StackEntry::Def {
                    def_node: node,
                    kind: DefKind::Item(self.out.items.push(node)),
                });
            }
            NodeKind::FnParamIdent | NodeKind::LetIdent => {
                self.stack.push(StackEntry::Def {
                    def_node: node,
                    kind: DefKind::Local {
                        prev_scope_start: self.scope_start,
                    },
                });
                self.scope_start = node;
                self.local_depth += 1;
            }
            NodeKind::Ident => {
                let entry = self.dedup.entry(source.text(node)).or_default();
                entry.head = MaxOr::new(self.out.refs.push(RefImpl {
                    target: RefTarget::Unresolved {
                        scope_start: self.scope_start,
                        depth: self.local_depth,
                    },
                    next: entry.head,
                }));
            }
            NodeKind::FileBegin
            | NodeKind::StructBodyBegin
            | NodeKind::FnParams
            | NodeKind::Block => {
                self.stack.push(StackEntry::Open {
                    prev_scope_start: self.scope_start,
                });
                self.scope_start = node;
            }
            NodeKind::FileEnd
            | NodeKind::StructBodyEnd
            | NodeKind::FnBodyEnd
            | NodeKind::BlockEnd => loop {
                let (def_node, def_kind) = match self.stack.pop().expect(
                    "shouldn't run out of elements because we stop as soon as we hit matching open",
                ) {
                    StackEntry::Open { prev_scope_start } => {
                        self.scope_start = prev_scope_start;
                        break;
                    }
                    StackEntry::Def { def_node, kind } => (def_node, kind),
                };

                let entry = self.dedup.entry(source.text(def_node)).or_default();

                if let DefKind::Item(item) = def_kind {
                    if entry.last_scope.into_non_max() == Some(self.scope_start) {
                        self.out.duplicates.push(item);
                        continue;
                    }
                    entry.last_scope = MaxOr::new(self.scope_start);
                }

                let first_ref = entry.head;
                let mut last_ref = MaxOr::max();
                while let Some(head_ref) = entry.head.into_non_max() {
                    let RefTarget::Unresolved {
                        scope_start: ref_scope_start,
                        depth: ref_depth,
                    } = self.out.refs[head_ref].target
                    else {
                        unreachable!("pending list only contains unresolved refs");
                    };
                    if ref_scope_start < self.scope_start {
                        break;
                    }
                    self.out.refs[head_ref].target = match def_kind {
                        DefKind::Item(item) => RefTarget::Item { def_node, item },
                        DefKind::Local { .. } => RefTarget::Local {
                            def_node,
                            de_bruijn: ref_depth - self.local_depth,
                        },
                    };
                    entry.head = self.out.refs[head_ref].next;
                    last_ref = MaxOr::new(head_ref);
                }
                if let Some(last_ref) = last_ref.into_non_max() {
                    self.out.refs[last_ref].next = first_ref;
                }
                if let DefKind::Local { prev_scope_start } = def_kind {
                    self.scope_start = prev_scope_start;
                    self.local_depth -= 1;
                }
            },
            _ => {}
        }
    }
}

pub fn resolve(nodes: &Nodes, spans: &ByteSpans, source: &str) -> Resolve {
    let mut resolver = Resolver::default();
    let node_source = nodes.source(spans, source);
    for &node_kind in &nodes.nodes {
        resolver.step(node_kind, node_source);
    }
    resolver.out
}

#[derive(Debug, Default)]
pub struct Resolve {
    pub refs: IndexVec<Ref, RefImpl>,
    pub items: IndexVec<Item, Node>,
    pub duplicates: Vec<Item>,
}

use ouro_parse::Parse;
use ouro_parse_node::Node;

#[derive(Copy, Clone)]
enum Char {
    Introducer,
    Leaf,
}

impl Char {
    fn character(self) -> char {
        match self {
            Char::Introducer => '╭',
            Char::Leaf => '├',
        }
    }
}

pub fn pprint(parse: &Parse, mut f: impl FnMut(Node, &mut String)) -> String {
    let mut indent = 0;
    let mut out = String::new();
    for (node, node_impl) in parse.nodes.iter_enumerated() {
        let ch = if node_impl.kind.is_introducer() {
            indent += 1;
            Char::Introducer
        } else {
            if node_impl.kind.is_terminator() {
                indent -= 1;
            }
            Char::Leaf
        };

        for _ in 0..indent {
            out.push_str("│ ");
        }
        out.push(ch.character());
        out.push('─');
        f(node, &mut out);
        out.push('\n');
    }
    out
}

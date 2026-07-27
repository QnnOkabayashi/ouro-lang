use ouro_parse::Parse;
use ouro_parse_node::Node;

pub fn pprint(parse: &Parse, mut f: impl FnMut(Node, &mut String)) -> String {
    let mut indent = 0;
    let mut out = String::new();
    for (node, node_impl) in parse.nodes.iter_enumerated() {
        let ch = if node_impl.kind.is_introducer() {
            indent += 1;
            "╭─"
        } else {
            if node_impl.kind.is_terminator() {
                indent -= 1;
            }
            "├─"
        };

        for _ in 0..indent {
            out.push_str("│ ");
        }
        out.push_str(ch);
        f(node, &mut out);
        out.push('\n');
    }
    out
}

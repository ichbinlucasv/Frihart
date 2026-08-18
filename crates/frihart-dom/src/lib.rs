//! Arena DOM over the HTML tree. Chrome does not mutate this.

#![forbid(unsafe_code)]

use frihart_html::Node;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct NodeId(pub u32);

#[derive(Clone, Debug)]
pub struct DomNode {
    pub id: NodeId,
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub text: Option<String>,
}

impl DomNode {
    pub fn is_text(&self) -> bool {
        self.text.is_some() && self.name.is_empty()
    }

    pub fn attr(&self, key: &str) -> Option<&str> {
        let key = key.to_ascii_lowercase();
        self.attrs
            .iter()
            .find(|(k, _)| k == &key)
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Clone, Debug)]
pub struct Document {
    nodes: Vec<DomNode>,
    root: NodeId,
}

impl Document {
    pub fn from_html(html: &str) -> Self {
        Self::from_tree(&frihart_html::parse(html))
    }

    pub fn from_tree(tree: &Node) -> Self {
        let mut doc = Self {
            nodes: Vec::new(),
            root: NodeId(0),
        };
        let root = doc.ingest(tree, None);
        doc.root = root;
        doc
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn get(&self, id: NodeId) -> Option<&DomNode> {
        self.nodes.get(id.0 as usize)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        self.get(id).map(|n| n.children.as_slice()).unwrap_or(&[])
    }

    pub fn title(&self) -> String {
        self.first_named(self.root, "title")
            .map(|id| self.text_content(id))
            .unwrap_or_default()
    }

    pub fn text_content(&self, id: NodeId) -> String {
        let Some(node) = self.get(id) else {
            return String::new();
        };
        if let Some(t) = &node.text {
            return collapse_ws(t);
        }
        let mut s = String::new();
        for child in &node.children {
            let part = self.text_content(*child);
            if part.is_empty() {
                continue;
            }
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(&part);
        }
        collapse_ws(&s)
    }

    pub fn serialize(&self) -> String {
        self.serialize_node(self.root)
    }

    fn first_named(&self, id: NodeId, name: &str) -> Option<NodeId> {
        let node = self.get(id)?;
        if node.name == name {
            return Some(id);
        }
        for child in &node.children {
            if let Some(found) = self.first_named(*child, name) {
                return Some(found);
            }
        }
        None
    }

    fn ingest(&mut self, tree: &Node, parent: Option<NodeId>) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(DomNode {
            id,
            name: tree.name.clone(),
            attrs: tree.attrs.clone(),
            parent,
            children: Vec::new(),
            text: tree.text.clone(),
        });
        let mut kids = Vec::new();
        for child in &tree.children {
            kids.push(self.ingest(child, Some(id)));
        }
        self.nodes[id.0 as usize].children = kids;
        id
    }

    fn serialize_node(&self, id: NodeId) -> String {
        let Some(node) = self.get(id) else {
            return String::new();
        };
        if let Some(t) = &node.text {
            return t.clone();
        }
        if node.name == "document" || node.name.is_empty() {
            return node
                .children
                .iter()
                .map(|c| self.serialize_node(*c))
                .collect();
        }
        let mut s = format!("<{}", node.name);
        for (k, v) in &node.attrs {
            s.push_str(&format!(" {k}=\"{v}\""));
        }
        s.push('>');
        for child in &node.children {
            s.push_str(&self.serialize_node(*child));
        }
        s.push_str(&format!("</{}>", node.name));
        s
    }
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_has_parent_links() {
        let d = Document::from_html("<title>X</title><p>y</p>");
        assert_eq!(d.title(), "X");
        assert!(d.len() >= 3);
        let root = d.get(d.root()).unwrap();
        assert!(root.parent.is_none());
        assert!(!d.children(d.root()).is_empty());
        assert!(d.serialize().contains("<p>"));
    }
}

//! A representation of a concrete, untyped syntax tree.

use super::kind::Kind;
use crate::source::Span;

/// A complete syntax tree. This is meant to be constructed using a [`Builder`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Tree {
    /// An arena containing all nodes in the tree contiguously.
    nodes: NodeArena,
    /// A reference to the tree's lone root node.
    root: Option<NodeKey>,
}

impl Tree {
    /// Construct an empty tree.
    const fn new() -> Self {
        Self { nodes: NodeArena::new(), root: None }
    }

    #[cfg(test)]
    fn children(&self, node: NodeKey) -> impl Iterator<Item = NodeKey> {
        let mut child = self.nodes[node].first;
        core::iter::from_fn(move || {
            let mut res = child?;
            child = self.nodes[res].next;
            Some(res)
        })
    }

    #[cfg(test)]
    pub(super) fn debug(
        &self,
        writer: &mut impl core::fmt::Write,
        input: &str,
    ) -> core::fmt::Result {
        let Some(root) = self.root else {
            return Ok(());
        };
        self.debug_helper(writer, input, root, 0)
    }

    #[cfg(test)]
    fn debug_helper(
        &self,
        writer: &mut impl core::fmt::Write,
        input: &str,
        node_key: NodeKey,
        indent: usize,
    ) -> core::fmt::Result {
        let node = self.nodes[node_key];
        write!(writer, "{: >indent$}{:?}@{}", "", node.kind, node.span)?;
        if node.kind.is_token() {
            write!(writer, " {:?}", &input[node.span])?;
        }
        writeln!(writer)?;
        for child in self.children(node_key) {
            self.debug_helper(writer, input, child, indent + 2)?;
        }
        Ok(())
    }
}

crate::structures::arena!(Node, pub(super) NodeArena, pub(super) NodeKey);

/// A node in a [`Tree`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Node {
    /// The type of syntax that this node represents.
    kind: Kind,
    /// The corresponding span within the original source.
    span: Span,
    /// The parent of the current node.
    parent: Option<NodeKey>,
    /// The previous sibling node.
    previous: Option<NodeKey>,
    /// The next sibling node.
    next: Option<NodeKey>,
    /// The first child node.
    first: Option<NodeKey>,
    /// The last child node.
    last: Option<NodeKey>,
}
const _: () = assert!(size_of::<Node>() == 32);

impl Node {
    /// The type of syntax that this node represents.
    #[cfg(test)]
    pub(super) fn kind(&self) -> Kind {
        self.kind
    }
}

/// A builder for a [`Tree`], using a stack of currently-open nodes.
#[derive(Debug)]
pub(super) struct Builder {
    /// The tree being built.
    tree: Tree,
    /// The parent of the current node.
    parent: Option<NodeKey>,
    /// The previous sibling of the current node.
    previous: Option<NodeKey>,
    /// The current location.
    cursor: u32,
}

impl Builder {
    /// Construct a new builder with an empty tree.
    pub(super) const fn new() -> Self {
        Self { tree: Tree::new(), parent: None, previous: None, cursor: 0 }
    }

    /// Finish building the tree.
    #[must_use]
    pub(super) fn build(self) -> Tree {
        if self.tree.root.is_none() {
            panic!("building tree with no root node")
        } else if self.parent.is_some() {
            panic!("building tree with unclosed nodes")
        }
        dbg!(&self.tree);
        self.tree
    }

    /// Insert a node at the current location.
    pub(super) fn insert(&mut self, kind: Kind, span: Span) -> NodeKey {
        let node = Node {
            kind,
            span,
            parent: self.parent,
            previous: self.previous,
            next: None,
            first: None,
            last: None,
        };
        let key = self.tree.nodes.alloc(node);

        if let Some(parent_key) = self.parent {
            let parent = &mut self.tree.nodes[parent_key];
            if parent.first.is_none() {
                parent.first = Some(key);
            }
            parent.last = Some(key);

            parent.span.end = self.cursor;
        } else if self.tree.root.is_none() {
            self.tree.root = Some(key);
        } else {
            dbg!(&self);
            panic!("building tree with multiple root nodes");
        }

        if let Some(previous_key) = self.previous {
            self.tree.nodes[previous_key].next = self.tree.nodes[previous_key].next.or(Some(key));
        }

        key
    }

    /// Begin a new node at the current location.
    pub(super) fn open(&mut self, kind: Kind) {
        let key = self.insert(kind, Span::new(self.cursor, self.cursor));
        self.parent = Some(key);
    }

    /// Close the current node.
    pub(super) fn close(&mut self) {
        let current = self.tree.nodes[self.parent.expect("close called with no open nodes")];
        self.previous = self.parent;
        if let Some(parent_key) = current.parent {
            let parent = &mut self.tree.nodes[parent_key];
            parent.span = parent.span.join(current.span);
            self.parent = Some(parent_key);
        } else {
            self.parent = None;
        }
        self.cursor = current.span.end;
    }

    /// Add a token to the current node.
    pub(super) fn token(&mut self, kind: Kind, length: u32) {
        let Some(parent) = self.parent else {
            panic!("adding token with no node");
        };
        let span = Span::new(self.cursor, self.cursor + length);
        self.cursor += length;
        self.tree.nodes[parent].span.end = span.end;
        let key = self.insert(kind, span);
        self.previous = Some(key);
    }
}

#[cfg(test)]
mod tests {
    use self::Event::*;
    use super::Builder;
    use crate::syntax::kind::Kind::*;

    #[derive(Debug, PartialEq, Eq)]
    enum Event {
        Next,
        Down,
        Up,
    }

    fn walk_tree(tree: &super::Tree) -> impl Iterator<Item = (Event, crate::syntax::kind::Kind)> {
        let mut current = tree.root;
        let mut event = Event::Next;
        core::iter::from_fn(move || {
            let current_node = tree.nodes[current?];
            if event == Event::Up {
                if let Some(next) = current_node.next {
                    current = Some(next);
                    event = Event::Next;
                    return Some((Event::Next, tree.nodes[next].kind));
                }
            } else if let Some(first) = current_node.first {
                current = Some(first);
                event = Event::Down;
                return Some((Event::Down, tree.nodes[first].kind));
            } else if let Some(next) = current_node.next {
                current = Some(next);
                event = Event::Next;
                return Some((Event::Next, tree.nodes[next].kind));
            }
            current = current_node.parent;
            event = Event::Up;
            Some((Event::Up, tree.nodes[current_node.parent?].kind))
        })
    }

    #[test]
    fn empty_tree() {
        let mut builder = Builder::new();
        builder.open(Root);
        builder.close();
        let tree = builder.build();
        assert_eq!(walk_tree(&tree).collect::<Vec<_>>(), []);
    }

    #[test]
    fn only_tokens() {
        let mut builder = Builder::new();
        builder.open(Root);
        builder.token(OpenParen, 1);
        builder.token(Ident, 5);
        builder.token(CloseParen, 1);
        builder.close();
        let tree = builder.build();
        assert_eq!(
            walk_tree(&tree).collect::<Vec<_>>(),
            [(Down, OpenParen), (Next, Ident), (Next, CloseParen), (Up, Root)]
        );
    }

    #[test]
    fn only_nodes() {
        let mut builder = Builder::new();
        builder.open(Root);
        builder.open(Fn);
        builder.open(ParamList);
        builder.close();
        builder.open(ParamList);
        builder.close();
        builder.close();
        builder.open(Fn);
        builder.close();
        builder.close();
        let tree = builder.build();
        assert_eq!(
            walk_tree(&tree).collect::<Vec<_>>(),
            [(Down, Fn), (Down, ParamList), (Next, ParamList), (Up, Fn), (Next, Fn), (Up, Root)]
        );
    }

    #[test]
    fn mix() {
        let mut builder = Builder::new();
        builder.open(Root);
        builder.open(Fn);
        builder.token(FnKw, 2);
        builder.token(Ident, 5);
        builder.open(ParamList);
        builder.token(OpenBrace, 1);
        builder.token(IntLiteral, 3);
        builder.token(CloseBrace, 1);
        builder.close();
        builder.token(Semi, 1);
        builder.close();
        builder.close();
        let tree = builder.build();
        assert_eq!(
            walk_tree(&tree).collect::<Vec<_>>(),
            [
                (Down, Fn),
                (Down, FnKw),
                (Next, Ident),
                (Next, ParamList),
                (Down, OpenBrace),
                (Next, IntLiteral),
                (Next, CloseBrace),
                (Up, ParamList),
                (Next, Semi),
                (Up, Fn),
                (Up, Root)
            ]
        );
    }

    #[test]
    #[should_panic = "building tree with no root node"]
    fn no_root_node() {
        let builder = Builder::new();
        let _ = builder.build();
    }

    #[test]
    #[should_panic = "building tree with multiple root nodes"]
    fn multiple_root_nodes() {
        let mut builder = Builder::new();
        builder.open(Root);
        builder.close();
        builder.open(Root);
        builder.close();
        let _ = builder.build();
    }

    #[test]
    #[should_panic = "building tree with unclosed nodes"]
    fn single_unclosed_node() {
        let mut builder = Builder::new();
        builder.open(Root);
        let _ = builder.build();
    }

    #[test]
    #[should_panic = "building tree with unclosed nodes"]
    fn multiple_unclosed_nodes() {
        let mut builder = Builder::new();
        builder.open(Root);
        builder.open(Fn);
        builder.close();
        builder.open(Fn);
        builder.open(ParamList);
        let _ = builder.build();
    }
}

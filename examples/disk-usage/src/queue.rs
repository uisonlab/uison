//! The Trash queue: what the user marked before pressing "Move to Trash".

use std::collections::HashSet;

use crate::tree::{NodeId, Tree};

/// Queue items don't overlap: a folder replaces its marked contents,
/// and anything inside a marked folder isn't added separately.
#[derive(Default)]
pub struct TrashQueue {
    /// In the order added.
    items: Vec<NodeId>,
    set: HashSet<NodeId>,
}

impl TrashQueue {
    pub fn items(&self) -> &[NodeId] {
        &self.items
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn contains(&self, node: NodeId) -> bool {
        self.set.contains(&node)
    }

    /// Whether `node` goes to the Trash with the queue: it or one of its folders is in it.
    pub fn covers(&self, tree: &Tree, node: NodeId) -> bool {
        if self.set.is_empty() {
            return false;
        }
        let mut cur = node;
        loop {
            if self.set.contains(&cur) {
                return true;
            }
            match tree.parent(cur) {
                Some(p) => cur = p,
                None => return false,
            }
        }
    }

    pub fn add(&mut self, tree: &Tree, node: NodeId) {
        if node == Tree::ROOT || self.covers(tree, node) {
            return;
        }
        self.items.retain(|&q| !tree.is_within(q, node));
        self.set.retain(|&q| !tree.is_within(q, node));
        self.items.push(node);
        self.set.insert(node);
    }

    pub fn remove(&mut self, node: NodeId) {
        if self.set.remove(&node) {
            self.items.retain(|&q| q != node);
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.set.clear();
    }

    pub fn size(&self, tree: &Tree) -> u64 {
        self.items.iter().map(|&n| tree.node(n).size).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_absorb_their_content() {
        let t = crate::tree::tests::sample();
        let photos = t.children(Tree::ROOT).nth(1).unwrap();
        let mut c = t.children(photos);
        let (a, b) = (c.next().unwrap(), c.next().unwrap());
        let big = t.children(Tree::ROOT).next().unwrap();

        let mut q = TrashQueue::default();
        q.add(&t, Tree::ROOT);
        assert!(q.is_empty());
        q.add(&t, a);
        q.add(&t, big);
        q.add(&t, a);
        assert_eq!(q.items(), [a, big]);
        q.add(&t, photos);
        assert_eq!(q.items(), [big, photos]);
        assert!(q.covers(&t, b) && !q.contains(b));
        assert_eq!(q.size(&t), 1500);
        q.remove(photos);
        assert_eq!(q.items(), [big]);
    }
}

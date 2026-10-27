//! The scanned folder's tree in a single array.
//!
//! Nodes refer to each other by `u32` indices. A node's children are contiguous and sorted
//! by size (largest first), so the chart and the list walk them directly.

use std::borrow::Cow;
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::ops::Range;
use std::path::{Path, PathBuf};

pub type NodeId = u32;
pub const NO_NODE: NodeId = u32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    File,
    Dir,
}

#[derive(Debug)]
pub struct Node {
    pub name: Box<OsStr>,
    /// Bytes on disk, including all contents.
    pub size: u64,
    /// Files in the subtree (1 for a file).
    pub files: u32,
    pub parent: NodeId,
    first_child: NodeId,
    child_count: u32,
    pub kind: Kind,
    /// Moved to the Trash after the scan.
    pub removed: bool,
}

impl Node {
    pub fn is_dir(&self) -> bool {
        self.kind == Kind::Dir
    }

    pub fn name_lossy(&self) -> Cow<'_, str> {
        self.name.to_string_lossy()
    }
}

/// The scanner's intermediate tree, before it's packed into the array.
#[derive(Debug)]
pub struct ScanNode {
    pub name: Box<OsStr>,
    pub size: u64,
    pub files: u32,
    pub kind: Kind,
    pub children: Vec<ScanNode>,
}

impl ScanNode {
    /// Sum up the children's sizes and file counts and sort them, largest first.
    /// `size` must already hold the node's own size.
    pub fn finish(&mut self) {
        for child in &self.children {
            self.size += child.size;
            self.files = self.files.saturating_add(child.files);
        }
        self.children.sort_unstable_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
        self.children.shrink_to_fit();
    }
}

#[derive(Debug)]
pub struct Tree {
    root_path: PathBuf,
    nodes: Vec<Node>,
}

impl Tree {
    pub const ROOT: NodeId = 0;

    /// Pack the tree into the array breadth-first: siblings stay contiguous.
    pub fn from_scan(root_path: PathBuf, root: ScanNode) -> Self {
        fn count(n: &ScanNode) -> usize {
            1 + n.children.iter().map(count).sum::<usize>()
        }
        // All at once: growing an array of millions of nodes would double the peak memory.
        let mut nodes = Vec::with_capacity(count(&root));
        let mut queue: VecDeque<(NodeId, Vec<ScanNode>)> = VecDeque::new();

        let ScanNode { name, size, files, kind, children } = root;
        nodes.push(Node { name, size, files, parent: NO_NODE, first_child: NO_NODE, child_count: 0, kind, removed: false });
        if !children.is_empty() {
            queue.push_back((Self::ROOT, children));
        }
        while let Some((parent, children)) = queue.pop_front() {
            let first = nodes.len() as NodeId;
            nodes[parent as usize].first_child = first;
            nodes[parent as usize].child_count = children.len() as u32;
            for child in children {
                let ScanNode { name, size, files, kind, children } = child;
                let id = nodes.len() as NodeId;
                nodes.push(Node { name, size, files, parent, first_child: NO_NODE, child_count: 0, kind, removed: false });
                if !children.is_empty() {
                    queue.push_back((id, children));
                }
            }
        }
        Self { root_path, nodes }
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id as usize]
    }

    fn child_range(&self, id: NodeId) -> Range<NodeId> {
        let n = self.node(id);
        if n.child_count == 0 { 0..0 } else { n.first_child..n.first_child + n.child_count }
    }

    /// Remaining children, largest first.
    pub fn children(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.child_range(id).filter(|&c| !self.node(c).removed)
    }

    /// Remaining children by size. Usually already in order; after the Trash a folder may have become
    /// smaller than its neighbors, then re-sort.
    pub fn sorted_children(&self, id: NodeId) -> Vec<NodeId> {
        let mut children: Vec<NodeId> = self.children(id).collect();
        let size = |c: &NodeId| self.node(*c).size;
        if !children.is_sorted_by(|a, b| size(a) >= size(b)) {
            children.sort_by_key(|c| std::cmp::Reverse(size(c)));
        }
        children
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        let p = self.node(id).parent;
        (p != NO_NODE).then_some(p)
    }

    /// Chain from the root to `id` inclusive.
    pub fn ancestry(&self, id: NodeId) -> Vec<NodeId> {
        let mut chain = vec![id];
        let mut cur = id;
        while let Some(p) = self.parent(cur) {
            chain.push(p);
            cur = p;
        }
        chain.reverse();
        chain
    }

    /// `ancestor` is `id` itself or one of its ancestors.
    pub fn is_within(&self, id: NodeId, ancestor: NodeId) -> bool {
        let mut cur = id;
        loop {
            if cur == ancestor {
                return true;
            }
            match self.parent(cur) {
                Some(p) => cur = p,
                None => return false,
            }
        }
    }

    pub fn path(&self, id: NodeId) -> PathBuf {
        let mut path = self.root_path.clone();
        for &n in self.ancestry(id).iter().skip(1) {
            path.push(&*self.node(n).name);
        }
        path
    }

    /// Name to show; the root shows its full path.
    pub fn display_name(&self, id: NodeId) -> Cow<'_, str> {
        if id == Self::ROOT { self.root_path.to_string_lossy() } else { self.node(id).name_lossy() }
    }

    /// Name without the path: for the root, the last component ("/" stays "/").
    pub fn short_name(&self, id: NodeId) -> Cow<'_, str> {
        match (id, self.root_path.file_name()) {
            (Self::ROOT, Some(name)) => name.to_string_lossy(),
            _ => self.display_name(id),
        }
    }

    /// Mark the subtree deleted and subtract its size from all ancestors.
    pub fn remove(&mut self, id: NodeId) {
        if id == Self::ROOT || self.node(id).removed {
            return;
        }
        let (size, files) = {
            let n = &mut self.nodes[id as usize];
            n.removed = true;
            (n.size, n.files)
        };
        let mut cur = id;
        while let Some(p) = self.parent(cur) {
            let n = &mut self.nodes[p as usize];
            n.size = n.size.saturating_sub(size);
            n.files = n.files.saturating_sub(files);
            cur = p;
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn file(name: &str, size: u64) -> ScanNode {
        ScanNode { name: OsStr::new(name).into(), size, files: 1, kind: Kind::File, children: vec![] }
    }

    pub fn dir(name: &str, children: Vec<ScanNode>) -> ScanNode {
        let mut n = ScanNode { name: OsStr::new(name).into(), size: 0, files: 0, kind: Kind::Dir, children };
        n.finish();
        n
    }

    pub fn sample() -> Tree {
        let root = dir(
            "/data",
            vec![
                file("small.txt", 10),
                dir("photos", vec![file("a.jpg", 300), file("b.jpg", 200)]),
                dir("empty", vec![]),
                file("big.iso", 1000),
            ],
        );
        Tree::from_scan(PathBuf::from("/data"), root)
    }

    fn names(t: &Tree, id: NodeId) -> Vec<String> {
        t.children(id).map(|c| t.node(c).name_lossy().into_owned()).collect()
    }

    #[test]
    fn aggregates_and_sorts() {
        let t = sample();
        assert_eq!(t.node(Tree::ROOT).size, 1510);
        assert_eq!(t.node(Tree::ROOT).files, 4);
        assert_eq!(names(&t, Tree::ROOT), ["big.iso", "photos", "small.txt", "empty"]);
        let photos = t.children(Tree::ROOT).nth(1).unwrap();
        assert_eq!(names(&t, photos), ["a.jpg", "b.jpg"]);
    }

    #[test]
    fn paths_and_ancestry() {
        let t = sample();
        let photos = t.children(Tree::ROOT).nth(1).unwrap();
        let a = t.children(photos).next().unwrap();
        assert_eq!(t.path(a), PathBuf::from("/data/photos/a.jpg"));
        assert_eq!(t.ancestry(a), vec![Tree::ROOT, photos, a]);
        assert!(t.is_within(a, photos) && !t.is_within(photos, a));
        assert_eq!(t.short_name(Tree::ROOT), "data");
    }

    #[test]
    fn remove_updates_ancestors() {
        let mut t = sample();
        let photos = t.children(Tree::ROOT).nth(1).unwrap();
        let a = t.children(photos).next().unwrap();
        t.remove(a);
        assert_eq!(t.node(photos).size, 200);
        assert_eq!(t.node(Tree::ROOT).size, 1210);
        assert_eq!(t.node(Tree::ROOT).files, 3);
        t.remove(a);
        assert_eq!(t.node(Tree::ROOT).size, 1210);
        assert_eq!(t.sorted_children(Tree::ROOT)[1], photos);
    }
}

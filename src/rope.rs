#![allow(dead_code)]
// a rope data structure built using b+ trees. each node contains the number of bytes as well as chars and lines.

// the b+ tree is a balanced tree with a maximum of 2 * t children and a minimum of t children.
// all leaves have the same depth.
// all keys in a node are sorted in increasing order.
// all children of a node contain keys that are less than the key in the parent node.

use bytecount::num_chars;
use smallvec::{smallvec, SmallVec};
use std::ops::{Add, AddAssign};

#[derive(Debug)]
struct Rope {
    root: Node,
}

#[derive(Debug)]
enum Node {
    Internal(Internal),
    Leaf(Leaf),
}

impl Node {
    fn metrics(&self) -> Metrics {
        match self {
            Node::Internal(node) => node
                .metrics
                .iter()
                .fold(Metrics::default(), |acc, x| acc + *x),
            Node::Leaf(node) => node.data.iter().fold(Metrics::default(), |acc, x| acc + *x),
        }
    }
}

#[derive(Debug, Default)]
struct Internal {
    metrics: SmallVec<[Metrics; MAX]>,
    children: SmallVec<[Box<Node>; MAX]>,
}

#[derive(Debug, Default)]
struct Leaf {
    data: SmallVec<[Metrics; MAX]>,
}

impl Leaf {
    fn new(metric: Metrics) -> Self {
        let mut data = SmallVec::new();
        data.push(metric);
        Self { data }
    }
}

#[derive(Debug, Default, Copy, Clone)]
struct Metrics {
    bytes: usize,
    chars: usize,
}

impl Metrics {
    fn new(bytes: usize, chars: usize) -> Self {
        Self { bytes, chars }
    }
}

impl Add for Metrics {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            bytes: self.bytes + rhs.bytes,
            chars: self.chars + rhs.chars,
        }
    }
}

impl AddAssign for Metrics {
    fn add_assign(&mut self, rhs: Self) {
        self.bytes += rhs.bytes;
        self.chars += rhs.chars;
    }
}

const MAX: usize = 2;
const MAX_1: usize = MAX - 1;
const NODE_SIZE: usize = 5;

impl Rope {
    fn new(string: &[u8], gap_start: usize, gap_end: usize) -> Self {
        // first we need to build a vec of leaves. We do this by splitting the
        // string into chunks, and the creating the metrics for each chunk. This
        // is done by counting the number of bytes, chars and lines in each
        // chunk. We then create a leaf for each chunk and add it to the vec.
        let mut nodes = Vec::new();
        fill_leaves(&string[..gap_start], &mut nodes);
        let gap = Metrics::new(gap_end - gap_start, 0);
        nodes.push(Node::Leaf(Leaf::new(gap)));
        fill_leaves(&string[gap_end..], &mut nodes);

        // At this point we have a vec of leaves. we need to build a b+ tree
        // from this. We do this by building layers of the tree one at a time
        // and then swapping the vecs to leapfrog up to the next layer.
        let mut output = Vec::new();
        loop {
            fill_rope_layer(&mut output, &mut nodes);
            if output.len() == 1 {
                let root = output.pop().unwrap();
                return Self { root };
            }
            std::mem::swap(&mut output, &mut nodes);
        }
    }
}

fn fill_rope_layer(to: &mut Vec<Node>, from: &mut Vec<Node>) {
    let mut iter = from.drain(..);
    loop {
        let mut children = SmallVec::new();
        fill_vec(&mut children, &mut iter);
        if children.is_empty() {
            break;
        }
        let metrics = children
            .iter()
            .map(|x| x.metrics())
            .collect::<SmallVec<_>>();
        let node = Node::Internal(Internal { metrics, children });
        to.push(node);
    }
}

// function that takes a vec and a iterator and fills the vec with MAX elements from the front of the iterator
fn fill_vec(vec: &mut SmallVec<[Box<Node>; MAX]>, iter: &mut impl Iterator<Item = Node>) {
    while vec.len() < MAX {
        if let Some(x) = iter.next() {
            vec.push(Box::new(x));
        } else {
            break;
        }
    }
}

fn fill_leaves(string: &[u8], chunks: &mut Vec<Node>) {
    let mut start = 0;
    while start < string.len() {
        let mut end = start + NODE_SIZE;
        if end > string.len() {
            end = string.len();
        }
        // align to a char boundary
        while !is_char_boundary(string.get(end).unwrap_or(&0)) {
            end += 1;
        }
        let data = Metrics::new(end - start, num_chars(&string[start..end]));
        start = end;
        if let Some(Node::Leaf(leaf)) = chunks.last_mut() {
            if leaf.data.len() < MAX {
                leaf.data.push(data);
                continue;
            }
        }
        chunks.push(Node::Leaf(Leaf::new(data)));
    }
}

const fn is_char_boundary(byte: &u8) -> bool {
    // This is bit magic equivalent to: b < 128 || b >= 192
    (*byte as i8) >= -0x40
}

impl Node {
    fn leaf_search(&self, k: usize) -> &Leaf {
        match self {
            Node::Internal(node) => {
                let children = node.children.as_slice();
                let metrics = node.metrics.as_slice();
                assert_eq!(children.len(), metrics.len() + 1);
                let m = children.len();
                for i in 0..(m - 1) {
                    if k <= metrics[i].bytes {
                        return children[i].leaf_search(k);
                    }
                }
                children[m].leaf_search(k)
            }
            Node::Leaf(leaf) => leaf,
        }
    }

    fn insert(&mut self, k: usize, v: Metrics) {
        match self {
            Node::Leaf(leaf) => {
                if leaf.data.len() < MAX {
                    let mut k = k;
                    for leaf in leaf.data.as_mut() {
                        println!("k: {:?}", k);
                        println!("leaf.bytes: {:?}", leaf.bytes);
                        if k <= leaf.bytes {
                            *leaf += v;
                            return;
                        }
                        k -= leaf.bytes;
                    }
                    panic!("index was out of bounds");
                } else {
                    let mut new = Leaf::default();
                    new.data.push(v);
                    todo!("split leaf");
                }
            }
            Node::Internal(node) => {
                let children = node.children.as_mut();
                let metrics = node.metrics.as_slice();
                assert_eq!(children.len(), metrics.len() + 1);
                let m = children.len();
                for i in 0..(m - 1) {
                    if k <= metrics[i].bytes {
                        children[i].insert(k, v);
                        todo!("update metrics");
                    }
                }
                children[m].insert(k, v);
            }
        }
    }
}

#[cfg(test)]
mod test {
    // use super::*;

    // #[test]
    // fn test_new() {
    //     let mut rope = Rope::new();
    //     println!("{:?}", rope);
    //     rope.root.insert(0, Metrics::default());
    //     rope.root.insert(0, Metrics::default());
    //     rope.root.insert(0, Metrics::default());
    // }
}

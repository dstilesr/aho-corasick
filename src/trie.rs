pub mod search;
use std::collections::VecDeque;

use super::follow_links;
pub use search::*;

#[derive(Debug, PartialEq, Eq)]
pub enum SearchError {
    InvalidNodeId(usize),
    DuplicateNode,
    InvalidDictionary,
}

pub type SearchResult<T> = Result<T, SearchError>;
pub type NodeId = usize;

/// A link between two nodes in the prefix tree
#[derive(Debug)]
pub struct Link(char, NodeId);

/// Represents a node in the prefix tree for the Aho-Corasick structure
#[derive(Debug)]
pub enum Node {
    /// Dictionary nodes represent a complete pattern. When these nodes are reached, a match has been found.
    DictNode {
        value: String,
        nxt: Vec<Link>,
        adj: Option<Link>,
    },

    /// MedNodes represent intermediate nodes / incomplete pattern matches
    MedNode { nxt: Vec<Link>, adj: Option<Link> },
}

impl Node {
    /// Instantiate a new node to add to the prefix tree. If a value is provided, a DictNode will
    /// be instantiated with that value. Otherwise, a MedNode will be created.
    pub fn new(value: Option<String>) -> Self {
        match value {
            None => Self::MedNode {
                nxt: Vec::new(),
                adj: None,
            },
            Some(s) => Self::DictNode {
                value: s,
                nxt: Vec::new(),
                adj: None,
            },
        }
    }

    /// Add a link to the node. If `adjacent` is true, adds the link to the adjacent
    /// links list. Otherwise, it is added to the next links list.
    fn add_link(&mut self, link: Link, adjacent: bool) {
        match self {
            Node::DictNode { value: _, nxt, adj } => {
                if adjacent {
                    adj.replace(link);
                } else {
                    nxt.push(link);
                }
            }
            Node::MedNode { nxt, adj } => {
                if adjacent {
                    adj.replace(link);
                } else {
                    nxt.push(link);
                }
            }
        }
    }

    /// Get the vector of following nodes
    pub fn next_nodes(&self) -> &Vec<Link> {
        match self {
            Node::DictNode {
                value: _,
                nxt,
                adj: _,
            } => nxt,
            Node::MedNode { nxt, adj: _ } => nxt,
        }
    }

    /// Get adjacent (failure) link of this node
    pub fn adj_node(&self) -> Option<&Link> {
        let out = match self {
            Node::DictNode {
                value: _,
                nxt: _,
                adj,
            } => adj,
            Node::MedNode { nxt: _, adj } => adj,
        };
        out.as_ref()
    }
}

/// Represents the root of the Aho-Corasick prefix tree
pub struct TrieRoot {
    nodes: Vec<Node>,
}

impl TrieRoot {
    /// Instantiate a new, empty prefix tree
    pub fn new() -> Self {
        Self {
            // Add root node
            nodes: vec![Node::new(None)],
        }
    }

    /// Get a node by its ID number. Returns error if the ID is out of bounds.
    pub fn get_node(&self, node_id: NodeId) -> SearchResult<&Node> {
        if node_id >= self.nodes.len() {
            Err(SearchError::InvalidNodeId(node_id))
        } else {
            Ok(&self.nodes[node_id])
        }
    }

    /// Get the ID of the root node of the tree
    pub fn root_node_id(&self) -> usize {
        0
    }

    /// Get the root node of the tree
    pub fn root_node(&self) -> &Node {
        &self.nodes[0]
    }

    /// Get the total number of nodes in the prefix tree
    pub fn total_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Add a new node to the tree and return its Id
    fn add_node(&mut self, node: Node) -> NodeId {
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    /// Add a link from one node in the tree to another
    fn add_link(&mut self, from: NodeId, to: NodeId, c: char, is_adj: bool) -> SearchResult<()> {
        if to >= self.nodes.len() {
            return Err(SearchError::InvalidNodeId(to));
        }
        if from >= self.nodes.len() {
            return Err(SearchError::InvalidNodeId(from));
        }
        let lnk = Link(c, to);
        let from_node = &mut self.nodes[from];
        from_node.add_link(lnk, is_adj);
        Ok(())
    }

    /// Add a new pattern / string to the prefix tree.
    ///
    /// Add the nodes corresponding to a new string to the prefix tree along with
    /// their corresponding "following" links. Adjacent or "failure" links must be added
    /// separately by calling the "compute_failure_links" function. This is only meant to
    /// be used during creation of the trie structure.
    fn add_pattern(&mut self, new_item: String) -> SearchResult<()> {
        let mut current_id = self.root_node_id();
        let characters: Vec<char> = new_item.chars().collect();

        for (i, c) in characters.iter().enumerate() {
            let next_nodes = match self.get_node(current_id)? {
                Node::DictNode {
                    value: _,
                    nxt,
                    adj: _,
                } => nxt,
                Node::MedNode { nxt, adj: _ } => nxt,
            };
            match follow_links!(next_nodes, *c).next() {
                Some(nid) => current_id = nid,
                None => {
                    // Next node not already present - add it to the trie
                    let val = if i == characters.len() - 1 {
                        Some(new_item.clone())
                    } else {
                        None
                    };
                    let node_id = self.add_node(Node::new(val));
                    self.add_link(current_id, node_id, *c, false)?;

                    current_id = node_id;
                }
            }
        }
        Ok(())
    }

    /// Find the non-redundant failure link target for a node.
    ///
    /// This follows the parent's failure chain to find the longest proper suffix
    /// that exists in the trie, can be reached via the given edge character, AND
    /// would not already be in the active set through parent's adj link.
    fn find_failure_target(
        &self,
        parent_id: NodeId,
        edge_char: char,
        parent_adj_id: Option<NodeId>,
    ) -> Option<NodeId> {
        // Children of root fail to root (not stored)
        if parent_id == self.root_node_id() {
            return None;
        }

        let parent_node = self.get_node(parent_id).unwrap();
        let mut current = parent_node.adj_node();

        // Follow failure links from parent until we find one with outgoing edge_char
        while let Some(Link(_, target_id)) = current {
            let current_node = self.get_node(*target_id).unwrap();

            // Try to follow edge_char from this node
            if let Some(nid) = follow_links!(current_node.next_nodes(), edge_char).next() {
                // Check if this target would be redundant
                if let Some(parent_adj) = parent_adj_id {
                    let parent_adj_node = self.get_node(parent_adj).unwrap();
                    let is_redundant = follow_links!(parent_adj_node.next_nodes(), edge_char)
                        .any(|adj_nid| adj_nid == nid);

                    if !is_redundant {
                        // Found non-redundant suffix
                        return Some(nid);
                    }
                    // Otherwise continue searching for shorter suffix
                } else {
                    // Parent has no adj, so this is not redundant
                    return Some(nid);
                }
            }

            // Not found, follow this node's failure link
            current = current_node.adj_node();
        }

        // Last resort: check if root has this edge
        if let Some(nid) = follow_links!(self.root_node().next_nodes(), edge_char).next() {
            // Check redundancy with parent's adj
            if let Some(parent_adj) = parent_adj_id {
                let parent_adj_node = self.get_node(parent_adj).unwrap();
                let is_redundant = follow_links!(parent_adj_node.next_nodes(), edge_char)
                    .any(|adj_nid| adj_nid == nid);

                if !is_redundant {
                    return Some(nid);
                }
            } else {
                return Some(nid);
            }
        }

        None
    }

    /// Compute the failure / adjacent links for the prefix tree.
    ///
    /// This will add only the "search suffix links". These are the links that will actually
    /// be followed during search. This should only be called during initialization after inserting
    /// the patterns with their respective "following" links.
    fn compute_failure_links(&mut self) -> SearchResult<()> {
        // Initialize queue with (parent_id, child_id, edge_char) tuples
        let mut queue = VecDeque::with_capacity(self.total_nodes());
        for link in self.root_node().next_nodes() {
            let Link(c, node_id) = link;
            queue.push_back((self.root_node_id(), *node_id, *c));
        }

        // Process each node in BFS order
        while let Some((parent_id, current_id, edge_char)) = queue.pop_front() {
            // Get parent's adj link if it exists
            let parent_node = self.get_node(parent_id)?;
            let parent_adj_id = parent_node.adj_node().map(|Link(_, nid)| *nid);

            // Find failure link target for current node (non-redundant)
            let failure_target = self.find_failure_target(parent_id, edge_char, parent_adj_id);

            // Add adj link if one was found
            if let Some(target_id) = failure_target {
                self.add_link(current_id, target_id, edge_char, true)?;
            }

            // Add children of current node to queue
            let current_node = self.get_node(current_id)?;
            for link in current_node.next_nodes() {
                let Link(c, child_id) = link;
                queue.push_back((current_id, *child_id, *c));
            }
        }

        Ok(())
    }

    /// Get the node on the prefix tree that lies at the end of the given path.
    ///
    /// The path is given by traversing the tree following the characters of the given string.
    /// If there is no node at that path, return None.
    pub fn node_by_path(&self, path: &str) -> Option<NodeId> {
        if path.is_empty() {
            return None;
        }

        let mut current = self.root_node_id();
        for c in path.chars() {
            let curr_node = self.get_node(current).unwrap();
            if let Some(nid) = follow_links!(curr_node.next_nodes(), c).next() {
                current = nid;
            } else {
                return None;
            }
        }
        Some(current)
    }
}

/// Instantiate a prefix tree for search from the given dictionary (list of strings). Returns
/// an error if the dictionary is empty or contains empty strings or duplicates.
///
/// Examples
/// ```rust
/// use aho_corasick::trie;
///
/// let my_dictionary = vec![String::from("abc"), String::from("ab"), String::from("cd")];
/// let prefix_tree = trie::create_prefix_tree(my_dictionary).unwrap();
/// ```
pub fn create_prefix_tree(mut dictionary: Vec<String>) -> SearchResult<TrieRoot> {
    if dictionary.is_empty() {
        return Err(SearchError::InvalidDictionary);
    }

    dictionary.sort();

    // Validate dictionary
    for (item, next) in dictionary.iter().zip(&dictionary[1..]) {
        if item == next {
            return Err(SearchError::DuplicateNode);
        } else if item == "" || next == "" {
            return Err(SearchError::InvalidDictionary);
        }
    }

    let mut pt = TrieRoot::new();
    for item in dictionary {
        pt.add_pattern(item).unwrap();
    }
    pt.compute_failure_links()?;
    Ok(pt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        let dictionary = vec![String::from("ab"), String::from("abc"), String::from("cd")];
        let pt = create_prefix_tree(dictionary).unwrap();

        // Verify root node properties
        assert!(pt.root_node().adj_node().is_none());
        assert_eq!(pt.root_node().next_nodes().len(), 2);

        let mut root_chars: Vec<char> = pt
            .root_node()
            .next_nodes()
            .iter()
            .map(|Link(c, _)| *c)
            .collect();

        root_chars.sort();
        assert_eq!(root_chars[0], 'a');
        assert_eq!(root_chars[1], 'c');

        // Total nodes
        assert_eq!(pt.total_nodes(), pt.nodes.len());
        assert_eq!(pt.total_nodes(), 6);

        // Count dictionary nodes
        let mut dct_vals = Vec::new();
        for node in pt.nodes {
            if let Node::DictNode {
                value,
                nxt: _,
                adj: _,
            } = node
            {
                dct_vals.push(value.clone());
            }
        }
        dct_vals.sort();
        assert_eq!(dct_vals.len(), 3);

        assert_eq!(&dct_vals[0], "ab");
        assert_eq!(&dct_vals[1], "abc");
        assert_eq!(&dct_vals[2], "cd");
    }

    #[test]
    fn test_node_by_path() {
        let pt = create_prefix_tree(vec![
            String::from("ab"),
            String::from("abc"),
            String::from("bcd"),
            String::from("cd"),
            String::from("cb"),
        ])
        .unwrap();

        // Check 'ab' node
        let ab_node = pt.get_node(pt.node_by_path("ab").unwrap()).unwrap();
        let ab_nxt = match ab_node {
            Node::MedNode { nxt: _, adj: _ } => panic!("Expected a dictionary node"),
            Node::DictNode { value, nxt, adj: _ } => {
                assert_eq!("ab", value);
                nxt
            }
        };
        assert_eq!(ab_nxt.len(), 1);
        let Link(c, _) = ab_nxt[0];
        assert_eq!(c, 'c');

        // Check 'c' node
        let c_node = pt.get_node(pt.node_by_path("c").unwrap()).unwrap();
        let c_nxt = match c_node {
            Node::MedNode { nxt, adj: _ } => nxt,
            Node::DictNode {
                value: _,
                nxt: _,
                adj: _,
            } => panic!("Expected intermediate node"),
        };
        assert_eq!(c_nxt.len(), 2);
        let mut chars: Vec<char> = c_nxt.iter().map(|Link(c, _)| *c).collect();
        chars.sort();
        assert_eq!(chars, ['b', 'd']);

        // Nonexistent nodes
        if let Some(_) = dbg!(pt.node_by_path("cdb")) {
            panic!("Did not expect to find node!")
        }
        if let Some(_) = dbg!(pt.node_by_path("xyz")) {
            panic!("Did not expect to find node!")
        }
        if let Some(_) = dbg!(pt.node_by_path("abd")) {
            panic!("Did not expect to find node!")
        }
        if let Some(_) = dbg!(pt.node_by_path("")) {
            panic!("Did not expect to find node!")
        }
    }

    #[test]
    fn test_adj_links() {
        let pt = create_prefix_tree(vec![
            String::from("ab"),
            String::from("abc"),
            String::from("bcd"),
            String::from("cd"),
        ])
        .unwrap();

        assert_eq!(pt.root_node().next_nodes().len(), 3);
        let ab_node = pt.node_by_path("ab").unwrap();
        let b_node = pt.node_by_path("b").unwrap();
        let c_node = pt.node_by_path("c").unwrap();
        let bc_node = pt.node_by_path("bc").unwrap();
        let abc_node = pt.node_by_path("abc").unwrap();
        let bcd_node = pt.node_by_path("bcd").unwrap();

        // bc -> c
        if let Some(Link(_, nid)) = pt.get_node(bc_node).unwrap().adj_node() {
            assert_eq!(*nid, c_node);
        } else {
            panic!("Expected an adjacent node!")
        }

        // bcd -> none
        if let Some(_) = pt.get_node(bcd_node).unwrap().adj_node() {
            panic!("Did not expect an adjacent node for 'bcd'");
        }

        // abc -> c
        if let Some(Link(_, nid)) = pt.get_node(abc_node).unwrap().adj_node() {
            assert_eq!(*nid, c_node);
        } else {
            panic!("Expected an adjacent node!")
        }

        // ab -> b
        if let Some(Link(_, nid)) = pt.get_node(ab_node).unwrap().adj_node() {
            assert_eq!(*nid, b_node);
        } else {
            panic!("Expected an adjacent node!")
        }
    }

    #[test]
    fn test_adj_links_medium() {
        let pt = create_prefix_tree(vec![
            String::from("a"),
            String::from("ab"),
            String::from("bab"),
            String::from("bca"),
            String::from("caa"),
            String::from("bc"),
        ])
        .unwrap();

        let a_node = pt.node_by_path("a").unwrap();
        let b_node = pt.node_by_path("b").unwrap();
        let c_node = pt.node_by_path("c").unwrap();

        let ab_node = pt.node_by_path("ab").unwrap();
        let ba_node = pt.node_by_path("ba").unwrap();
        let bc_node = pt.node_by_path("bc").unwrap();
        let ca_node = pt.node_by_path("ca").unwrap();
        let caa_node = pt.node_by_path("caa").unwrap();
        let bca_node = pt.node_by_path("bca").unwrap();
        let bab_node = pt.node_by_path("bab").unwrap();

        // ba -> a
        if let Some(Link(_, nid)) = pt.get_node(ba_node).unwrap().adj_node() {
            assert_eq!(*nid, a_node);
        } else {
            panic!("Expected an adjacent node!")
        }

        // ca -> a
        if let Some(Link(_, nid)) = pt.get_node(ca_node).unwrap().adj_node() {
            assert_eq!(*nid, a_node);
        } else {
            panic!("Expected an adjacent node!")
        }

        // bc -> c
        if let Some(Link(_, nid)) = pt.get_node(bc_node).unwrap().adj_node() {
            assert_eq!(*nid, c_node);
        } else {
            panic!("Expected an adjacent node!")
        }

        // ab -> b
        if let Some(Link(_, nid)) = pt.get_node(ab_node).unwrap().adj_node() {
            assert_eq!(*nid, b_node);
        } else {
            panic!("Expected an adjacent node!")
        }

        // caa -> a
        if let Some(Link(_, nid)) = pt.get_node(caa_node).unwrap().adj_node() {
            assert_eq!(*nid, a_node);
        } else {
            panic!("Expected an adjacent node!")
        }

        // bca -> a
        if let Some(Link(_, nid)) = pt.get_node(bca_node).unwrap().adj_node() {
            assert_eq!(*nid, a_node);
        } else {
            panic!("Expected an adjacent node!")
        }

        // bab -> none
        if let Some(_) = dbg!(pt.get_node(bab_node).unwrap().adj_node()) {
            panic!("Expected no adjacent node for 'bab'!");
        }
    }

    #[test]
    #[should_panic]
    fn test_initialization_empty_str() {
        let res = create_prefix_tree(vec![String::from("abc"), String::from("")]);
        res.unwrap();
    }

    #[test]
    #[should_panic]
    fn test_initialization_empty_dct() {
        let res = create_prefix_tree(Vec::new());
        res.unwrap();
    }

    #[test]
    #[should_panic]
    fn test_initialization_duplicate() {
        let res = create_prefix_tree(vec![
            String::from("abc"),
            String::from("xy"),
            String::from("abc"),
            String::from("opq"),
        ]);
        res.unwrap();
    }
}

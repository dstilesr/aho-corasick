use std::collections::VecDeque;
use unicode_normalization::UnicodeNormalization;
pub mod ring_buffer;
pub use ring_buffer::RingBuffer;
pub mod search;
pub use search::*;

/// Type alias to reference the ID of a node in the prefix tree.
pub type NodeId = usize;

/// Errors that can be raised by the library functions
#[derive(Debug, PartialEq, Eq)]
pub enum SearchError {
    InvalidNodeId(NodeId),
    DuplicateNode,
    InvalidDictionary,
    MissingLink(NodeId),
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str_val = match self {
            Self::InvalidNodeId(id) => format!("Invalid node ID: {}", id),
            Self::DuplicateNode => "Duplicate node".to_string(),
            Self::InvalidDictionary => "Invalid dictionary".to_string(),
            Self::MissingLink(id) => format!("Missing link for node ID: {}", id),
        };
        write!(f, "{}", str_val)
    }
}

/// Result type for this library
pub type SearchResult<T> = Result<T, SearchError>;

/// A link between two nodes in the prefix tree
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Link(char, NodeId);

impl Link {
    /// Get the character element of the link
    #[inline]
    fn get_char(&self) -> char {
        self.0
    }

    /// Get the Node ID element of the link
    #[inline]
    fn get_node_id(&self) -> NodeId {
        self.1
    }
}

/// Options to use when performing searches
#[derive(Debug)]
pub struct SearchOptions {
    /// Whether to distinguish uppercase and lowercase characters.
    pub case_sensitive: bool,

    /// Whether to return only matches that begin and end with word boundaries.
    pub check_bounds: bool,
}

impl Default for SearchOptions {
    /// Default oprions: case sensitive search without checking word boundaries.
    fn default() -> Self {
        SearchOptions {
            case_sensitive: true,
            check_bounds: false,
        }
    }
}

/// Represents a node in the prefix tree for the Aho-Corasick structure
#[derive(Debug)]
pub struct Node {
    value: Option<String>,
    keyword: Option<String>,
    nxt: Vec<Link>,
    fail_to: Option<NodeId>,
    dct_to: Option<NodeId>,
    pattern_len: usize,
}

impl Default for Node {
    /// Default instantiation - no value, keyword, empty links
    fn default() -> Self {
        Self {
            value: None,
            keyword: None,
            nxt: Vec::new(),
            fail_to: None,
            dct_to: None,
            pattern_len: 0,
        }
    }
}

impl Node {
    /// Instantiate a new node to add to the prefix tree. If a value is provided, a DictNode will
    /// be instantiated with that value. Otherwise, a MedNode will be created.
    ///
    /// Example
    /// ```rust
    /// use ac_search_rs::trie::Node;
    ///
    /// let node_1 = Node::new(Some(String::from("variant")), Some(String::from("Standard Variant")));
    ///
    /// // Keyword equal to the value to match
    /// let node_2 = Node::new(Some(String::from("pattern")), None);
    /// ```
    pub fn new(value: Option<String>, keyword: Option<String>) -> Self {
        match value {
            None => Self::default(),
            Some(s) => {
                let total_chars = s.chars().count();
                Self {
                    keyword: Some(keyword.unwrap_or_else(|| s.clone())),
                    value: Some(s),
                    nxt: Vec::new(),
                    fail_to: None,
                    dct_to: None,
                    pattern_len: total_chars,
                }
            }
        }
    }

    /// Add a link to the node. The link is pushed to the node's list of following nodes.
    fn add_link(&mut self, link: Link) {
        self.nxt.push(link);
    }

    /// Set the node's failure node to the given node ID.
    fn add_fail_node(&mut self, node_id: NodeId) {
        self.fail_to.replace(node_id);
    }

    /// Get the vector of following nodes
    #[inline]
    pub fn next_nodes(&self) -> &Vec<Link> {
        &self.nxt
    }

    /// Get adjacent (failure) link of this node
    #[inline]
    pub fn fail_node(&self) -> Option<NodeId> {
        self.fail_to
    }

    /// Get the first dictionary node found by following the trie's failure links
    /// from this node.
    #[inline]
    pub fn fail_dct(&self) -> Option<NodeId> {
        self.dct_to
    }

    /// Get a link to a following node for a suffix starting with the given character
    ///
    /// Use binary search to search for a link that has the given character. Returns None
    /// if there is no following link indexed with the given character.
    pub fn follow_link(&self, ch: char) -> Option<NodeId> {
        if self.nxt.len() < 8 {
            // Small array: simple search
            self.nxt
                .iter()
                .find(|&l| l.get_char() == ch)
                .map(|l| l.get_node_id())
        } else {
            // Larger array: binary search
            self.nxt
                .binary_search_by_key(&ch, |l| l.get_char())
                .ok()
                .map(|i| self.nxt[i].get_node_id())
        }
    }

    /// Get the value and keyword of the node. These are not None if the node is a dictionary node.
    pub fn value_keyword(&self) -> Option<(&str, &str)> {
        match (&self.value, &self.keyword) {
            (Some(s), Some(t)) => Some((s, t)),
            _ => None,
        }
    }
}

/// Represents the root of the Aho-Corasick prefix tree
pub struct TrieRoot {
    nodes: Vec<Node>,
    options: SearchOptions,
    max_pattern_len: usize,
}

impl TrieRoot {
    /// Instantiate a new, empty prefix tree. This should not be called directly, use
    /// create_prefix_tree function instead.
    fn new(options: SearchOptions) -> Self {
        Self {
            // Add root node
            nodes: vec![Node::default()],
            max_pattern_len: 0,
            options,
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

    /// Get a node without bounds checking  - to use with guaranteed-safe Ids
    #[inline]
    fn get_node_unchecked(&self, node_id: NodeId) -> &Node {
        &self.nodes[node_id]
    }

    /// Get the vector of nodes
    pub fn nodes_vec(&self) -> &Vec<Node> {
        &self.nodes
    }

    /// Get the ID of the root node of the tree
    #[inline]
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
    fn add_link(&mut self, from: NodeId, to: NodeId, c: char, fail_link: bool) -> SearchResult<()> {
        if to >= self.nodes.len() {
            return Err(SearchError::InvalidNodeId(to));
        }
        if from >= self.nodes.len() {
            return Err(SearchError::InvalidNodeId(from));
        }
        let from_node = &mut self.nodes[from];
        if fail_link {
            from_node.add_fail_node(to);
        } else {
            let lnk = Link(c, to);
            from_node.add_link(lnk);
        }
        Ok(())
    }

    /// Add a new pattern / string to the prefix tree.
    ///
    /// Add the nodes corresponding to a new string to the prefix tree along with
    /// their corresponding "following" links. Adjacent or "failure" links must be added
    /// separately by calling the "compute_failure_links" function. This is only meant to
    /// be used during creation of the trie structure.
    fn add_pattern(&mut self, mut new_item: String, kw: Option<String>) -> SearchResult<()> {
        // Normalize pattern to unicode NFC (combined)
        new_item = new_item.nfc().collect();

        let mut current_id = self.root_node_id();
        let characters: Vec<char> = new_item.chars().collect();
        if characters.len() > self.max_pattern_len {
            self.max_pattern_len = characters.len();
        }

        for (i, &c) in characters.iter().enumerate() {
            match self.get_node_unchecked(current_id).follow_link(c) {
                Some(nid) => current_id = nid,
                None => {
                    // Next node not already present - add it to the trie
                    let (val, key) = if i == characters.len() - 1 {
                        (Some(new_item.clone()), kw.clone())
                    } else {
                        (None, None)
                    };
                    let node_id = self.add_node(Node::new(val, key));
                    self.add_link(current_id, node_id, c, false)?;

                    current_id = node_id;
                }
            }
        }
        Ok(())
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
            // Push children to queue
            let curr_node = self.get_node(current_id)?;
            for Link(c, nid) in curr_node.next_nodes() {
                queue.push_back((current_id, *nid, *c));
            }

            // Level 1 failure nodes point to root
            if parent_id == self.root_node_id() {
                self.add_link(current_id, parent_id, edge_char, true)?;
                continue;
            }

            let parent = self.get_node(parent_id)?;
            let mut check_id = match parent.fail_node() {
                Some(nid) => nid,
                _ => return Err(SearchError::MissingLink(parent_id)),
            };
            let mut check = self.get_node(check_id)?;
            loop {
                match check.follow_link(edge_char) {
                    Some(nid) => {
                        // Found the node
                        self.add_link(current_id, nid, '\0', true)?;
                        break;
                    }
                    None => {
                        // No node found
                        if check_id == self.root_node_id() {
                            self.add_link(current_id, check_id, edge_char, true)?;
                            break;
                        } else if let Some(nid) = check.fail_node() {
                            check_id = nid;
                            check = self.get_node(check_id)?;
                        } else {
                            return Err(SearchError::MissingLink(check_id));
                        }
                    }
                }
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
            if let Some(nid) = curr_node.follow_link(c) {
                current = nid;
            } else {
                return None;
            }
        }
        Some(current)
    }

    /// Sort the lists of next links for all the nodes in the tree. This should be called just
    /// once when initializing. Also assigns the dictionary failure nodes.
    fn finalize_links(&mut self) {
        for node in self.nodes.iter_mut() {
            node.nxt.sort();
        }

        for i in 0..self.nodes.len() {
            if i == self.root_node_id() {
                continue;
            }

            // Follow fail nodes until reaching root or a dictionary node
            let mut curr_id = self.nodes[i].fail_node().unwrap();
            while curr_id != self.root_node_id() {
                let curr = self.get_node_unchecked(curr_id);
                match curr.value_keyword() {
                    Some(_) => {
                        self.nodes[i].dct_to.replace(curr_id);
                        break;
                    }
                    None => {
                        curr_id = curr.fail_node().unwrap();
                    }
                }
            }
        }
    }
}

/// Given a vector of strings, return a vector of (pattern, keyword).
///
/// This instantiates the new vector by adding a None keyword to each element.
/// This will make a vector that can be used to instantiate the prefix tree.
pub fn add_keyword_slot(patterns: Vec<String>) -> Vec<(String, Option<String>)> {
    let mut new = Vec::with_capacity(patterns.len());

    for val in patterns {
        new.push((val, None));
    }

    new
}

/// Instantiate a prefix tree for search from the given dictionary (list of strings). Returns
/// an error if the dictionary is empty or contains empty strings or duplicates.
///
/// Examples
/// ```rust
/// use ac_search_rs::trie;
///
/// let my_dictionary = trie::add_keyword_slot(vec![
///     String::from("abc"),
///     String::from("ab"),
///     String::from("cd"),
/// ]);
/// let prefix_tree = trie::create_prefix_tree(my_dictionary, None).unwrap();
///
/// // Use non-default options
/// let my_dictionary = trie::add_keyword_slot(vec![
///     String::from("abc"),
///     String::from("ab"),
///     String::from("cd"),
/// ]);
/// let opts = trie::SearchOptions{case_sensitive: false, check_bounds: true};
/// let prefix_tree = trie::create_prefix_tree(my_dictionary, Some(opts)).unwrap();
///
/// // With keywords and variants to match different patterns to "Python"
/// let my_dictionary = vec![
///     (String::from("Python"), None),
///     (String::from("Python3"), Some(String::from("Python"))),
///     (String::from("PythonLang"), Some(String::from("Python"))),
/// ];
/// let prefix_tree = trie::create_prefix_tree(my_dictionary, None).unwrap();
/// ```
pub fn create_prefix_tree(
    mut dictionary: Vec<(String, Option<String>)>,
    opts: Option<SearchOptions>,
) -> SearchResult<TrieRoot> {
    if dictionary.is_empty() {
        return Err(SearchError::InvalidDictionary);
    }

    let opts_obj = opts.unwrap_or_default();
    if !opts_obj.case_sensitive {
        // Case insensitive - convert all dictionary elements to lowercase
        for item in &mut dictionary {
            item.0 = item.0.to_lowercase();
        }
    }
    dictionary.sort();

    // Validate dictionary - no duplicate patterns
    for (item, next) in dictionary.iter().zip(&dictionary[1..]) {
        if item.0 == next.0 {
            return Err(SearchError::DuplicateNode);
        } else if item.0.is_empty() || next.0.is_empty() {
            return Err(SearchError::InvalidDictionary);
        }
    }

    let mut pt = TrieRoot::new(opts_obj);
    for (pattern, keyword) in dictionary {
        pt.add_pattern(pattern, keyword).unwrap();
    }
    pt.compute_failure_links()?;
    pt.finalize_links();
    Ok(pt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        let dictionary = add_keyword_slot(vec![
            String::from("ab"),
            String::from("abc"),
            String::from("cd"),
        ]);
        let pt = create_prefix_tree(dictionary, None).unwrap();

        // Verify root node properties
        assert!(pt.root_node().fail_node().is_none());
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
            if let Some((value, _)) = node.value_keyword() {
                dct_vals.push(value.to_string());
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
        let pt = create_prefix_tree(
            add_keyword_slot(vec![
                String::from("ab"),
                String::from("abc"),
                String::from("bcd"),
                String::from("cd"),
                String::from("cb"),
            ]),
            None,
        )
        .unwrap();

        // Check 'ab' node
        let ab_node = pt.get_node(pt.node_by_path("ab").unwrap()).unwrap();
        let ab_nxt = match ab_node.value_keyword() {
            None => panic!("Expected a dictionary node"),
            Some((value, _)) => {
                assert_eq!("ab", value);
                &ab_node.nxt
            }
        };
        assert_eq!(ab_nxt.len(), 1);
        let Link(c, _) = ab_nxt[0];
        assert_eq!(c, 'c');

        // Check 'c' node
        let c_node = pt.get_node(pt.node_by_path("c").unwrap()).unwrap();
        let c_nxt = match c_node.value_keyword() {
            None => &c_node.nxt,
            Some(_) => panic!("Expected intermediate node"),
        };
        assert_eq!(c_nxt.len(), 2);
        let mut chars: Vec<char> = c_nxt.iter().map(|Link(c, _)| *c).collect();
        chars.sort();
        assert_eq!(chars, ['b', 'd']);

        // Nonexistent nodes
        if dbg!(pt.node_by_path("cdb")).is_some() {
            panic!("Did not expect to find node!")
        }
        if dbg!(pt.node_by_path("xyz")).is_some() {
            panic!("Did not expect to find node!")
        }
        if dbg!(pt.node_by_path("abd")).is_some() {
            panic!("Did not expect to find node!")
        }
        if dbg!(pt.node_by_path("")).is_some() {
            panic!("Did not expect to find node!")
        }
    }

    #[test]
    fn test_adj_links() {
        let pt = create_prefix_tree(
            add_keyword_slot(vec![
                String::from("ab"),
                String::from("abc"),
                String::from("bcd"),
                String::from("cd"),
            ]),
            None,
        )
        .unwrap();

        assert_eq!(pt.root_node().next_nodes().len(), 3);
        let ab_node = dbg!(pt.node_by_path("ab").unwrap());
        let b_node = dbg!(pt.node_by_path("b").unwrap());
        let c_node = dbg!(pt.node_by_path("c").unwrap());
        let cd_node = dbg!(pt.node_by_path("cd").unwrap());
        let bc_node = dbg!(pt.node_by_path("bc").unwrap());
        let abc_node = dbg!(pt.node_by_path("abc").unwrap());
        let bcd_node = dbg!(pt.node_by_path("bcd").unwrap());

        // bc -> c
        if let Some(nid) = pt.get_node(bc_node).unwrap().fail_node() {
            assert_eq!(nid, c_node);
        } else {
            panic!("Expected an adjacent node!")
        }

        // bcd -> cd
        if let Some(nid) = pt.get_node(bcd_node).unwrap().fail_node() {
            assert_eq!(nid, cd_node);
        } else {
            panic!("Expected an adjacent node!")
        }

        // abc -> bc
        if let Some(nid) = pt.get_node(abc_node).unwrap().fail_node() {
            assert_eq!(nid, bc_node);
        } else {
            panic!("Expected an adjacent node!")
        }

        // ab -> b
        if let Some(nid) = pt.get_node(ab_node).unwrap().fail_node() {
            assert_eq!(nid, b_node);
        } else {
            panic!("Expected an adjacent node!")
        }

        // b -> root
        if let Some(nid) = pt.get_node(b_node).unwrap().fail_node() {
            assert_eq!(nid, pt.root_node_id());
        } else {
            panic!("Expected an adjacent node!")
        }

        // cd -> root
        if let Some(nid) = pt.get_node(cd_node).unwrap().fail_node() {
            assert_eq!(nid, pt.root_node_id());
        } else {
            panic!("Expected an adjacent node!")
        }
    }

    #[test]
    fn test_adj_links_medium() {
        let pt = create_prefix_tree(
            add_keyword_slot(vec![
                String::from("a"),
                String::from("ab"),
                String::from("bab"),
                String::from("bca"),
                String::from("ca"),
                String::from("bc"),
            ]),
            None,
        )
        .unwrap();

        let a_node = dbg!(pt.node_by_path("a").unwrap());
        let b_node = dbg!(pt.node_by_path("b").unwrap());
        let c_node = dbg!(pt.node_by_path("c").unwrap());

        let ab_node = dbg!(pt.node_by_path("ab").unwrap());
        let ba_node = dbg!(pt.node_by_path("ba").unwrap());
        let bc_node = dbg!(pt.node_by_path("bc").unwrap());
        let ca_node = dbg!(pt.node_by_path("ca").unwrap());
        let bca_node = dbg!(pt.node_by_path("bca").unwrap());
        let bab_node = dbg!(pt.node_by_path("bab").unwrap());

        // ba -> a
        if let Some(nid) = pt.get_node(ba_node).unwrap().fail_node() {
            assert_eq!(nid, a_node);
        } else {
            panic!("Expected an adjacent node!")
        }

        // ca -> a
        if let Some(nid) = pt.get_node(ca_node).unwrap().fail_node() {
            assert_eq!(nid, a_node);
        } else {
            panic!("Expected an adjacent node!")
        }

        // bc -> c
        if let Some(nid) = pt.get_node(bc_node).unwrap().fail_node() {
            assert_eq!(nid, c_node);
        } else {
            panic!("Expected an adjacent node!")
        }

        // ab -> b
        if let Some(nid) = pt.get_node(ab_node).unwrap().fail_node() {
            assert_eq!(nid, b_node);
        } else {
            panic!("Expected an adjacent node!")
        }

        // bca -> ca
        if let Some(nid) = pt.get_node(bca_node).unwrap().fail_node() {
            assert_eq!(nid, ca_node);
        } else {
            panic!("Expected an adjacent node!")
        }

        // bab -> ab
        if let Some(nid) = pt.get_node(bab_node).unwrap().fail_node() {
            assert_eq!(nid, ab_node);
        } else {
            panic!("Expected an adjacent node!")
        }
    }

    #[test]
    #[should_panic]
    fn test_initialization_empty_str() {
        let res = create_prefix_tree(
            add_keyword_slot(vec![String::from("abc"), String::from("")]),
            None,
        );
        res.unwrap();
    }

    #[test]
    #[should_panic]
    fn test_initialization_empty_dct() {
        let res = create_prefix_tree(Vec::new(), None);
        res.unwrap();
    }

    #[test]
    #[should_panic]
    fn test_initialization_duplicate() {
        let res = create_prefix_tree(
            add_keyword_slot(vec![
                String::from("abc"),
                String::from("xy"),
                String::from("abc"),
                String::from("opq"),
            ]),
            None,
        );
        res.unwrap();
    }

    #[test]
    fn test_create_case_insensitive() {
        let pt = create_prefix_tree(
            add_keyword_slot(vec![
                String::from("abc"),
                String::from("xY"),
                String::from("Xyz"),
                String::from("AB"),
            ]),
            Some(SearchOptions {
                case_sensitive: false,
                check_bounds: false,
            }),
        )
        .unwrap();

        assert_eq!(pt.total_nodes(), 7);
        let mut total_dct = 0;
        for node in pt.nodes {
            if let Some((value, _)) = node.value_keyword() {
                total_dct += 1;
                assert_eq!(value, value.to_lowercase());
            }
        }
        assert_eq!(total_dct, 4);
    }

    #[test]
    #[should_panic]
    fn test_initialization_case_insensitive_duplicate() {
        let res = create_prefix_tree(
            add_keyword_slot(vec![
                String::from("abc"),
                String::from("xy"),
                String::from("aBc"),
            ]),
            Some(SearchOptions {
                case_sensitive: false,
                check_bounds: false,
            }),
        );
        res.unwrap();
    }

    #[test]
    #[should_panic]
    fn test_instantiate_keywords_duplicate() {
        let dct = vec![
            (String::from("abc"), Some(String::from("Abc"))),
            (String::from("ABC"), Some(String::from("Acd"))),
            (String::from("aBc"), Some(String::from("ABC"))),
            (String::from("abc"), Some(String::from("def"))),
        ];
        create_prefix_tree(dct, None).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_instantiate_keywords_duplicate_case_insensitive() {
        let dct = vec![
            (String::from("abc"), Some(String::from("Abc"))),
            (String::from("def"), Some(String::from("Def"))),
            (String::from("gHI"), Some(String::from("Ghi"))),
            (String::from("ABC"), Some(String::from("Abc"))),
        ];
        create_prefix_tree(
            dct,
            Some(SearchOptions {
                case_sensitive: false,
                check_bounds: false,
            }),
        )
        .unwrap();
    }

    #[test]
    fn test_dct_links_have_kw() {
        let pt = create_prefix_tree(
            add_keyword_slot(vec![
                String::from("ab"),
                String::from("abc"),
                String::from("bcd"),
                String::from("cd"),
                String::from("acdb"),
            ]),
            None,
        )
        .unwrap();

        let mut total_dct = 0;
        for node in &pt.nodes {
            if let Some(nid) = dbg!(node).fail_dct() {
                total_dct += 1;
                dbg!(pt.get_node_unchecked(nid)).value_keyword().unwrap();
            }
        }
        // Expect bcd -> cd, acd -> cd
        assert_eq!(total_dct, 2);
    }

    #[test]
    fn test_dct_links_vals() {
        let pt = create_prefix_tree(
            add_keyword_slot(vec![
                String::from("ab"),
                String::from("abc"),
                String::from("bcd"),
                String::from("cd"),
                String::from("acdb"),
            ]),
            None,
        )
        .unwrap();

        let bcd_id = pt.node_by_path("bcd").unwrap();
        let bcd_node = pt.get_node_unchecked(bcd_id);

        let cd_id = pt.node_by_path("cd").unwrap();

        let acd_id = pt.node_by_path("acd").unwrap();
        let acd_node = pt.get_node_unchecked(acd_id);

        // Expect bcd -> cd, acd -> cd
        assert_eq!(bcd_node.dct_to.unwrap(), cd_id);
        assert_eq!(acd_node.dct_to.unwrap(), cd_id);
    }
}

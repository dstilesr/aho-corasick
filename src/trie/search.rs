use std::cmp::Ordering;
use std::collections::VecDeque;

use super::{Link, Node, NodeId, SearchResult, TrieRoot, follow_links};

/// Represents a match found in a text
#[derive(PartialEq, Eq, Debug)]
pub struct Match {
    start: usize,
    end: usize,
    value: String,
}

impl Match {
    /// Instantiate a new match from a value and 1 + index of the last character in the match.
    pub fn new(value: String, end_pos: usize) -> Self {
        Self {
            start: end_pos - value.chars().count(),
            end: end_pos,
            value,
        }
    }
}

impl PartialOrd for Match {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else if self.start < other.start
            || (self.start == other.start && self.value < other.value)
        {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Greater)
        }
    }
}

impl Ord for Match {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl TrieRoot {
    /// Update the lists of active nodes and matches upon feeding a new character.
    fn update_node_list(
        &self,
        active_nodes: &mut VecDeque<NodeId>,
        matches: &mut Vec<Match>,
        idx: usize,
        c: char,
    ) -> SearchResult<()> {
        let active_total = active_nodes.len();
        for _ in 0..active_total {
            let curr_node = self.get_node(active_nodes.pop_front().unwrap())?;

            // Add current node if it is a full match
            if let Node::DictNode {
                value,
                nxt: _,
                adj: _,
            } = curr_node
            {
                matches.push(Match::new(value.clone(), idx));
            }

            for follow in follow_links!(curr_node.next_nodes(), c) {
                active_nodes.push_back(follow);

                // Add adjacent nodes as well
                let nxt_node = self.get_node(follow)?;
                if let Some(Link(_, nid)) = nxt_node.adj_node() {
                    active_nodes.push_back(*nid);
                }
            }
        }
        Ok(())
    }

    /// Find all matches for the search dictionary in the given text.
    pub fn find_text_matches(&self, text: &str) -> SearchResult<Vec<Match>> {
        let mut active: VecDeque<NodeId> = VecDeque::with_capacity(self.total_nodes());
        let mut matches: Vec<Match> = Vec::new();

        // Initialize
        active.push_back(self.root_node_id());

        let mut idx = 0;
        for c in text.chars() {
            self.update_node_list(&mut active, &mut matches, idx, c)?;

            // Add root node
            active.push_back(self.root_node_id());
            idx += 1;
        }

        // Final update with remaining dictionary nodes after text end
        for nid in active {
            if let Node::DictNode {
                value,
                nxt: _,
                adj: _,
            } = self.get_node(nid)?
            {
                matches.push(Match::new(value.clone(), idx))
            }
        }
        Ok(matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Make a sample tree for the dictionary {ab, abc, cd} (built manually)
    fn sample_tree_1() -> TrieRoot {
        let mut trie = TrieRoot::new();

        // Add nodes
        let a_node_id = trie.add_node(Node::new(None));
        let b_node_id = trie.add_node(Node::new(Some(String::from("ab"))));
        let c_node_id = trie.add_node(Node::new(Some(String::from("abc"))));
        let c_mid_node_id = trie.add_node(Node::new(None));
        let d_node_id = trie.add_node(Node::new(Some(String::from("cd"))));

        // Links
        trie.add_link(trie.root_node_id(), a_node_id, 'a', false)
            .unwrap();
        trie.add_link(trie.root_node_id(), c_mid_node_id, 'c', false)
            .unwrap();

        trie.add_link(a_node_id, b_node_id, 'b', false).unwrap();
        trie.add_link(b_node_id, c_node_id, 'c', false).unwrap();
        trie.add_link(c_node_id, c_mid_node_id, 'c', true).unwrap();

        trie.add_link(c_mid_node_id, d_node_id, 'd', false).unwrap();
        trie
    }

    #[test]
    fn test_search_simple() {
        let pref_tree = sample_tree_1();
        let sample = "123 a ab c d cd bc abc";

        let mut matches = dbg!(pref_tree.find_text_matches(sample).unwrap());
        matches.sort();
        // Expect 4 matches
        assert_eq!(matches.len(), 4);

        // Validate individual matches
        assert_eq!(&matches[0].value, "ab");
        assert_eq!(matches[0].start, 6);
        assert_eq!(matches[0].end - matches[0].start, matches[0].value.len());

        assert_eq!(&matches[1].value, "cd");
        assert_eq!(matches[1].start, 13);
        assert_eq!(matches[1].end - matches[1].start, matches[1].value.len());

        assert_eq!(&matches[2].value, "ab");
        assert_eq!(matches[2].start, 19);
        assert_eq!(matches[2].end - matches[2].start, matches[2].value.len());

        assert_eq!(&matches[3].value, "abc");
        assert_eq!(matches[3].start, 19);
        assert_eq!(matches[3].end - matches[3].start, matches[3].value.len());
    }

    #[test]
    fn test_search_no_matches() {
        let pref_tree = sample_tree_1();
        let sample = "123 x, y aBcD wXyAb dc";
        let matches = dbg!(pref_tree.find_text_matches(sample).unwrap());
        assert!(matches.is_empty());
    }
}

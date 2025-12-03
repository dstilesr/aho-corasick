use std::cmp::Ordering;

use crate::trie::SearchError;

use super::{Link, Node, SearchResult, TrieRoot};

/// Represents a match found in a text
#[derive(PartialEq, Eq, Debug)]
pub struct Match {
    pub start: usize,
    pub end: usize,
    pub value: String,
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
    /// Find all matches for the search dictionary in the given text.
    ///
    /// Example:
    /// ```rust
    /// use aho_corasick::trie::{self, Match};
    ///
    /// let search_dictionary = vec![
    ///     String::from("a"),
    ///     String::from("abb"),
    ///     String::from("bb"),
    ///     String::from("bCd"),
    ///     String::from("bCx"),
    ///     String::from("Cxaabb"),
    /// ];
    /// let search_tree = trie::create_prefix_tree(search_dictionary).unwrap();
    /// let haystack = String::from("This is a string with some nonsense to check: abbaaCxa bCdbCxbb");
    /// let matches = search_tree.find_text_matches(&haystack).unwrap();
    ///
    /// for Match{start, end, value} in matches {
    ///    println!("Found matching string '{value}' in characters {start}-{end}");
    /// }
    /// ```
    pub fn find_text_matches(&self, text: &str) -> SearchResult<Vec<Match>> {
        let mut matches: Vec<Match> = Vec::new();
        let root_id = self.root_node_id();

        let mut curr_id = root_id;
        let mut current = self.root_node();

        for (idx, ch) in text.chars().enumerate() {
            // Node does not have link with the required char - try failovers
            // until node found or root reached
            while curr_id != root_id
                && let None = current.follow_link(ch)
            {
                match current.adj_node() {
                    None => return Err(SearchError::MissingLink(curr_id)),
                    Some(Link(_, nid)) => {
                        curr_id = *nid;
                        current = self.get_node(*nid)?;
                    }
                }
            }

            // Move to node if edge available. Now we are at a node with the
            // right last character or at root.
            if let Some(Link(_, nid)) = current.follow_link(ch) {
                curr_id = *nid;
                current = self.get_node(*nid)?;
            }

            // Check for matches
            let mut check_id = curr_id;
            while check_id != root_id {
                let check = self.get_node(check_id)?;
                if let Node::DictNode {
                    value,
                    nxt: _,
                    adj: _,
                } = check
                {
                    matches.push(Match::new(value.clone(), idx + 1));
                }
                match check.adj_node() {
                    None => return Err(SearchError::MissingLink(check_id)),
                    Some(Link(_, nid)) => {
                        check_id = *nid;
                    }
                }
            }
        }

        Ok(matches)
    }
}

#[cfg(test)]
mod tests {

    use super::super::create_prefix_tree;
    use super::*;
    use rand::{Rng, distr::Alphanumeric};

    /// Make a sample tree for the dictionary {ab, abc, cd}
    fn sample_tree_1() -> TrieRoot {
        create_prefix_tree(vec![
            String::from("ab"),
            String::from("abc"),
            String::from("cd"),
        ])
        .unwrap()
    }

    /// Generate a random alphanumeric string of the given length (in bytes)
    fn random_string(length: usize) -> String {
        let mut rng = rand::rng();
        (0..length)
            .map(|_| rng.sample(Alphanumeric) as char)
            .collect()
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

    #[test]
    fn test_search_random_string() {
        let haystack = random_string(8192);
        let haystack_chars: Vec<char> = haystack.chars().collect();

        let pt = create_prefix_tree(vec![
            String::from("a"),
            String::from("b"),
            String::from("aB"),
            String::from("bcd"),
            String::from("abcd"),
            String::from("AbcdaB"),
            String::from("0"),
            String::from("0bcd"),
            String::from("a0b"),
        ])
        .unwrap();

        let mut matches = pt.find_text_matches(&haystack).unwrap();
        matches.sort();
        assert!(dbg!(matches.len()) > 0);

        for Match { start, end, value } in &matches {
            assert_eq!(*end - *start, value.len());

            let val_chars: Vec<char> = value.chars().collect();
            assert_eq!(&val_chars, &haystack_chars[*start..*end]);
        }
    }
}

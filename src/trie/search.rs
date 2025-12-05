use super::{Link, Node, SearchError, SearchResult, TrieRoot};

/// Represents a match found in a text.
///
/// The match contains the index of the start and end characters of the match, so that
/// `haystack_chars[start:end]` should be equal to the character vector of the "value". Note
/// that matches are done on a character level, not a byte level, so indexing the string directly
/// may not yield the expected result.
#[derive(PartialEq, Eq, Debug, PartialOrd, Ord)]
pub struct Match {
    /// Index of first character in the match
    start: usize,

    /// The match value substring that was actually found
    value: String,

    /// The corresponding keyword / standard form of the match
    kw: String,

    /// 1 + index of last character in the match
    end: usize,
}

impl Match {
    /// Instantiate a new match from a value and 1 + index of the last character in the match.
    pub fn new(value: String, kw: String, end_pos: usize) -> Self {
        Self {
            start: end_pos - value.chars().count(),
            end: end_pos,
            kw,
            value,
        }
    }

    /// Return the value stored in the match.
    pub fn value(&self) -> &String {
        &self.value
    }

    /// Return the value of the associated keyword of the match
    pub fn keyword(&self) -> &String {
        &self.kw
    }

    /// Return the range of characters the match spans.
    pub fn char_range(&self) -> (usize, usize) {
        (self.start, self.end)
    }
}

impl TrieRoot {
    /// Find all matches for the search dictionary in the given text.
    ///
    /// Example:
    /// ```rust
    /// use ah_search_rs::trie::{self, Match};
    ///
    /// let search_dictionary = trie::add_keyword_slot(vec![
    ///     String::from("a"),
    ///     String::from("abb"),
    ///     String::from("bb"),
    ///     String::from("bCd"),
    ///     String::from("bCx"),
    ///     String::from("Cxaabb"),
    /// ]);
    /// let search_tree = trie::create_prefix_tree(search_dictionary, None).unwrap();
    /// let haystack = String::from("This is a string with some nonsense to check: abbaaCxa bCdbCxbb");
    /// let matches = search_tree.find_text_matches(haystack).unwrap();
    ///
    /// for m in matches {
    ///    let value: &str =  m.value();
    ///    let (start, end) = m.char_range();
    ///    println!("Found matching string '{value}' in characters {start}-{end}");
    /// }
    /// ```
    pub fn find_text_matches(&self, mut text: String) -> SearchResult<Vec<Match>> {
        if !self.options.case_sensitive {
            text = text.to_lowercase();
        };

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
                if let Node::DictNode { value, keyword, .. } = check {
                    matches.push(Match::new(value.clone(), keyword.clone(), idx + 1));
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

    use crate::trie::SearchOptions;

    use super::super::{add_keyword_slot, create_prefix_tree};
    use super::*;
    use rand::{Rng, distr::Alphanumeric};

    /// Make a sample tree for the dictionary {ab, abc, cd}
    fn sample_tree_1() -> TrieRoot {
        create_prefix_tree(
            add_keyword_slot(vec![
                String::from("ab"),
                String::from("abc"),
                String::from("cd"),
            ]),
            None,
        )
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

        let mut matches = dbg!(pref_tree.find_text_matches(sample.to_string()).unwrap());
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
        let matches = dbg!(pref_tree.find_text_matches(sample.to_string()).unwrap());
        assert!(matches.is_empty());
    }

    #[test]
    fn test_search_random_string() {
        let haystack = random_string(8192);
        let haystack_chars: Vec<char> = haystack.chars().collect();

        let pt = create_prefix_tree(
            add_keyword_slot(vec![
                String::from("a"),
                String::from("b"),
                String::from("aB"),
                String::from("bcd"),
                String::from("abcd"),
                String::from("AbcdaB"),
                String::from("0"),
                String::from("0bcd"),
                String::from("a0b"),
            ]),
            None,
        )
        .unwrap();

        let mut matches = pt.find_text_matches(haystack).unwrap();
        matches.sort();
        assert!(dbg!(matches.len()) > 0);

        for Match {
            start, end, value, ..
        } in &matches
        {
            assert_eq!(*end - *start, value.len());

            let val_chars: Vec<char> = value.chars().collect();
            assert_eq!(&val_chars, &haystack_chars[*start..*end]);
        }
    }

    #[test]
    fn test_search_keywords() {
        let dct = vec![
            (String::from("abc"), None),
            (String::from("ac"), Some(String::from("abc"))),
            (String::from("ABC"), Some(String::from("abc"))),
            (String::from("acq"), Some(String::from("abc"))),
        ];
        let pt = create_prefix_tree(dct, None).unwrap();
        let matches = dbg!(pt.find_text_matches(String::from("abq dc ac ABCac pqracq"))).unwrap();

        assert_eq!(matches.len(), 5);
        for m in matches {
            assert_eq!(m.keyword(), "abc")
        }

        let dct = vec![
            (String::from("abc"), None),
            (String::from("ab"), Some(String::from("ab"))),
            (String::from("ABC"), Some(String::from("abc"))),
            (String::from("acq"), Some(String::from("ab"))),
        ];
        let pt = create_prefix_tree(dct, None).unwrap();
        let matches = dbg!(pt.find_text_matches(String::from("abq dc ac ABCac pqracq"))).unwrap();
        assert_eq!(matches.len(), 3);

        assert_eq!(matches[0].value(), "ab");
        assert_eq!(matches[0].keyword(), "ab");

        assert_eq!(matches[1].value(), "ABC");
        assert_eq!(matches[1].keyword(), "abc");

        assert_eq!(matches[2].value(), "acq");
        assert_eq!(matches[2].keyword(), "ab");
    }

    #[test]
    fn test_search_keywords_uncased() {
        let dct = vec![
            (String::from("abc"), Some(String::from("Abc"))),
            (String::from("ab"), Some(String::from("Ab"))),
            (String::from("DC"), Some(String::from("Abc"))),
            (String::from("acq"), Some(String::from("Ab"))),
        ];
        let pt = create_prefix_tree(
            dct,
            Some(SearchOptions {
                check_bounds: false,
                case_sensitive: false,
            }),
        )
        .unwrap();
        let matches = dbg!(pt.find_text_matches(String::from("aBq dc ABCac pqracQ AbC"))).unwrap();
        assert_eq!(matches.len(), 7);

        assert_eq!(matches[0].value(), "ab");
        assert_eq!(matches[0].keyword(), "Ab");

        assert_eq!(matches[1].value(), "dc");
        assert_eq!(matches[1].keyword(), "Abc");

        assert_eq!(matches[2].value(), "ab");
        assert_eq!(matches[2].keyword(), "Ab");

        assert_eq!(matches[3].value(), "abc");
        assert_eq!(matches[3].keyword(), "Abc");

        assert_eq!(matches[4].value(), "acq");
        assert_eq!(matches[4].keyword(), "Ab");

        assert_eq!(matches[5].value(), "ab");
        assert_eq!(matches[5].keyword(), "Ab");

        assert_eq!(matches[6].value(), "abc");
        assert_eq!(matches[6].keyword(), "Abc");
    }
}

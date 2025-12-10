use super::{Link, SearchError, SearchResult, TrieRoot};
use std::collections::VecDeque;

/// Return whether the given character is a "word character", i.e. a Unicode
/// alphanumeric character, a number or an underscore.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Represents a match found in a text.
///
/// The match contains the index of the start and end characters of the match, so that
/// `haystack_chars[start:end]` should be equal to the character vector of the "value". Note
/// that matches are done on a character level, not a byte level, so indexing the string directly
/// may not yield the expected result.
///
/// **CAVEAT**
/// Matches cannot outlive the TrieRoot object that created them. This is because the values and
/// keywords are references to those stored in the Trie to avoid excessive cloning. Matches must be
/// processed / consumed immediately after search.
#[derive(PartialEq, Eq, Debug, PartialOrd, Ord)]
pub struct Match<'a> {
    /// Index of first character in the match
    start: usize,

    /// The match value substring that was actually found
    value: &'a str,

    /// The corresponding keyword / standard form of the match
    kw: &'a str,

    /// 1 + index of last character in the match
    end: usize,
}

impl<'a> Match<'a> {
    /// Instantiate a new match from a value and 1 + index of the last character in the match.
    pub fn new(value: &'a str, kw: &'a str, end_pos: usize) -> Self {
        Self {
            start: end_pos - value.chars().count(),
            end: end_pos,
            kw,
            value,
        }
    }

    /// Return the value stored in the match.
    pub fn value(&self) -> &str {
        self.value
    }

    /// Return the value of the associated keyword of the match
    pub fn keyword(&self) -> &str {
        self.kw
    }

    /// Return the range of characters the match spans.
    pub fn char_range(&self) -> (usize, usize) {
        (self.start, self.end)
    }
}

/// Check if a match is word bounded. That is, check if the preceding and following characters
/// are not word-characters.
fn is_word_bounded(m: &Match, buffer: &VecDeque<char>, next_char: Option<char>) -> bool {
    let pat_len = m.end - m.start;
    let left = m.start == 0 || (!is_word_char(buffer[buffer.len() - pat_len - 1]));
    let right = match next_char {
        None => true,
        Some(ch) => !is_word_char(ch),
    };
    left && right
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
    pub fn find_text_matches<'a>(&'a self, mut text: String) -> SearchResult<Vec<Match<'a>>> {
        let mut char_buffer = VecDeque::with_capacity(self.max_pattern_len + 2);
        if !self.options.case_sensitive {
            text = text.to_lowercase();
        };

        let mut matches: Vec<Match> = Vec::new();
        let root_id = self.root_node_id();

        let mut curr_id = root_id;
        let mut current = self.root_node();

        let mut chars_iter = text.chars().peekable();
        let mut idx: usize = 0;
        while let Some(ch) = chars_iter.next() {
            // Buffer updates
            if self.options.check_bounds {
                if char_buffer.len() >= (self.max_pattern_len + 1) {
                    char_buffer.pop_front();
                }
                char_buffer.push_back(ch);
            }

            // Node does not have link with the required char - try failovers
            // until node found or root reached
            while curr_id != root_id
                && let None = current.follow_link(ch)
            {
                match current.fail_node() {
                    None => return Err(SearchError::MissingLink(curr_id)),
                    Some(nid) => {
                        curr_id = nid;
                        current = self.get_node_unchecked(nid);
                    }
                }
            }

            // Move to node if edge available. Now we are at a node with the
            // right last character or at root.
            if let Some(Link(_, nid)) = current.follow_link(ch) {
                curr_id = *nid;
                current = self.get_node_unchecked(*nid);
            }

            // Check for matches
            let mut check_id = curr_id;
            while check_id != root_id {
                let check = self.get_node_unchecked(check_id);
                if let Some((value, keyword)) = check.value_keyword() {
                    let m = Match::new(value, keyword, idx + 1);
                    let nxt_ch: Option<char> = chars_iter.peek().copied();

                    if (!self.options.check_bounds) || is_word_bounded(&m, &char_buffer, nxt_ch) {
                        matches.push(m);
                    }
                }
                check_id = check.fail_dct().unwrap_or(root_id);
            }
            idx += 1;
        }

        Ok(matches)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{SearchOptions, add_keyword_slot, create_prefix_tree};
    use super::*;
    use rand::{Rng, distr::Alphanumeric};
    use unicode_normalization::UnicodeNormalization;

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
        assert_eq!(matches[0].value, "ab");
        assert_eq!(matches[0].start, 6);
        assert_eq!(matches[0].end - matches[0].start, matches[0].value.len());

        assert_eq!(matches[1].value, "cd");
        assert_eq!(matches[1].start, 13);
        assert_eq!(matches[1].end - matches[1].start, matches[1].value.len());

        assert_eq!(matches[2].value, "ab");
        assert_eq!(matches[2].start, 19);
        assert_eq!(matches[2].end - matches[2].start, matches[2].value.len());

        assert_eq!(matches[3].value, "abc");
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

    #[test]
    fn test_search_bounded() {
        let dct = vec![
            (String::from("ab"), None),
            (String::from("abc"), Some("ab".to_string())),
            (String::from("bcd"), None),
            (String::from("def"), None),
        ];
        let pt = create_prefix_tree(
            dct,
            Some(SearchOptions {
                case_sensitive: true,
                check_bounds: true,
            }),
        )
        .unwrap();

        // No word bounds around patterns
        let matches = dbg!(pt.find_text_matches("abp pabc bcdefg abhx cab".to_string())).unwrap();
        assert_eq!(matches.len(), 0);

        // Word bounds around patterns
        let mut matches = dbg!(pt.find_text_matches("abc. -bcd- AB def".to_string())).unwrap();
        assert_eq!(matches.len(), 3);
        matches.sort();

        assert_eq!(matches[0].value(), "abc");
        assert_eq!(matches[1].value(), "bcd");
        assert_eq!(matches[2].value(), "def");
    }

    #[test]
    fn test_search_bounded_diacritics() {
        let dct = vec![
            (String::from("abc"), Some("ab".to_string())),
            (String::from("ábc"), Some("ab-accent".to_string())),
            (String::from("xyzò"), Some("xyzo-accent".to_string())),
            (String::from("xyzo"), None),
        ];
        let pt = create_prefix_tree(
            dct,
            Some(SearchOptions {
                case_sensitive: true,
                check_bounds: true,
            }),
        )
        .unwrap();

        let hs = "abc-ábc: xyzo!xyzò äbc-".nfc().collect();
        let matches = dbg!(pt.find_text_matches(hs)).unwrap();
        assert_eq!(matches.len(), 4);

        assert_eq!(matches[0].kw, "ab");
        assert_eq!(matches[1].kw, "ab-accent");
        assert_eq!(matches[2].kw, "xyzo");
        assert_eq!(matches[3].kw, "xyzo-accent");
    }

    #[test]
    fn test_search_bounded_case_insensitive() {
        let dct = vec![
            (String::from("abC"), Some("ab".to_string())),
            (String::from("áBC"), Some("ab-accent".to_string())),
            (String::from("xyzò"), Some("xyzo-accent".to_string())),
            (String::from("xyzo"), None),
            (String::from("yöyyi"), Some("Yoyyi".to_string())),
        ];
        let pt = create_prefix_tree(
            dct,
            Some(SearchOptions {
                case_sensitive: false,
                check_bounds: true,
            }),
        )
        .unwrap();
        let hs = "TEXT: ÁBC_3 Yöyyiaa, ABc, XYzò YÖyyi".nfc().collect();
        let matches = pt.find_text_matches(hs).unwrap();
        assert_eq!(matches.len(), 3);

        assert_eq!(matches[0].kw, "ab");
        assert_eq!(matches[1].kw, "xyzo-accent");
        assert_eq!(matches[2].kw, "Yoyyi");
    }
}

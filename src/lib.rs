#[cfg(feature = "python_bind")]
mod py_bind;

pub mod trie;

/// Macro to get an iterator over the  node Ids of a links vector whose links
/// match the given character. If no character is given, skips the filter step.
macro_rules! follow_links {
    ($node_list:expr, $character:expr) => {
        $node_list
            .iter()
            .filter(|Link(ch, _)| $character == *ch)
            .map(|Link(_, idx)| *idx)
    };
}

pub(crate) use follow_links;

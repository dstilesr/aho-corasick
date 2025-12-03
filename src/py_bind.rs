use super::trie::*;
use pyo3::exceptions as py_errs;
use pyo3::prelude::*;

/// Map a SearchError to an appropriate Python error
fn map_error_py(err: SearchError) -> PyErr {
    match err {
        SearchError::DuplicateNode => {
            PyErr::new::<py_errs::PyValueError, _>("Duplicate nodes in prefx tree")
        }
        SearchError::InvalidDictionary => {
            PyErr::new::<py_errs::PyValueError, _>("Invalid search dictionary provided!")
        }
        SearchError::InvalidNodeId(i) => {
            PyErr::new::<py_errs::PyKeyError, _>(format!("Tried to access invalid node: {}", i))
        }
        SearchError::MissingLink(i) => PyErr::new::<py_errs::PyValueError, _>(format!(
            "Node {} does not have a fallback link!",
            i
        )),
    }
}

/// Search for all occurences of strings in the "dictionary" in the given "haystack".
///
/// The dictionary must be a list of non-empty strings. It is usually better to process
/// texts in batch if you are using the same dictionary, since this requires only one
/// instantiation of the prefix tree.
#[pyfunction]
#[pyo3(signature = (dictionary: "list[str]", haystack: "str") -> "list[str]")]
fn search_in_text(dictionary: Vec<String>, haystack: String) -> PyResult<Vec<String>> {
    let prefix_tree = create_prefix_tree(dictionary).map_err(map_error_py)?;
    let matches = prefix_tree
        .find_text_matches(&haystack)
        .map_err(map_error_py)?;

    Ok(matches.iter().map(|m| m.value().to_string()).collect())
}

/// Search for all occurences of strings in the "dictionary" in the given "haystack" strings.
///
/// The dictionary must be a list of non-empty strings. If you have multiple texts, this is more efficient
/// than calling "search_in_text" individually on each one, since that would require the prefix tree to
/// be instantiated multiple times.
#[pyfunction]
#[pyo3(signature = (dictionary: "list[str]", haystacks: "list[str]") -> "list[list[str]]")]
fn search_in_texts(dictionary: Vec<String>, haystacks: Vec<String>) -> PyResult<Vec<Vec<String>>> {
    let prefix_tree = create_prefix_tree(dictionary).map_err(map_error_py)?;
    let mut matches_list = Vec::with_capacity(haystacks.len());

    for h in &haystacks {
        let matches = prefix_tree.find_text_matches(h).map_err(map_error_py)?;
        matches_list.push(matches.iter().map(|m| m.value().to_string()).collect());
    }

    Ok(matches_list)
}

#[pyo3::pymodule]
#[pyo3(name = "ah_search_rs")]
pub mod aho_corasick_search {

    #[pymodule_export]
    use super::{search_in_text, search_in_texts};
}

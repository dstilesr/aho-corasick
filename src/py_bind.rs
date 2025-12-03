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

/// Search for all occurences of strings in the "dictionary" in the given "haystack"
#[pyfunction]
#[pyo3(signature = (dictionary: "list[str]", haystack: "str") -> "list[str]")]
fn search_in_text(dictionary: Vec<String>, haystack: String) -> PyResult<Vec<String>> {
    let prefix_tree = create_prefix_tree(dictionary).map_err(map_error_py)?;
    let matches = prefix_tree
        .find_text_matches(&haystack)
        .map_err(map_error_py)?;

    Ok(matches.iter().map(|m| m.value().to_string()).collect())
}

#[pyo3::pymodule]
#[pyo3(name = "ah_search_rs")]
pub mod aho_corasick_search {

    #[pymodule_export]
    use super::search_in_text;
}

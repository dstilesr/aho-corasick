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

/// A Match found in a text. This contains the start character, end character, and the
/// string value of the match.
#[pyclass]
pub struct PyMatch {
    #[pyo3(get)]
    pub value: String,

    #[pyo3(get)]
    pub from_char: usize,

    #[pyo3(get)]
    pub to_char: usize,
}

impl From<&Match> for PyMatch {
    /// Convert a Match object from the Rust API into a PyMatch object to be used
    /// in the Python API
    fn from(m: &Match) -> Self {
        let (start, end) = m.char_range();
        let value = m.value().clone();
        Self {
            value: value,
            from_char: start,
            to_char: end,
        }
    }
}

#[pymethods]
impl PyMatch {
    /// Initialize a new match given the start and end of its character range, and the
    /// string value.
    #[new]
    #[pyo3(signature = (start: "int", end: "int", value: "str"))]
    pub fn new(start: usize, end: usize, value: String) -> PyResult<Self> {
        if start >= end {
            return Err(PyErr::new::<py_errs::PyValueError, _>(
                "Start must precede end of match!",
            ));
        } else if end - start != value.chars().count() {
            return Err(PyErr::new::<py_errs::PyValueError, _>(
                "Range length must match total characters in value!",
            ));
        }

        Ok(Self {
            from_char: start,
            to_char: end,
            value: value,
        })
    }

    pub fn __repr__(&self) -> String {
        format!(
            "PyMatch(start={}, end={}, value=\"{}\")",
            self.from_char,
            self.to_char,
            self.value.replace('"', "[QUOT]")
        )
    }
}

/// Search for all occurences of strings in the "dictionary" in the given "haystack".
///
/// The dictionary must be a list of non-empty strings. It is usually better to process
/// texts in batch if you are using the same dictionary, since this requires only one
/// instantiation of the prefix tree.
#[pyfunction]
#[pyo3(signature = (dictionary: "list[str]", haystack: "str", case_sensitive=true) -> "list[PyMatch]")]
fn search_in_text(
    dictionary: Vec<String>,
    haystack: String,
    case_sensitive: bool,
) -> PyResult<Vec<PyMatch>> {
    let opts = SearchOptions {
        case_sensitive,
        check_bounds: false,
    };
    let prefix_tree = create_prefix_tree(dictionary, Some(opts)).map_err(map_error_py)?;
    let matches = prefix_tree
        .find_text_matches(haystack)
        .map_err(map_error_py)?;

    Ok(matches.iter().map(PyMatch::from).collect())
}

/// Search for all occurences of strings in the "dictionary" in the given "haystack" strings.
///
/// The dictionary must be a list of non-empty strings. If you have multiple texts, this is more efficient
/// than calling "search_in_text" individually on each one, since that would require the prefix tree to
/// be instantiated multiple times.
#[pyfunction]
#[pyo3(signature = (dictionary: "list[str]", haystacks: "list[str]", case_sensitive=true) -> "list[list[str]]")]
fn search_in_texts(
    dictionary: Vec<String>,
    haystacks: Vec<String>,
    case_sensitive: bool,
) -> PyResult<Vec<Vec<PyMatch>>> {
    let opts = SearchOptions {
        case_sensitive,
        check_bounds: false,
    };
    let prefix_tree = create_prefix_tree(dictionary, Some(opts)).map_err(map_error_py)?;
    let mut matches_list = Vec::with_capacity(haystacks.len());

    for h in haystacks {
        let matches = prefix_tree.find_text_matches(h).map_err(map_error_py)?;
        matches_list.push(matches.iter().map(PyMatch::from).collect());
    }

    Ok(matches_list)
}

#[pyo3::pymodule]
#[pyo3(name = "ah_search_rs")]
pub mod aho_corasick_search {

    #[pymodule_export]
    use super::{PyMatch, search_in_text, search_in_texts};
}

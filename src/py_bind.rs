use super::trie::*;
use pyo3::exceptions as py_errs;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use unicode_normalization::UnicodeNormalization;

/// Normalize the given string to unicode NFC standard. This is needed to
/// properly check word bounds.
#[pyfunction]
#[pyo3(signature = (input: "str") -> "str")]
fn normalize_string(input: String) -> String {
    input.nfc().collect()
}

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
    pub kw: String,

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
        Self {
            value: m.value().clone(),
            kw: m.keyword().clone(),
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
    #[pyo3(signature = (start: "int", end: "int", value: "str", keyword: "str"))]
    pub fn new(start: usize, end: usize, value: String, keyword: String) -> PyResult<Self> {
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
            kw: keyword,
        })
    }

    pub fn __repr__(&self) -> String {
        format!(
            "PyMatch(start={}, end={}, value=\"{}\", keyword=\"{}\")",
            self.from_char,
            self.to_char,
            self.value.replace('"', "[QUOT]"),
            self.kw.replace('"', "[QUOT]"),
        )
    }
}

/// Prefix tree for performing string searches.
///
/// This is a wrapper around the Rust prefix tree implementation to avoid
/// recomputing the trie unnecessarily when calling from Python. This wrapper
/// is essentially immutable once created.
#[pyclass]
pub struct PyTrie {
    /// The Rust implemented Trie that is wrapped
    trie_inner: TrieRoot,

    /// The list of keywords stored in the trie
    #[pyo3(get)]
    keywords: Vec<String>,
}

#[pymethods]
impl PyTrie {
    /// Instantiate a prefix tree from a mapping of pattern -> keyword
    #[new]
    #[pyo3(signature = (dictionary: "dict[str, str]", case_sensitive=true))]
    pub fn new(dictionary: &Bound<'_, PyDict>, case_sensitive: bool) -> PyResult<Self> {
        let entries = py_dict_to_vector(dictionary)?;
        let opts = Some(SearchOptions {
            case_sensitive,
            check_bounds: false,
        });
        let trie_inner = create_prefix_tree(entries, opts).map_err(map_error_py)?;

        let mut keywords = Vec::new();
        for node in trie_inner.nodes_vec() {
            if let Node::DictNode { keyword, .. } = node {
                keywords.push(keyword.clone());
            }
        }
        Ok(Self {
            trie_inner,
            keywords,
        })
    }

    /// Return the total number of nodes in the prefix tree
    pub fn total_nodes(&self) -> usize {
        self.trie_inner.total_nodes()
    }

    /// Search for occurrences of the defined patterns in the given text
    pub fn search(&self, text: String) -> PyResult<Vec<PyMatch>> {
        let results = self
            .trie_inner
            .find_text_matches(text)
            .map_err(map_error_py)?;

        Ok(results.iter().map(PyMatch::from).collect())
    }

    /// Search for occurrences in a list of texts
    pub fn search_many(&self, texts: Vec<String>) -> PyResult<Vec<Vec<PyMatch>>> {
        let mut results_all = Vec::with_capacity(texts.len());
        for txt in texts {
            results_all.push(self.search(txt)?)
        }
        Ok(results_all)
    }

    pub fn __str__(&self) -> String {
        format!(
            "PyTrie(keywords={:?}, total_nodes={})",
            self.keywords,
            self.total_nodes()
        )
    }
}

/// Convert a dictionary of python str -> str into the vector expected by the Rust API.
fn py_dict_to_vector(dct: &Bound<'_, PyDict>) -> PyResult<Vec<(String, Option<String>)>> {
    let mut items = Vec::with_capacity(dct.len());
    for (k, v) in dct.iter() {
        let key: String = k.extract()?;
        let val: String = v.extract()?;
        items.push((key, Some(val)))
    }
    Ok(items)
}

/// Search for all occurences of strings in the "dictionary" in the given "haystack".
///
/// The dictionary must be a mapping of pattern -> keyword. It is usually better to process
/// texts in batch if you are using the same dictionary, since this requires only one
/// instantiation of the prefix tree.
#[pyfunction]
#[pyo3(signature = (dictionary: "dict[str, str]", haystack: "str", case_sensitive=true) -> "list[PyMatch]")]
fn search_in_text(
    dictionary: &Bound<'_, PyDict>,
    haystack: String,
    case_sensitive: bool,
) -> PyResult<Vec<PyMatch>> {
    let opts = SearchOptions {
        case_sensitive,
        check_bounds: false,
    };
    let prefix_tree =
        create_prefix_tree(py_dict_to_vector(dictionary)?, Some(opts)).map_err(map_error_py)?;
    let matches = prefix_tree
        .find_text_matches(haystack)
        .map_err(map_error_py)?;

    Ok(matches.iter().map(PyMatch::from).collect())
}

/// Search for all occurences of strings in the "dictionary" in the given "haystack" strings.
///
/// The dictionary must be a mapping of pattern -> keyword. If you have multiple texts, this is more efficient
/// than calling "search_in_text" individually on each one, since that would require the prefix tree to
/// be instantiated multiple times.
#[pyfunction]
#[pyo3(signature = (dictionary: "dict[str, str]", haystacks: "list[str]", case_sensitive=true) -> "list[list[str]]")]
fn search_in_texts(
    dictionary: &Bound<'_, PyDict>,
    haystacks: Vec<String>,
    case_sensitive: bool,
) -> PyResult<Vec<Vec<PyMatch>>> {
    let opts = SearchOptions {
        case_sensitive,
        check_bounds: false,
    };
    let prefix_tree =
        create_prefix_tree(py_dict_to_vector(dictionary)?, Some(opts)).map_err(map_error_py)?;
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
    use super::{PyMatch, PyTrie, normalize_string, search_in_text, search_in_texts};
}

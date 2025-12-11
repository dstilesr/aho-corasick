//! Python bindings for the string search library.
//!
//! This module provides wrappers and python bindings to access the package
//! functionality from Python.
use super::multi_proc;
use super::trie::*;
use pyo3::exceptions as py_errs;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashSet;
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
    /// The matching value found in the string
    #[pyo3(get)]
    pub value: String,

    /// The standard keyword associated with the match
    #[pyo3(get)]
    pub kw: String,

    /// Start of the match character range in the input text
    #[pyo3(get)]
    pub from_char: usize,

    ///End of the match character range in the input text
    #[pyo3(get)]
    pub to_char: usize,
}

impl<'a> From<&'a Match<'a>> for PyMatch {
    /// Convert a Match object from the Rust API into a PyMatch object to be used
    /// in the Python API
    fn from(m: &'a Match) -> Self {
        let (start, end) = m.char_range();
        Self {
            value: m.value().to_string(),
            kw: m.keyword().to_string(),
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
    #[pyo3(signature = (from_char: "int", to_char: "int", value: "str", keyword: "str"))]
    pub fn new(from_char: usize, to_char: usize, value: String, keyword: String) -> PyResult<Self> {
        if from_char >= to_char {
            return Err(PyErr::new::<py_errs::PyValueError, _>(
                "Start must precede end of match!",
            ));
        } else if to_char - from_char != value.chars().count() {
            return Err(PyErr::new::<py_errs::PyValueError, _>(
                "Range length must match total characters in value!",
            ));
        }

        Ok(Self {
            from_char,
            to_char,
            value,
            kw: keyword,
        })
    }

    pub fn __repr__(&self) -> String {
        format!(
            "PyMatch(from_char={}, to_char={}, value=\"{}\", kw=\"{}\")",
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
    #[pyo3(signature = (dictionary: "dict[str, str]", case_sensitive=true, check_bounds=false))]
    pub fn new(
        dictionary: &Bound<'_, PyDict>,
        case_sensitive: bool,
        check_bounds: bool,
    ) -> PyResult<Self> {
        let entries = py_dict_to_vector(dictionary)?;
        let opts = Some(SearchOptions {
            case_sensitive,
            check_bounds,
        });
        let trie_inner = create_prefix_tree(entries, opts).map_err(map_error_py)?;

        // Avoid storing duplicates
        let mut keywords = HashSet::with_capacity(dictionary.len());
        for node in trie_inner.nodes_vec() {
            if let Some((_, keyword)) = node.value_keyword() {
                keywords.insert(keyword.to_string());
            }
        }
        Ok(Self {
            trie_inner,
            keywords: keywords.drain().collect(),
        })
    }

    /// Return the total number of nodes in the prefix tree
    pub fn total_nodes(&self) -> usize {
        self.trie_inner.total_nodes()
    }

    /// Search for occurrences of the defined patterns in the given text
    #[pyo3(signature = (text: "str") -> "list[PyMatch]")]
    pub fn search(&self, text: String) -> PyResult<Vec<PyMatch>> {
        let results = self
            .trie_inner
            .find_text_matches(text)
            .map_err(map_error_py)?;

        Ok(results.iter().map(PyMatch::from).collect())
    }

    /// Search for occurrences in a list of texts. Search will be done in parallel across texts.
    #[pyo3(signature = (texts: "list[str]", num_threads: "int | None" = None) -> "list[list[PyMatch]]")]
    pub fn search_many(
        &self,
        texts: Vec<String>,
        num_threads: Option<usize>,
    ) -> PyResult<Vec<Vec<PyMatch>>> {
        let results = multi_proc::parallel_apply(texts, |txt| self.search(txt), num_threads);
        let mut results_out = Vec::with_capacity(results.len());
        for r in results {
            match r {
                Ok(v) => results_out.push(v),
                Err(e) => return Err(e),
            }
        }
        Ok(results_out)
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
#[pyo3(signature = (dictionary: "dict[str, str]", haystack: "str", case_sensitive=true, check_bounds=false) -> "list[PyMatch]")]
fn search_in_text(
    dictionary: &Bound<'_, PyDict>,
    haystack: String,
    case_sensitive: bool,
    check_bounds: bool,
) -> PyResult<Vec<PyMatch>> {
    let opts = SearchOptions {
        case_sensitive,
        check_bounds,
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
#[pyo3(signature = (
    dictionary: "dict[str, str]",
    haystacks: "list[str]",
    case_sensitive=true,
    check_bounds=false,
    num_threads: "int | None" = None) -> "list[list[PyMatch]]")]
fn search_in_texts(
    dictionary: &Bound<'_, PyDict>,
    haystacks: Vec<String>,
    case_sensitive: bool,
    check_bounds: bool,
    num_threads: Option<usize>,
) -> PyResult<Vec<Vec<PyMatch>>> {
    let opts = SearchOptions {
        case_sensitive,
        check_bounds,
    };
    let dct = py_dict_to_vector(dictionary)?;
    let prefix_tree = create_prefix_tree(dct, Some(opts)).map_err(map_error_py)?;

    let matches = multi_proc::parallel_apply(
        haystacks,
        |txt| {
            prefix_tree
                .find_text_matches(txt)
                .map_err(map_error_py)
                .map(|result| result.iter().map(PyMatch::from).collect())
        },
        num_threads,
    );

    let mut matches_list = Vec::with_capacity(matches.len());
    for m in matches {
        match m {
            Err(e) => return Err(e),
            Ok(item) => matches_list.push(item),
        }
    }

    Ok(matches_list)
}

/// The module to expose as importable from Python.
#[pyo3::pymodule]
#[pyo3(name = "ac_search_rs")]
pub mod aho_corasick_search {
    use pyo3::prelude::*;

    /// Module initialization - setup Python logging integration
    #[pymodule_init]
    fn init(_m: &Bound<'_, PyModule>) -> PyResult<()> {
        pyo3_log::init();
        Ok(())
    }

    #[pymodule_export]
    use super::{PyMatch, PyTrie, normalize_string, search_in_text, search_in_texts};
}

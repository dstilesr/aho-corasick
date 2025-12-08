//! # String Search Library
//!
//! ## About
//! This library provides an implementation of the Aho-Corasick algorithm for string searching,
//! along with bindings to build as a python library as well.
#[cfg(feature = "python_bind")]
pub mod py_bind;

pub mod trie;

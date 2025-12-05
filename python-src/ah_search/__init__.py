"""
Aho-Corasick algorithm for efficient text searches - implemented in Rust!
"""

from .ah_search_rs import PyMatch, PyTrie, search_in_text, search_in_texts
from .util import to_dictionary

__all__ = [
    "search_in_text",
    "search_in_texts",
    "to_dictionary",
    "PyMatch",
    "PyTrie",
]

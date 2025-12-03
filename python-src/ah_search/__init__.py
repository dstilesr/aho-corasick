"""
Aho-Corasick algorithm for efficient text searches - implemented in Rust!
"""

from .ah_search_rs import PyMatch, search_in_text, search_in_texts

__all__ = ["search_in_text", "search_in_texts", "PyMatch"]

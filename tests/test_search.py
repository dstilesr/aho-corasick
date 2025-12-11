import pytest
from ac_search import (
    PyMatch,
    PyTrie,
    normalize_string,
    search_in_text,
    search_in_texts,
    to_dictionary,
)


def test_search_invalid():
    """
    Test that an error is raised when trying to search with an invalid dictionary.
    """

    # Empty dictionary
    with pytest.raises(ValueError) as exc_info:
        _ = search_in_text({}, "this is a bit of text")
        assert False, "Searched with empty dictionary"

    assert isinstance(exc_info.value, ValueError)

    # Empty string in dictionary
    with pytest.raises(ValueError) as exc_info:
        _ = search_in_text(
            to_dictionary(["a", "ab", "", "a"]), "this is a bit of text"
        )
        assert False, "Searched with empty string in dictionary"

    assert isinstance(exc_info.value, ValueError)


def test_search_simple():
    """
    Perform some simple search tests with a small dictionary.
    """

    matches: list[PyMatch] = search_in_text(
        to_dictionary(["ab", "abc", "cd", "bcd", "dq"]), "abq cdr qpbcd 12abcd"
    )
    assert len(matches) == 8, "Expected 8 matches"

    matches.sort(key=lambda pm: (pm.from_char, pm.value))

    assert matches[0].value == "ab"
    assert matches[1].value == "cd"
    assert matches[2].value == "bcd"
    assert matches[3].value == "cd"
    assert matches[4].value == "ab"
    assert matches[5].value == "abc"
    assert matches[6].value == "bcd"
    assert matches[7].value == "cd"


def test_search_multiple():
    """
    Test search with a simple dictionary over multiple texts
    """
    texts = [
        "abq cdr qpbcd 12abcd",
        "xy, tre, 1245, mllmkh, aqqsd",
        "432 bcda plodq",
    ]
    dct = to_dictionary(["ab", "abc", "cd", "bcd", "dq"])
    matches: list[list[PyMatch]] = search_in_texts(dct, texts)

    assert len(matches) == len(texts)
    m1, m2, m3 = matches

    assert len(m1) == 8
    assert len(m2) == 0
    assert len(m3) == 3

    for match in m3:
        assert texts[2][match.from_char : match.to_char] == match.value
        assert match.value in dct

    for match in m1:
        assert texts[0][match.from_char : match.to_char] == match.value
        assert match.value in dct


def test_search_case_insensitive():
    """
    Test search with case-insensitive option.
    """
    dictionary = to_dictionary(["abc", "cde", "erx"])
    haystack = "ABCDE eRX cDe"
    matches = search_in_text(dictionary, haystack, case_sensitive=False)

    assert len(matches) == 4, "Expected 4 matches (case insensitive)"


def test_search_case_insensitive_invalid():
    """
    Test searching with case insensitive option on an invalid dictionary.
    """
    dictionary = {"a": "a", "b": "b", "A": "a"}
    haystack = "ABCDE eRX cDe"
    with pytest.raises(ValueError) as exc_info:
        _ = search_in_text(dictionary, haystack, case_sensitive=False)
        assert False, "Exception should have been raised"

    assert isinstance(exc_info.value, ValueError)


def test_search_trie_obj():
    """
    Basic search tests using the PyTrie object.
    """
    dct = {"abc": "Abc", "ab": "Abc", "bcd": "Bc", "pqr": "Pqr"}
    kws = set(dct.values())
    trie = PyTrie(dct)

    hs = "abcd pqr abQd"
    matches = trie.search(hs)

    assert len(matches) == 5
    for m in matches:
        assert m.kw in kws


def test_search_multiple_trie_obj():
    """
    Test search with a simple dictionary over multiple texts, with the trie object
    """
    texts = [
        "abq cdr qpbcd 12abcd",
        "xy, tre, 1245, mllmkh, aqqsd",
        "432 bcda plodq",
    ]
    dct = to_dictionary(["ab", "abc", "cd", "bcd", "dq"])
    trie = PyTrie(dct)
    matches: list[list[PyMatch]] = trie.search_many(texts)

    assert len(matches) == len(texts)
    m1, m2, m3 = matches

    assert len(m1) == 8
    assert len(m2) == 0
    assert len(m3) == 3

    for match in m3:
        assert texts[2][match.from_char : match.to_char] == match.value
        assert match.value in dct

    for match in m1:
        assert texts[0][match.from_char : match.to_char] == match.value
        assert match.value in dct


def test_search_trie_obj_case_insensitive():
    """
    Basic search tests using the PyTrie object with case insensitive option
    """
    trie = PyTrie(to_dictionary(["abc", "cde", "erx"]), case_sensitive=False)

    haystack = "ABCDE eRX cDe"
    matches = trie.search(haystack)

    assert len(matches) == 4, "Expected 4 matches (case insensitive)"


def test_search_word_bounds():
    """
    Test that searching works with word boundary checking enabled.
    """
    dct = {"ab": "ab", "abc": "ab", "épq": "epq", "épqr": "epq"}
    trie = PyTrie(dct, case_sensitive=True, check_bounds=True)

    hs1 = normalize_string("abco zab épqrst! -épqo")
    matches = trie.search(hs1)
    assert len(matches) == 0

    hs2 = normalize_string("abc :ab épqr! -épq")
    matches = trie.search(hs2)
    assert len(matches) == 4

    assert matches[0].kw == "ab" and matches[0].value == "abc"
    assert matches[1].kw == "ab" and matches[1].value == "ab"
    assert matches[2].kw == "epq" and matches[2].value == "épqr"
    assert matches[3].kw == "epq" and matches[3].value == "épq"

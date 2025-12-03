import pytest
from ah_search import PyMatch, search_in_text, search_in_texts


def test_search_invalid():
    """
    Test that an error is raised when trying to search with an invalid dictionary.
    """

    # Empty dictionary
    with pytest.raises(ValueError) as exc_info:
        _ = search_in_text([], "this is a bit of text")
        assert False, "Searched with empty dictionary"

    assert isinstance(exc_info.value, ValueError)

    # Duplicates in dictionary
    with pytest.raises(ValueError) as exc_info:
        _ = search_in_text(["a", "ab", "text", "a"], "this is a bit of text")
        assert False, "Searched with duplicates in dictionary"

    assert isinstance(exc_info.value, ValueError)

    # Empty string in dictionary
    with pytest.raises(ValueError) as exc_info:
        _ = search_in_text(["a", "ab", "", "a"], "this is a bit of text")
        assert False, "Searched with empty string in dictionary"

    assert isinstance(exc_info.value, ValueError)


def test_search_simple():
    """
    Perform some simple search tests with a small dictionary.
    """

    matches: list[PyMatch] = search_in_text(
        ["ab", "abc", "cd", "bcd", "dq"], "abq cdr qpbcd 12abcd"
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
    texts = ["abq cdr qpbcd 12abcd", "xy, tre, 1245, mllmkh, aqqsd", "432 bcda plodq"]
    dct = ["ab", "abc", "cd", "bcd", "dq"]
    matches: list[list[PyMatch]] = search_in_texts(dct, texts)

    assert len(matches) == len(texts)
    m1, m2, m3 = matches

    assert len(m1) == 8
    assert len(m2) == 0
    assert len(m3) == 3

    for match in m3:
        assert texts[2][match.from_char : match.to_char] == match.value

    for match in m1:
        assert texts[0][match.from_char : match.to_char] == match.value

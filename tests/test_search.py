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

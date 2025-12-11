def to_dictionary(words: list[str]) -> dict[str, str]:
    """
    Convert a list of words / patterns into a dictionary to use in search. The
    keywords in this case will be the same as the patterns.
    :param words: List of strings to find in text.
    :return: mapping of word -> word after deduplicating.
    """
    clean = list(set(words))
    return dict(zip(clean, clean))

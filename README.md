# Aho Corasick Library

## Table of Contents
- [About](#about)
  - [Overview](#overview)
  - [Usage Examples](#usage-examples)
    - [Rust Examples](#rust-examples)
    - [Python Examples](#python-examples)
- [Environment Setup](#environment-setup)
- [Installation](#installation)
- [Unit Tests](#unit-tests)

## About

### Overview
This is a string search library that uses the Aho-Corasick algorithm to find occurences of the strings in the given dictionary in a given text. The library can be used in Rust, and it also has Python bindings.

### Usage Examples

#### Rust Examples
```rust
use ah_search_rs::trie;

// Dictionary of strings to find in a text
let dictionary = trie::add_keyword_slot(vec![
    String::from("find"),
    String::from("these"),
    String::from("fun"),
    String::from("words"),
]);

// Override default options - otherwise provide None
let options = Some(trie::SearchOptions{
    // Case insensitive search: convert dictionary and text to lowercase before proceeding
    case_sensitive: false,

    // Do not require word boundaries around the matches
    check_bounds: false,
});

let haystack = String::from("Finding words in these texts is a lot of fun!");

let prefix_tree = trie::create_prefix_tree(dictionary, options).unwrap();
let matches = prefix_tree.find_text_matches(haystack.clone());

for m in matches {
    let (start, end) = m.char_range();
    println!(
        "Found a match - Text: {}, from char: {}, to char: {}",
        m.value(),
        start,
        end,
    );
}
```

#### Python Examples

**Example Using the Search Functions**
```python
import ah_search as acs

dictionary = acs.to_dictionary(["find", "Ding", "these", "fun", "WORDs"])
haystack = "Finding words in these texts is a lot of fun!"

# Case-insensitive search
for match in acs.search_in_text(dictionary, haystack, case_sensitive=False):
    print(
        "Found a match - Text: '%s', from char: %d, to char: %d"
        % (
            match.value,
            match.from_char,
            match.to_char,
        )
    )

# TODO: Other options to be added soon...
```

**Example Using the Trie Object**
```python
import ah_search as acs

dictionary = {
    "find": "Find",
    "ding": "Ding",
    "these": "These",
    "fun": "Fun",
    "WORDs": "Words",
    "words": "Words"
}
trie = acs.PyTrie(dictionary, case_sensitive=True)
print(f"Let's see what we have here: {str(trie)}")

haystack = "Finding words in these texts is a lot of fun!"


for match in trie.search(haystack):
    print(
        "Found a match - Text: '%s' ('%s'), from char: %d, to char: %d"
        % (
            match.value,
            match.kw,
            match.from_char,
            match.to_char,
        )
    )
```

## Environment Setup
To set up your environment for development, you must have the Rust development tools (the Rust compiler and `cargo`) installed on your machine. Next, set up a python virtual environment with the python version you want to build for with `uv`, and install the development dependencies: `uv sync --all-groups`.

## Installation
In order to install the library for development, you can compile a debug build and install it in your Python virtualenv with Maturin, use `maturin develop`. To compile a release build and get a python `.whl` file, you can use one of the following:

- `maturin build --release`: This will output the wheel to the `target/wheels` directory.
- `uv build`: This will output the wheel to the `dist` directory by default.

Once you have the wheel, you can install it in other environments. Note however, that they must have the same Python minor version as the virtual environment where you compiled the wheel, and must run on the same operating system / architecture as the environment where you ran the build.

## Unit Tests
The projects contains unit tests for both Python and Rust. You can run them with `cargo test`and `uv run pytest`, respectively. To run the python tests, you must first have a develop build compiled and installed.

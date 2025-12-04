# Aho Corasick Library

## About
This is a string search library that uses the Aho-Corasick algorithm to find occurences of the strings in the given dictionary in a given text. The library can be used in Rust, and it also has Python bindings.

## Environment Setup
To set up your environment for development, you must have the Rust development tools (the Rust compiler and `cargo`) installed on your machine. Next, set up a python virtual environment with the python version you want to build for with `uv`, and install the development dependencies: `uv sync --all-groups`.

## Installation
In order to install the library for development, you can compile a debug build and install it in your Python virtualenv with Maturin, use `maturin develop`. To compile a release build and get a python `.whl` file, you can use one of the following:

- `maturin build --release`: This will output the wheel to the `target/wheels` directory.
- `uv build`: This will output the wheel to the `dist` directory by default.

Once you have the wheel, you can install it in other environments. Note however, that they must have the same Python minor version as the virtual environment where you compiled the wheel, and must run on the same operating system / architecture as the environment where you ran the build.

## Unit Tests
The projects contains unit tests for both Python and Rust. You can run them with `cargo test`and `uv run pytest`, respectively. To run the python tests, you must first have a develop build compiled and installed.

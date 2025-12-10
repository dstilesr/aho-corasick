//! CLI Application to Run Simple Searches and Save Results as TSV
use ah_search_rs::trie;
use std::{
    fmt::Display,
    fs,
    io::{self, BufRead, Write},
    process,
};

use clap::Parser;

/// Program to run a text search with a given dictionary file and text file.
///
/// Usage Examples
/// ```shell
/// # Simple search
/// search_single -d my-dictionary-file.txt -t my-text-file.txt -o save-here.tsv
///
/// # Custom Search Options
/// search_single -d my-dictionary-file.txt \
///               -t my-text-file.txt \
///               -o save-here.tsv \
///               --case-insensitive \
///               --word-bounds
/// ```
#[derive(Parser, Debug)]
#[command(about, version)]
struct Args {
    /// File containing the dictionary of keywords to find
    ///
    /// This must be a file where each line contains a value and keyword
    /// to match separated by a tab.
    #[arg(short, long)]
    dictionary_file: String,

    /// File containing text to search in.
    #[arg(short, long)]
    text_file: String,

    /// If true, return only matches with words bounds at the start and end.
    #[arg(short, long, default_value_t = false)]
    word_bounds: bool,

    /// If true, make matches case-insensitive
    #[arg(short, long, default_value_t = false)]
    case_insensitive: bool,

    /// The filepath to output the results to
    #[arg(short, long, default_value = "output.tsv")]
    output_file: String,
}

fn err_to_string<T: Display>(err: T) -> String {
    format!("Execution failed. Error: {}", err)
}

/// Read the dictionary of search terms from a file.
///
/// Reads the value / keyword pairs from a given filepath. The file must contain
/// a value and keyword in each line, separated by a tab character. If only the value
/// is provided, the same string will also be used as a keyword.
fn read_dictionary(filepath: &str) -> io::Result<Vec<(String, Option<String>)>> {
    let file = fs::File::open(filepath)?;
    let buf = io::BufReader::new(file);

    let mut elems = Vec::new();
    for line in buf.lines() {
        match line {
            Err(err) => return Err(err),
            Ok(s) => {
                let mut parts = s.split('\t');
                let value = parts.next().unwrap().trim().replace('\t', " ");
                let keyword = parts.next().map(|s| s.trim().replace('\t', " "));
                elems.push((value, keyword));
            }
        }
    }
    Ok(elems)
}

/// Save the encountered matches to the output file.
///
/// Saves the matches in a TSV format at the output path.
fn save_matches(matches: Vec<trie::Match>, filepath: &str) -> io::Result<()> {
    let mut out_file = fs::File::create(filepath)?;
    out_file.write_all(b"start\tend\tvalue\tkeyword\n")?;
    for m in matches {
        let (start, end) = m.char_range();
        let line = format!("{}\t{}\t{}\t{}\n", start, end, m.value(), m.keyword());
        out_file.write_all(line.as_bytes())?;
    }

    Ok(())
}

fn run(args: Args) -> Result<(), String> {
    let dictionary = read_dictionary(&args.dictionary_file).map_err(err_to_string)?;
    let content = fs::read_to_string(&args.text_file).map_err(err_to_string)?;
    let prefix_tree = trie::create_prefix_tree(
        dictionary,
        Some(trie::SearchOptions {
            case_sensitive: !args.case_insensitive,
            check_bounds: args.word_bounds,
        }),
    )
    .map_err(err_to_string)?;

    let matches = prefix_tree
        .find_text_matches(content)
        .map_err(err_to_string)?;

    save_matches(matches, &args.output_file).map_err(err_to_string)?;
    Ok(())
}

fn main() {
    let args = Args::parse();
    if let Err(s) = run(args) {
        println!("{s}");
        process::exit(1);
    }
    process::exit(0);
}

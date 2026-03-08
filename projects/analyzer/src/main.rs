use collections::deque::{BankersDeque, Deque};
use functional::functor::Functor;
use functional::io::IO;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

mod token;

use token::{tokenize, tokens_to_xml, Token};

fn output_path(input: &str) -> String {
    let path = PathBuf::from(input);
    let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    parent
        .join(format!("{}T.xml", stem))
        .to_string_lossy()
        .into_owned()
}

fn process_file(path: String) {
    let output = output_path(&path);
    IO::<String>::read_file(path)
        .flat_map(
            move |content: String| match tokenize::<BankersDeque<Token>>(&content) {
                Ok(tokens) => IO::<String>::write_file(output, tokens_to_xml(&tokens)),
                Err(e) => IO::Error(e),
            },
        )
        .unsafe_run()
        .unwrap_or_else(|e| panic!("Error: {}", e));
}

fn get_source(path_str: &str) -> BankersDeque<String> {
    let path = Path::new(path_str);
    if path.is_dir() {
        fs::read_dir(path)
            .map(|entries| {
                entries.flatten().fold(BankersDeque::empty(), |acc, e| {
                    let p = e.path();
                    if p.extension().and_then(|s| s.to_str()) == Some("jack") {
                        acc.push_back(p.to_string_lossy().into_owned())
                    } else {
                        acc
                    }
                })
            })
            .unwrap_or_else(|_| BankersDeque::empty())
    } else if path.extension().and_then(|s| s.to_str()) == Some("jack") {
        BankersDeque::empty().push_back(path_str.to_string())
    } else {
        BankersDeque::empty()
    }
}

fn run(input: &str) {
    let paths = get_source(input);
    if paths.is_empty() {
        eprintln!("Usage: compiler <file.jack | directory containing .jack files>");
        process::exit(1);
    }
    paths.iter().for_each(|path| process_file((*path).clone()));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file.jack | directory>", args[0]);
        process::exit(1);
    }
    const STACK_SIZE: usize = 16 * 1024 * 1024;
    let input = args[1].clone();
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || run(&input))
        .unwrap()
        .join()
        .unwrap();
}

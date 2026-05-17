use collections::deque::{BankersDeque, Deque};
use collections::Empty;
use functional::functor::Functor;
use functional::io::IO;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use tokenizer::token::{tokenize, Token};

mod ast;
mod codegen;
mod parser;
mod symbol_table;

use codegen::compile_class;
use parser::parse_class;

fn output_path(input: &str) -> String {
    let path = PathBuf::from(input);
    let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    parent
        .join(format!("{}.vm", stem))
        .to_string_lossy()
        .into_owned()
}

fn process_file(path: String) {
    let output = output_path(&path);
    IO::<String>::read_file(path)
        .flat_map(move |content: String| {
            let tokens: BankersDeque<Token> = match tokenize(&content) {
                Ok(tokens) => tokens,
                Err(error) => return IO::Error(error),
            };
            let token_slice: Vec<Token> = tokens
                .iter()
                .map(|token_ref| token_ref.as_ref().clone())
                .collect();
            match parse_class(&token_slice) {
                Ok(class) => {
                    let code = compile_class(&class);
                    let vm_output = code
                        .iter()
                        .map(|line_ref| line_ref.as_ref().clone())
                        .collect::<Vec<String>>()
                        .join("\n");
                    IO::<String>::write_file(output, vm_output)
                }
                Err(error) => IO::Error(error),
            }
        })
        .unsafe_run()
        .unwrap_or_else(|error| panic!("Error: {}", error));
}

fn get_source(path_str: &str) -> BankersDeque<String> {
    let path = Path::new(path_str);
    if path.is_dir() {
        fs::read_dir(path)
            .map(|entries| {
                entries
                    .flatten()
                    .fold(BankersDeque::empty(), |paths, entry| {
                        let entry_path = entry.path();
                        if entry_path.extension().and_then(|ext| ext.to_str()) == Some("jack") {
                            paths.push_back(entry_path.to_string_lossy().into_owned())
                        } else {
                            paths
                        }
                    })
            })
            .unwrap_or_else(|_| BankersDeque::empty())
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("jack") {
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
    paths
        .iter()
        .for_each(|path_ref| process_file(path_ref.as_ref().clone()));
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

use cli::{abort, join_lines, process_file, resolve, run_on_large_stack, sibling_path};
use collections::deque::{BankersDeque, Deque};
use tokenizer::token::{tokenize, Token};

mod ast;
mod codegen;
mod parser;
mod recursive_descent;
mod symbol_table;

use codegen::compile_class;
use parser::parse_class;

const USAGE: &str = "<file.jack | folder containing .jack files>";

fn compile(source: &str) -> Result<String, String> {
    let tokens: BankersDeque<Token> = tokenize(source)?;
    let token_slice: Vec<Token> = tokens.iter().map(|token| (*token).clone()).collect();
    let class = parse_class(&token_slice)?;
    Ok(join_lines(&compile_class(&class)))
}

fn compile_file(path: String) {
    let output = sibling_path(&path, ".vm");
    process_file(path, output, |content: String| compile(&content));
}

fn run(argument: Option<String>) {
    let source = match resolve::<BankersDeque<String>>(argument.as_deref(), "jack") {
        Ok(source) => source,
        Err(error) => abort(error, USAGE),
    };
    source
        .files
        .iter()
        .for_each(|path| compile_file((*path).clone()));
}

fn main() {
    let argument = cli::argument();
    run_on_large_stack(move || run(argument));
}

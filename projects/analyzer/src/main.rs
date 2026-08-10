use cli::{abort, process_file, resolve, run_on_large_stack, sibling_path};
use collections::deque::{BankersDeque, Deque};
use tokenizer::token::{tokenize, tokens_to_xml, Token};

const USAGE: &str = "<file.jack | folder containing .jack files>";

fn analyze(path: String) {
    let output = sibling_path(&path, "T.xml");
    process_file(path, output, |content: String| {
        tokenize::<BankersDeque<Token>>(&content).map(|tokens| tokens_to_xml(&tokens))
    });
}

fn run(argument: Option<String>) {
    let source = match resolve::<BankersDeque<String>>(argument.as_deref(), "jack") {
        Ok(source) => source,
        Err(error) => abort(error, USAGE),
    };
    source
        .files
        .iter()
        .for_each(|path| analyze((*path).clone()));
}

fn main() {
    let argument = cli::argument();
    run_on_large_stack(move || run(argument));
}

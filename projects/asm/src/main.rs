use cli::{abort, join_lines, process_file, resolve_file, run_on_large_stack, sibling_path};
use collections::deque::*;
use collections::hashmap::*;
use instruction::*;
use parser::parser::*;
use translation::*;

mod instruction;
mod translation;

fn preprocess<'a>(lines: &[&str]) -> Result<HashMap<String, u32>, &'a str> {
    let instruction = instruction();
    let symbol_table = symbol_table();

    lines
        .iter()
        .try_fold(
            (0, symbol_table),
            |(line_number, symbol_table), &line| match instruction.parse(line) {
                Ok(("", Instruction::L(symbol))) => {
                    Ok((line_number, symbol_table.insert(symbol, line_number)))
                }
                Ok(("", Instruction::A(_))) | Ok(("", Instruction::C(_, _, _))) => {
                    Ok((line_number + 1, symbol_table))
                }
                Err(_) => Ok((line_number, symbol_table)),
                _ => Err("Filed to preprocess"),
            },
        )
        .map(|(_, symbol_table)| symbol_table)
}

fn assemble<'a, D: Deque<String>>(
    lines: &[&str],
    symbol_table: HashMap<String, u32>,
    code: D,
) -> Result<D, &'a str> {
    let available_address = 16;
    let code = code;
    let instruction = instruction();
    let dest_table = dest_table();
    let comp_table = comp_table();
    let jump_table = jump_table();

    lines
        .iter()
        .try_fold(
            (symbol_table, available_address, code),
            |(symbol_table, available_address, code), &line| match instruction.parse(line) {
                Ok(("", Instruction::L(_))) => Ok((symbol_table, available_address, code)),
                Ok(("", Instruction::A(symbol))) => match symbol.parse::<u32>() {
                    Ok(decimal) => Ok((
                        symbol_table,
                        available_address,
                        code.push_back(format!("{:016b}", decimal)),
                    )),
                    Err(_) => match symbol_table.get(&symbol) {
                        Some(&decimal) => Ok((
                            symbol_table,
                            available_address,
                            code.push_back(format!("{:016b}", decimal)),
                        )),
                        None => Ok((
                            symbol_table.insert(symbol, available_address),
                            available_address + 1,
                            code.push_back(format!("{:016b}", available_address)),
                        )),
                    },
                },
                Ok(("", Instruction::C(dest, comp, jump))) => {
                    let binary = (
                        dest.as_deref().map_or(Some(&"000"), |c| dest_table.get(&c)),
                        comp_table.get(&comp.as_str()),
                        jump.as_deref().map_or(Some(&"000"), |j| jump_table.get(&j)),
                    );

                    match binary {
                        (Some(&dest_bin), Some(&comp_bin), Some(&jump_bin)) => Ok((
                            symbol_table,
                            available_address,
                            code.push_back(format!("111{}{}{}", comp_bin, dest_bin, jump_bin)),
                        )),
                        _ => Err("Failed to assemble"),
                    }
                }
                Err(_) => Ok((symbol_table, available_address, code)),
                _ => Err("Failed to assemble"),
            },
        )
        .map(|(_, _, code)| code)
}

const USAGE: &str = "<file.asm>";

fn translate<D: Deque<String>>(source: &str) -> Result<String, String> {
    let lines = source.lines().collect::<Vec<&str>>();
    let symbol_table = preprocess(&lines).map_err(str::to_string)?;
    let binary = assemble::<D>(&lines, symbol_table, D::empty()).map_err(str::to_string)?;
    Ok(join_lines(&binary))
}

fn run(argument: Option<String>) {
    let source = match resolve_file::<BankersDeque<String>>(argument.as_deref(), "asm") {
        Ok(source) => source,
        Err(error) => abort(error, USAGE),
    };
    let input = (*source
        .files
        .front()
        .expect("a resolved file source has one file"))
    .clone();
    let output = sibling_path(&input, ".hack");
    process_file(input, output, |content: String| {
        translate::<BankersDeque<String>>(&content)
    });
}

fn main() {
    let argument = cli::argument();
    run_on_large_stack(move || run(argument));
}

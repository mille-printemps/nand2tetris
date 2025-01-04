use collections::deque::*;
use collections::hashmap::*;
use functional::functor::*;
use functional::io::*;
use instruction::*;
use parser::parser::*;
use std::env;
use std::path::PathBuf;
use std::process;
use translation::*;

mod instruction;
mod translation;

fn preprocess<'a>(lines: &'a [&'a str]) -> Result<HashMap<String, u32>, &'a str> {
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

fn assemble<'a>(
    lines: &'a [&'a str],
    symbol_table: HashMap<String, u32>,
) -> Result<Deque<String>, &'a str> {
    let available_address = 16;
    let code = Deque::new();
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

fn run<'a>(input: &'a str, output: &'a str) {
    IO::<String>::read_file(&input)
        .flat_map(|content| {
            let assembly = content.lines().collect::<Vec<&str>>();
            preprocess(&assembly).map_or_else(
                |error| IO::Error(error.to_string()),
                |symbol_table| {
                    assemble(&assembly, symbol_table).map_or_else(
                        |error| IO::Error(error.to_string()),
                        |binary| {
                            let lines = binary
                                .iter()
                                .map(|s| s.as_ref().clone())
                                .collect::<Vec<String>>()
                                .join("\n");
                            IO::<String>::write_file(&output, lines)
                        },
                    )
                },
            )
        })
        .unsafe_run()
        .unwrap_or_else(|_| panic!("Failed to write {}", output));
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        eprintln!("Usage: {} <asm file name>", &args[0]);
        process::exit(1);
    }

    let output = format!(
        "{}.hack",
        PathBuf::from(&args[1])
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    );

    const STACK_SIZE: usize = 16 * 1024 * 1024;
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || run(&args[1], &output))
        .unwrap()
        .join()
        .unwrap();
}

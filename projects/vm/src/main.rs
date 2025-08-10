use collections::deque::*;
use command::*;
use functional::functor::*;
use functional::io::*;
use parser::parser::Parser;
use std::env;
use std::path::PathBuf;
use std::process;
use translation::*;

mod command;
mod translation;

fn translate<'a>(lines: &'a [&'a str], file_stem: &'a str) -> Result<Deque<String>, &'a str> {
    let eq_index = 0;
    let gt_index = 0;
    let lt_index = 0;
    let assembly = Deque::new();
    let command = command();

    lines
        .iter()
        .try_fold(
            (eq_index, gt_index, lt_index, assembly),
            |(eq_index, gt_index, lt_index, assembly), &line| match command.parse(line) {
                // Push
                Ok(("", Command::Push(segment, index))) => {
                    if index.parse::<u32>().is_ok() {
                        let assembly_code = match segment.as_str() {
                            "local" => Some(
                                SEGMENT
                                    .replace("{segment}", "LCL")
                                    .replace("{index}", &index),
                            ),
                            "argument" => Some(
                                SEGMENT
                                    .replace("{segment}", "ARG")
                                    .replace("{index}", &index),
                            ),
                            "this" => Some(
                                SEGMENT
                                    .replace("{segment}", "THIS")
                                    .replace("{index}", &index),
                            ),
                            "that" => Some(
                                SEGMENT
                                    .replace("{segment}", "THAT")
                                    .replace("{index}", &index),
                            ),
                            "temp" => Some(TEMP.replace("{index}", &index)),
                            "pointer" => match index.as_str() {
                                "0" => Some(
                                    POINTER.replace("{segment}", "THIS").replace("{index}", "0"),
                                ),
                                "1" => Some(
                                    POINTER.replace("{segment}", "THAT").replace("{index}", "1"),
                                ),
                                _ => None,
                            },
                            "static" => Some(
                                STATIC
                                    .replace("{file}", file_stem)
                                    .replace("{index}", &index),
                            ),
                            "constant" => Some(CONSTANT.replace("{index}", &index)),
                            _ => None,
                        };
                        assembly_code.map_or(Err("Failed to translate"), |code| {
                            Ok((
                                eq_index,
                                gt_index,
                                lt_index,
                                assembly
                                    .push_back(format!("// push {segment} {index}"))
                                    .push_back(code)
                                    .push_back(POST_PUSH.to_string()),
                            ))
                        })
                    } else {
                        Err("Failed to translate")
                    }
                }
                // Pop
                Ok(("", Command::Pop(segment, index))) => {
                    if index.parse::<u32>().is_ok() {
                        let assembly_code = match segment.as_str() {
                            "local" => Some(
                                SEGMENT_ADDRESS
                                    .replace("{segment}", "LCL")
                                    .replace("{index}", &index),
                            ),
                            "argument" => Some(
                                SEGMENT_ADDRESS
                                    .replace("{segment}", "ARG")
                                    .replace("{index}", &index),
                            ),
                            "this" => Some(
                                SEGMENT_ADDRESS
                                    .replace("{segment}", "THIS")
                                    .replace("{index}", &index),
                            ),
                            "that" => Some(
                                SEGMENT_ADDRESS
                                    .replace("{segment}", "THAT")
                                    .replace("{index}", &index),
                            ),
                            "temp" => Some(TEMP_ADDRESS.replace("{index}", &index)),
                            "pointer" => match index.as_str() {
                                "0" => Some(POINTER_ADDRESS.replace("{segment}", "THIS")),
                                "1" => Some(POINTER_ADDRESS.replace("{segment}", "THAT")),
                                _ => None,
                            },
                            "static" => Some(
                                STAIC_ADDRESS
                                    .replace("{file}", file_stem)
                                    .replace("{index}", &index),
                            ),
                            _ => None,
                        };
                        assembly_code.map_or(Err("Failed to translate"), |code| {
                            Ok((
                                eq_index,
                                gt_index,
                                lt_index,
                                assembly
                                    .push_back(format!("// pop {segment} {index}"))
                                    .push_back(PRE_POP.to_string())
                                    .push_back(code)
                                    .push_back(POST_POP.to_string()),
                            ))
                        })
                    } else {
                        Err("Failed to translate")
                    }
                }
                // Operator
                Ok(("", Command::Arithmetic(operator))) => {
                    let assembly_code = match operator.as_str() {
                        "add" => Some(BINARY_COMP.replace("{comp}", "D=D+M")),
                        "sub" => Some(BINARY_COMP.replace("{comp}", "D=M-D")),
                        "and" => Some(BINARY_COMP.replace("{comp}", "D=D&M")),
                        "or" => Some(BINARY_COMP.replace("{comp}", "D=D|M")),
                        "neg" => Some(UNARY_COMP.replace("{comp}", "D=-D")),
                        "not" => Some(UNARY_COMP.replace("{comp}", "D=!D")),
                        "eq" => Some(
                            COMPARISON
                                .replace("{label}", &format!("EQUAL.{eq_index}"))
                                .replace("{jump}", "JEQ"),
                        ),
                        "gt" => Some(
                            COMPARISON
                                .replace("{label}", &format!("GREATERTHAN.{gt_index}"))
                                .replace("{jump}", "JLT"),
                        ),
                        "lt" => Some(
                            COMPARISON
                                .replace("{label}", &format!("LESSTHAN.{lt_index}"))
                                .replace("{jump}", "JGT"),
                        ),
                        _ => None,
                    };
                    assembly_code.map_or(Err("Failed to translate"), |code| {
                        Ok((
                            eq_index + if operator.eq("eq") { 1 } else { 0 },
                            gt_index + if operator.eq("gt") { 1 } else { 0 },
                            lt_index + if operator.eq("lt") { 1 } else { 0 },
                            assembly.push_back(format!("// {operator}")).push_back(code),
                        ))
                    })
                }
                Err(_) => Ok((eq_index, gt_index, lt_index, assembly)),
                _ => Err("Failed to traslate"),
            },
        )
        .map(|(_, _, _, assembly)| assembly)
}

fn run(input: &str) {
    let file_stem = PathBuf::from(&input)
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .into_owned();

    let output = format!("{}.asm", file_stem);

    IO::<String>::read_file(input)
        .flat_map(|content| {
            let commands = content.lines().collect::<Vec<&str>>();
            translate(&commands, &file_stem).map_or_else(
                |error| IO::Error(error.to_string()),
                |assembly| {
                    let lines = assembly
                        .iter()
                        .map(|s| s.as_ref().clone())
                        .collect::<Vec<String>>()
                        .join("\n");
                    IO::<String>::write_file(&output, lines)
                },
            )
        })
        .unsafe_run()
        .unwrap_or_else(|_| panic!("Failed to write {}", output))
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        eprintln!("Usage: {} <vm file name>", &args[0]);
        process::exit(1);
    }

    const STACK_SIZE: usize = 16 * 1024 * 1024;
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || run(&args[1]))
        .unwrap()
        .join()
        .unwrap();
}

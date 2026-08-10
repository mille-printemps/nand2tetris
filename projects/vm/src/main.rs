use cli::{abort, fail, is_blank, join_lines, resolve, run_on_large_stack, SourceKind};
use collections::catdeque::CatenableDeque;
use collections::deque::*;
use collections::Empty;
use command::*;
use functional::functor::*;
use functional::io::*;
use parser::parser::Parser;
use std::path::PathBuf;
use translation::*;

mod command;
mod translation;

fn translate<'a>(
    lines: &'a [&'a str],
    file_stem: &'a str,
    file_index: usize,
) -> Result<CatenableDeque<String>, String> {
    const SENTINEL: &str = "\0";

    let command = command();

    let assembly = if file_index == 0 {
        CatenableDeque::<String>::empty()
            .push_back("// bootstrap".to_string())
            .push_back(BOOTSTRAP.to_string())
    } else {
        CatenableDeque::<String>::empty()
    };

    let eq_index = 0;
    let gt_index = 0;
    let lt_index = 0;

    let callee_index = 0;
    let caller_stack = if file_index == 0 {
        BankersDeque::<String>::empty().push_back("CALLER".to_string())
    } else {
        BankersDeque::<String>::empty()
    };

    let prefix = if file_index == 0 {
        Some("call Sys.init 0")
    } else {
        None
    };

    prefix
        .into_iter()
        .chain(lines.iter().copied())
        .chain(std::iter::once(SENTINEL))
        .try_fold(
            (
                assembly,
                eq_index,
                gt_index,
                lt_index,
                callee_index,
                caller_stack,
                None,
            ),
            |(assembly, eq_index, gt_index, lt_index, callee_index, caller_stack, pending),
             incoming| {
                match pending {
                    Some(current) if current != SENTINEL => {
                        match command.parse(current) {
                            // Push
                            Ok(("", Command::Push(segment, index))) => index.parse::<u32>().map_or(
                                Err(format!("failed to translate: {}", current)),
                                |_| {
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
                                                POINTER
                                                    .replace("{segment}", "THIS")
                                                    .replace("{index}", "0"),
                                            ),
                                            "1" => Some(
                                                POINTER
                                                    .replace("{segment}", "THAT")
                                                    .replace("{index}", "1"),
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
                                    assembly_code.map_or(
                                        Err(format!("failed to translate: {}", current)),
                                        |code| {
                                            Ok((
                                                assembly
                                                    .push_back(format!("// push {segment} {index}"))
                                                    .push_back(code)
                                                    .push_back(POST_PUSH.to_string()),
                                                eq_index,
                                                gt_index,
                                                lt_index,
                                                callee_index,
                                                caller_stack,
                                                Some(incoming),
                                            ))
                                        },
                                    )
                                },
                            ),
                            // Pop
                            Ok(("", Command::Pop(segment, index))) => index.parse::<u32>().map_or(
                                Err(format!("failed to translate: {}", current)),
                                |_| {
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
                                            "0" => {
                                                Some(POINTER_ADDRESS.replace("{segment}", "THIS"))
                                            }
                                            "1" => {
                                                Some(POINTER_ADDRESS.replace("{segment}", "THAT"))
                                            }
                                            _ => None,
                                        },
                                        "static" => Some(
                                            STAIC_ADDRESS
                                                .replace("{file}", file_stem)
                                                .replace("{index}", &index),
                                        ),
                                        _ => None,
                                    };
                                    assembly_code.map_or(
                                        Err(format!("failed to translate: {}", current)),
                                        |code| {
                                            Ok((
                                                assembly
                                                    .push_back(format!("// pop {segment} {index}"))
                                                    .push_back(PRE_POP.to_string())
                                                    .push_back(code)
                                                    .push_back(POST_POP.to_string()),
                                                eq_index,
                                                gt_index,
                                                lt_index,
                                                callee_index,
                                                caller_stack,
                                                Some(incoming),
                                            ))
                                        },
                                    )
                                },
                            ),
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
                                assembly_code.map_or(
                                    Err(format!("failed to translate: {}", current)),
                                    |code| {
                                        Ok((
                                            assembly
                                                .push_back(format!("// {operator}"))
                                                .push_back(code),
                                            eq_index + if operator.eq("eq") { 1 } else { 0 },
                                            gt_index + if operator.eq("gt") { 1 } else { 0 },
                                            lt_index + if operator.eq("lt") { 1 } else { 0 },
                                            callee_index,
                                            caller_stack,
                                            Some(incoming),
                                        ))
                                    },
                                )
                            }
                            // Label
                            Ok(("", Command::Label(label))) => {
                                let assembly_code = if caller_stack.is_empty() {
                                    Some(format!("({label})"))
                                } else {
                                    caller_stack
                                        .back()
                                        .map(|function| format!("({function}${label})"))
                                };
                                assembly_code.map_or(
                                    Err(format!("failed to translate: {}", current)),
                                    |code| {
                                        Ok((
                                            assembly
                                                .push_back(format!("// {label}"))
                                                .push_back(code),
                                            eq_index,
                                            gt_index,
                                            lt_index,
                                            callee_index,
                                            caller_stack,
                                            Some(incoming),
                                        ))
                                    },
                                )
                            }
                            // Goto
                            Ok(("", Command::Goto(label))) => {
                                let assembly_code = if caller_stack.is_empty() {
                                    Some(format!("@{label}"))
                                } else {
                                    caller_stack
                                        .back()
                                        .map(|function| format!("@{function}${label}"))
                                };
                                assembly_code.map_or(
                                    Err(format!("failed to translate: {}", current)),
                                    |code| {
                                        Ok((
                                            assembly
                                                .push_back(format!("// goto {label}"))
                                                .push_back(code)
                                                .push_back("0;JMP".to_string()),
                                            eq_index,
                                            gt_index,
                                            lt_index,
                                            callee_index,
                                            caller_stack,
                                            Some(incoming),
                                        ))
                                    },
                                )
                            }
                            // If-Goto
                            Ok(("", Command::IfGoto(label))) => {
                                let assembly_code = if caller_stack.is_empty() {
                                    Some(
                                        IF_GOTO
                                            .replace("{dontgoto}", "DONTGOTO")
                                            .replace("{label}", &label),
                                    )
                                } else {
                                    caller_stack.back().map(|function| {
                                        IF_GOTO
                                            .replace("{dontgoto}", &format!("{function}.DONTGOTO"))
                                            .replace("{label}", &format!("{function}${label}"))
                                    })
                                };
                                assembly_code.map_or(
                                    Err(format!("failed to translate: {}", current)),
                                    |code| {
                                        Ok((
                                            assembly
                                                .push_back(format!("// if-goto {label}"))
                                                .push_back(code),
                                            eq_index,
                                            gt_index,
                                            lt_index,
                                            callee_index,
                                            caller_stack,
                                            Some(incoming),
                                        ))
                                    },
                                )
                            }
                            // Function
                            Ok(("", Command::Function(caller, nvers))) => nvers
                                .parse::<usize>()
                                .map_or(Err(format!("failed to translate: {}", current)), |vers| {
                                    Ok((
                                        assembly
                                            .push_back(format!("// function {caller} {nvers}"))
                                            .push_back(format!("({caller})"))
                                            .push_back(
                                                format!("{FUNCTION}\n")
                                                    .repeat(vers)
                                                    .trim_end_matches("\n")
                                                    .to_owned(),
                                            ),
                                        eq_index,
                                        gt_index,
                                        lt_index,
                                        callee_index,
                                        caller_stack.push_back(caller.clone()),
                                        Some(incoming),
                                    ))
                                }),
                            // Call
                            Ok(("", Command::Call(callee, nargs))) => {
                                match (nargs.parse::<usize>(), caller_stack.back()) {
                                    (Ok(_), Some(caller)) => Ok((
                                        assembly
                                            .push_back(format!("// call {callee} {nargs}"))
                                            .push_back(
                                                CALL.replace("{caller}", &caller)
                                                    .replace(
                                                        "{callee_index}",
                                                        &callee_index.to_string(),
                                                    )
                                                    .replace("{nargs}", &nargs)
                                                    .replace("{callee}", &callee),
                                            ),
                                        eq_index,
                                        gt_index,
                                        lt_index,
                                        callee_index + 1,
                                        caller_stack,
                                        Some(incoming),
                                    )),
                                    _ => Err(format!("failed to translate: {}", current)),
                                }
                            }
                            // Return
                            Ok(("", Command::Return)) => {
                                // if there is a label for a jump after return, keep the caller stack as is, since the process is still in a function
                                if matches!(command.parse(incoming), Ok(("", Command::Label(_)))) {
                                    Ok(caller_stack)
                                // otherwise, pop the caller stack and continue
                                } else {
                                    caller_stack.pop_back().map_or(
                                        Err(format!("return without a matching call: {}", current)),
                                        |(_, next_caller_stack)| Ok(next_caller_stack),
                                    )
                                }
                                .and_then(|next_caller_stack| {
                                    Ok((
                                        assembly
                                            .push_back("// return".to_string())
                                            .push_back(RETURN.to_string()),
                                        eq_index,
                                        gt_index,
                                        lt_index,
                                        callee_index,
                                        next_caller_stack,
                                        Some(incoming),
                                    ))
                                })
                            }
                            Err(_) if is_blank(current) => Ok((
                                assembly,
                                eq_index,
                                gt_index,
                                lt_index,
                                callee_index,
                                caller_stack,
                                Some(incoming),
                            )),
                            Err(_) => Err(format!("could not parse command: {}", current)),
                            _ => Err(format!("unrecognized command: {}", current)),
                        }
                    }
                    _ => Ok((
                        assembly,
                        eq_index,
                        gt_index,
                        lt_index,
                        callee_index,
                        caller_stack,
                        Some(incoming),
                    )),
                }
            },
        )
        .map(|(assembly, _, _, _, _, _, _)| assembly.push_back("\n".to_string()))
}

const USAGE: &str = "<file.vm | folder containing .vm files>";

fn run(argument: Option<String>) {
    let source = match resolve::<BankersDeque<String>>(argument.as_deref(), "vm") {
        Ok(source) => source,
        Err(error) => abort(error, USAGE),
    };

    // A folder is a whole program and gets bootstrap code; a lone file is a
    // fragment and does not. Shifting the index past 0 suppresses it.
    let offset = if source.kind == SourceKind::Folder {
        0
    } else {
        1
    };
    let output = source.combined_output("asm");

    // Fold over the file paths, accumulating the results into an IO<CatenableDeque>
    source
        .files
        .iter()
        .enumerate()
        .fold(
            IO::Return(CatenableDeque::<String>::empty()),
            |acc, (file_index, path)| {
                let path_string = path.to_string();
                let file_stem = PathBuf::from(path.as_str())
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Main".to_string());

                let read_context = path_string.clone();
                let translate_context = path_string.clone();

                IO::<String>::read_file(path_string)
                    .map_err(move |error| format!("{}: {}", read_context, error))
                    .flat_map(move |content| {
                        let commands = content.lines().collect::<Vec<&str>>();
                        translate(&commands, &file_stem, file_index + offset).map_or_else(
                            move |error| IO::Error(format!("{}: {}", translate_context, error)),
                            |assembly| acc.map(move |deque| deque.append(&assembly)),
                        )
                    })
            },
        )
        // After processing all files, write a single combined result
        .flat_map(|assembly| {
            let write_context = output.clone();
            IO::<String>::write_file(output, join_lines(&assembly))
                .map_err(move |error| format!("{}: {}", write_context, error))
        })
        .unsafe_run()
        .unwrap_or_else(|error| fail(&error));
}

fn main() {
    let argument = cli::argument();
    run_on_large_stack(move || run(argument));
}

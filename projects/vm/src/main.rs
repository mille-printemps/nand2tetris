use collections::catdeque::CatenableDeque;
use collections::deque::*;
use command::*;
use functional::functor::*;
use functional::io::*;
use parser::parser::Parser;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use translation::*;

mod command;
mod translation;

fn translate<'a>(
    lines: &'a [&'a str],
    file_stem: &'a str,
    file_index: usize,
) -> Result<CatenableDeque<String>, &'a str> {
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
                            Ok(("", Command::Push(segment, index))) => {
                                index
                                    .parse::<u32>()
                                    .map_or(Err("Failed to translate"), |_| {
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
                                        assembly_code.map_or(Err("Failed to translate"), |code| {
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
                                        })
                                    })
                            }
                            // Pop
                            Ok(("", Command::Pop(segment, index))) => {
                                index
                                    .parse::<u32>()
                                    .map_or(Err("Failed to translante"), |_| {
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
                                                "0" => Some(
                                                    POINTER_ADDRESS.replace("{segment}", "THIS"),
                                                ),
                                                "1" => Some(
                                                    POINTER_ADDRESS.replace("{segment}", "THAT"),
                                                ),
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
                                        })
                                    })
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
                                })
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
                                assembly_code.map_or(Err("Failed to translate"), |code| {
                                    Ok((
                                        assembly.push_back(format!("// {label}")).push_back(code),
                                        eq_index,
                                        gt_index,
                                        lt_index,
                                        callee_index,
                                        caller_stack,
                                        Some(incoming),
                                    ))
                                })
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
                                assembly_code.map_or(Err("Failed to translate"), |code| {
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
                                })
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
                                assembly_code.map_or(Err("Failed to translate"), |code| {
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
                                })
                            }
                            // Function
                            Ok(("", Command::Function(caller, nvers))) => nvers
                                .parse::<usize>()
                                .map_or(Err("Failed to translate"), |vers| {
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
                                    _ => Err("Failed to translate"),
                                }
                            }
                            // Return
                            Ok(("", Command::Return)) => {
                                // if there is a label for a jump after return, keep the caller stack as is, since the process is still in a function
                                if matches!(command.parse(incoming), Ok(("", Command::Label(_)))) {
                                    Ok(caller_stack)
                                // otherwise, pop the caller stack and continue
                                } else {
                                    caller_stack
                                        .pop_back()
                                        .map_or(Err(""), |(_, next_caller_stack)| {
                                            Ok(next_caller_stack)
                                        })
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
                            Err(_) => Ok((
                                assembly,
                                eq_index,
                                gt_index,
                                lt_index,
                                callee_index,
                                caller_stack,
                                Some(incoming),
                            )),
                            _ => Err("Failed to translate"),
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum SourceType {
    File,
    Directory,
}

struct Source<D: Deque<String>> {
    source_type: SourceType,
    dir: String,
    file_paths: D,
}

fn get_file_paths<D: Deque<String>>(dir: &str, ext: &str, paths: D) -> D {
    fs::read_dir(dir).map_or(D::empty(), |entries| {
        entries.flatten().fold(paths, |paths, entry| {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some(ext) {
                return paths.push_back(path.to_string_lossy().into_owned());
            }
            paths
        })
    })
}

fn get_file_path<D: Deque<String>>(path: &Path, ext: &str, paths: D) -> D {
    if path.extension().and_then(|s| s.to_str()) == Some(ext) {
        return paths.push_back(path.to_string_lossy().into_owned());
    }
    paths
}

fn get_source<D: Deque<String>>(input: Option<&str>, ext: &str, paths: D) -> Source<D> {
    match input {
        None => Source {
            source_type: SourceType::Directory,
            dir: "./".to_string(),
            file_paths: get_file_paths(".", ext, paths),
        },
        Some(path_str) => {
            let path = PathBuf::from(path_str);
            if path.is_dir() {
                Source {
                    source_type: SourceType::Directory,
                    dir: path_str.to_string(),
                    file_paths: get_file_paths(path_str, ext, paths),
                }
            } else {
                Source {
                    source_type: SourceType::File,
                    dir: path.parent().unwrap().to_string_lossy().into_owned(),
                    file_paths: get_file_path(&path, ext, paths),
                }
            }
        }
    }
}

fn run(input: Option<&str>) {
    let paths = BankersDeque::<String>::empty();
    let source = get_source(input, "vm", paths);
    let offset = if source.source_type == SourceType::Directory { 0 } else { 1 };

    if source.file_paths.is_empty() {
        eprintln!("Usage: vm <vm file name|dir name where vm files reside>");
        process::exit(1);
    }

    let output = if source.dir == "./" {
        let stem = if source.file_paths.len() == 1 {
            PathBuf::from(source.file_paths.front().unwrap().as_str())
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        } else {
            "Main".to_string()
        };
        format!("{}.asm", stem)
    } else {
        let stem = PathBuf::from(source.dir.clone())
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        format!("{}/{}.asm", source.dir, stem)
    };

    // Fold over the file paths, accumulating the results into an IO<CatenableDeque>
    source
        .file_paths
        .iter()
        .enumerate()
        .fold(
            IO::Return(CatenableDeque::<String>::empty()),
            |acc, (file_index, path)| {
                let file_stem = PathBuf::from(path.as_str())
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                IO::<String>::read_file(path.to_string()).flat_map(move |content| {
                    let commands = content.lines().collect::<Vec<&str>>();
                    translate(&commands, &file_stem, file_index + offset).map_or_else(
                        |error| IO::Error(error.to_string()),
                        |assembly| acc.map(move |deque| deque.append(&assembly)),
                    )
                })
            },
        )
        // After processing all files, write a single combined result
        .flat_map(|assembly| {
            let lines = assembly
                .iter()
                .map(|s| s.as_ref().clone())
                .collect::<Vec<String>>()
                .join("\n");
            IO::<String>::write_file(output, lines)
        })
        .unsafe_run()
        .unwrap_or_else(|e| panic!("Failed to process files: {}", e));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    const STACK_SIZE: usize = 16 * 1024 * 1024;
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || run(if 1 < args.len() { Some(&args[1]) } else { None }))
        .unwrap()
        .join()
        .unwrap();
}

use parser::parser::*;

#[derive(PartialEq, Debug)]
pub enum Command {
    Push(String, String),
    Pop(String, String),
    Arithmetic(String),
    Label(String),
    IfGoto(String),
    Goto(String),
    Function(String, String),
    Call(String, String),
    Return,
    Error(String),
}

// Returns a parser for a command
pub fn command<'a>() -> impl Parser<'a, Command> {
    whitespace_wrap(right(
        simple_comment(),
        either(
            either(command_and_target_variable(), command_and_target()),
            command_only(),
        ),
    ))
}

// Returns a parser for a command with its target and variable
fn command_and_target_variable<'a>() -> impl Parser<'a, Command> {
    pair(
        identifier,
        pair(right(space1(), identifier), right(space1(), number)),
    )
    .map(|(command, (target, variable))| match command.as_str() {
        "push" => Command::Push(target, variable),
        "pop" => Command::Pop(target, variable),
        "function" => Command::Function(target, variable),
        "call" => Command::Call(target, variable),
        _ => Command::Error(command),
    })
}

// Returns a parser for a command with its target
fn command_and_target<'a>() -> impl Parser<'a, Command> {
    pair(identifier, right(space1(), identifier)).map(|(command, target)| match command.as_str() {
        "label" => Command::Label(target),
        "if-goto" => Command::IfGoto(target),
        "goto" => Command::Goto(target),
        _ => Command::Error(command),
    })
}

// Returns a parser for a command as itself
fn command_only<'a>() -> impl Parser<'a, Command> {
    identifier.map(|command| match command.as_str() {
        "add" | "sub" | "neg" | "eq" | "gt" | "lt" | "and" | "or" | "not" => {
            Command::Arithmetic(command)
        }
        "return" => Command::Return,
        _ => Command::Error(command),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_push_pop_command() {
        assert_eq!(
            Ok(("", Command::Push("constant".to_string(), "10".to_string()))),
            command_and_target_variable().parse("push constant 10")
        );
        assert_eq!(
            Ok(("", Command::Pop("local".to_string(), "0".to_string()))),
            command_and_target_variable().parse("pop local 0")
        );
    }

    #[test]
    fn parse_arithmetic_command() {
        assert_eq!(
            Ok(("", Command::Arithmetic("add".to_string()))),
            command_only().parse("add")
        );
    }

    #[test]
    fn parse_branching_command() {
        assert_eq!(
            Ok(("", Command::Label("LOOP_START".to_string()))),
            command_and_target().parse("label LOOP_START")
        );
        assert_eq!(
            Ok(("", Command::IfGoto("LOOP_START".to_string()))),
            command_and_target().parse("if-goto LOOP_START")
        );
        assert_eq!(
            Ok(("", Command::Goto("LOOP_START".to_string()))),
            command_and_target().parse("goto LOOP_START")
        );
    }

    #[test]
    fn parse_function_call_command() {
        assert_eq!(
            Ok((
                "",
                Command::Function("Class.test".to_string(), "2".to_string())
            )),
            command_and_target_variable().parse("function Class.test 2")
        );
        assert_eq!(
            Ok(("", Command::Call("Class.test".to_string(), "2".to_string()))),
            command_and_target_variable().parse("call Class.test 2")
        );
    }

    #[test]
    fn parse_return_command() {
        assert_eq!(Ok(("", Command::Return)), command_only().parse("return"));
    }

    #[test]
    fn parse_command() {
        assert_eq!(Err(""), command().parse("   // Comment"));
        assert_eq!(
            Ok(("", Command::Push("constant".to_string(), "21".to_string()))),
            command().parse("   push constant 21")
        );
        assert_eq!(
            Ok(("", Command::Push("constant".to_string(), "21".to_string()))),
            command().parse("   push constant 21 // Comment")
        );
        assert_eq!(
            Ok(("", Command::Arithmetic("add".to_string()))),
            command().parse("  add")
        );
        assert_eq!(
            Ok(("", Command::Label("LOOP_START".to_string()))),
            command().parse("  label LOOP_START")
        );
        assert_eq!(
            Ok(("", Command::IfGoto("LOOP_START".to_string()))),
            command().parse("  if-goto LOOP_START")
        );
        assert_eq!(
            Ok(("", Command::Goto("LOOP_START".to_string()))),
            command().parse("  goto LOOP_START")
        );
        assert_eq!(
            Ok((
                "",
                Command::Function("Class.test".to_string(), "2".to_string())
            )),
            command().parse("  function Class.test 2")
        );
        assert_eq!(
            Ok(("", Command::Call("Class.test".to_string(), "2".to_string()))),
            command().parse("  call Class.test 2")
        );
        assert_eq!(Ok(("", Command::Return)), command().parse("  return"));
    }
}

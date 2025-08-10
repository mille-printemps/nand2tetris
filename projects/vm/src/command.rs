use parser::parser::*;

#[derive(PartialEq, Debug)]
pub enum Command {
    Push(String, String),
    Pop(String, String),
    Arithmetic(String),
    Error(String),
}

pub fn command<'a>() -> impl Parser<'a, Command> {
    whitespace_wrap(right(
        simple_comment(),
        either(push_or_pop_command(), arithmetic_command()),
    ))
}

fn push_or_pop_command<'a>() -> impl Parser<'a, Command> {
    pair(
        identifier,
        pair(right(space1(), identifier), right(space1(), number)),
    )
    .map(|(command, (segment, index))| match command.as_str() {
        "push" => Command::Push(segment, index),
        "pop" => Command::Pop(segment, index),
        _ => Command::Error(command),
    })
}

fn arithmetic_command<'a>() -> impl Parser<'a, Command> {
    identifier.map(Command::Arithmetic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_push_or_pop_command() {
        assert_eq!(
            Ok(("", Command::Push("constant".to_string(), "10".to_string()))),
            push_or_pop_command().parse("push constant 10")
        );
        assert_eq!(
            Ok(("", Command::Pop("local".to_string(), "0".to_string()))),
            push_or_pop_command().parse("pop local 0")
        );
    }

    #[test]
    fn parse_arithmetic_command() {
        assert_eq!(
            Ok(("", Command::Arithmetic("add".to_string()))),
            arithmetic_command().parse("add")
        );
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
    }
}

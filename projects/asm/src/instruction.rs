use parser::parser::*;

#[derive(PartialEq, Debug)]
pub enum Instruction {
    A(String),
    C(Option<String>, String, Option<String>),
    L(String),
}

pub fn instruction<'a>() -> impl Parser<'a, Instruction> {
    whitespace_wrap(right(
        simple_comment(),
        either(l_instruction(), either(a_instruction(), c_instruction())),
    ))
}

fn a_instruction<'a>() -> impl Parser<'a, Instruction> {
    right(match_literal("@"), either(number, identifier)).map(Instruction::A)
}

fn c_instruction<'a>() -> impl Parser<'a, Instruction> {
    fn comp<'a>() -> impl Parser<'a, String> {
        one_or_more(pred(any_char, |c| {
            c.is_alphanumeric() || ['+', '-', '&', '|', '!'].contains(c)
        }))
        .map(|chars| chars.into_iter().collect())
    }

    pair(
        either(
            assignment_pair(comp()),
            map(comp(), |comp| ("".to_string(), comp)),
        ),
        zero_or_more(right(match_literal(";"), identifier)).map(|strs| strs.concat()),
    )
    .map(|((dest, comp), jump)| {
        Instruction::C(
            Some(dest).filter(|d| !d.is_empty()),
            comp,
            Some(jump).filter(|j| !j.is_empty()),
        )
    })
}

fn l_instruction<'a>() -> impl Parser<'a, Instruction> {
    enclosed_string(Enclosure::Parentheses).map(Instruction::L)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_a_instruction() {
        assert_eq!(
            Ok(("", Instruction::A("abcd".to_string()))),
            a_instruction().parse("@abcd")
        );
        assert_eq!(
            Ok(("", Instruction::A("ABC_EFG".to_string()))),
            a_instruction().parse("@ABC_EFG")
        );
        assert_eq!(
            Ok(("", Instruction::A("123".to_string()))),
            a_instruction().parse("@123")
        );
    }

    #[test]
    fn parse_c_instruction() {
        assert_eq!(
            Ok((
                "",
                Instruction::C(Some("D".to_string()), "M".to_string(), None)
            )),
            c_instruction().parse("D=M")
        );
        assert_eq!(
            Ok((
                "",
                Instruction::C(Some("M".to_string()), "-1".to_string(), None)
            )),
            c_instruction().parse("M=-1")
        );
        assert_eq!(
            Ok((
                "",
                Instruction::C(
                    Some("D".to_string()),
                    "M".to_string(),
                    Some("JMP".to_string())
                )
            )),
            c_instruction().parse("D=M;JMP")
        );
        assert_eq!(
            Ok((
                "",
                Instruction::C(None, "D".to_string(), Some("JMP".to_string()))
            )),
            c_instruction().parse("D;JMP")
        );
        assert_eq!(
            Ok((
                "",
                Instruction::C(None, "0".to_string(), Some("JMP".to_string()))
            )),
            c_instruction().parse("0;JMP")
        );
        assert_eq!(
            Ok((
                "",
                Instruction::C(Some("D".to_string()), "D+M".to_string(), None)
            )),
            c_instruction().parse("D=D+M")
        );
        assert_eq!(
            Ok((
                "",
                Instruction::C(
                    Some("D".to_string()),
                    "D+M".to_string(),
                    Some("JMP".to_string())
                ),
            )),
            c_instruction().parse("D=D+M;JMP")
        );
    }

    #[test]
    fn parse_l_instruction() {
        assert_eq!(
            Ok(("", Instruction::L("xxx".to_string()))),
            l_instruction().parse("(xxx)")
        );
    }

    #[test]
    fn parse_instruction() {
        assert_eq!(Err(""), instruction().parse("   // Comment"));
        assert_eq!(
            Ok(("", Instruction::A("xxx".to_string()))),
            instruction().parse("  @xxx")
        );
        assert_eq!(
            Ok(("", Instruction::A("xxx".to_string()))),
            instruction().parse("  @xxx // Comment")
        );
        assert_eq!(
            Ok(("", Instruction::L("xxx".to_string()))),
            instruction().parse("   (xxx) // Comment")
        );
        assert_eq!(
            Ok((
                "",
                Instruction::C(
                    Some("D".to_string()),
                    "M".to_string(),
                    Some("JMP".to_string())
                )
            )),
            instruction().parse("   D=M;JMP//Comment")
        )
    }
}

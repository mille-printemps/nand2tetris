use collections::deque::Deque;
use parser::parser::*;

const KEYWORDS: &[&str] = &[
    "class",
    "constructor",
    "function",
    "method",
    "field",
    "static",
    "var",
    "int",
    "char",
    "boolean",
    "void",
    "true",
    "false",
    "null",
    "this",
    "let",
    "do",
    "if",
    "else",
    "while",
    "return",
];

const SYMBOLS: &str = "{}()[].,;+-*/&|<>=~";
const ERROR_PREVIEW_LEN: usize = 20;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Keyword(String),
    Symbol(char),
    IntegerConstant(u16),
    StringConstant(String),
    Identifier(String),
}

// Skips any leading combination of whitespace, `//` line comments, and `/* */` block comments.
pub fn skip(input: &str) -> &str {
    match zero_or_more(either(
        either(line_comment(), block_comment()),
        space1().map(drop),
    ))
    .parse(input)
    {
        Ok((remaining, _)) => remaining,
        Err(_) => input,
    }
}

// Parses a Jack identifier: `[a-zA-Z_][a-zA-Z0-9_]*`.
fn jack_identifier(input: &str) -> ParseResult<String> {
    pair(
        pred(any_char, |c| c.is_alphabetic() || *c == '_'),
        zero_or_more(pred(any_char, |c| c.is_alphanumeric() || *c == '_')),
    )
    .map(|(first, rest)| std::iter::once(first).chain(rest).collect())
    .parse(input)
}

fn keyword_or_identifier(input: &str) -> ParseResult<Token> {
    jack_identifier
        .map(|s| {
            if KEYWORDS.contains(&s.as_str()) {
                Token::Keyword(s)
            } else {
                Token::Identifier(s)
            }
        })
        .parse(input)
}

// TODO should the tokenizer enforce the 32767 limit on integer constants?
fn integer_constant(input: &str) -> ParseResult<Token> {
    let (remaining, s) = one_or_more(pred(any_char, |c| c.is_ascii_digit()))
        .map(|chars| chars.into_iter().collect::<String>())
        .parse(input)?;
    s.parse::<u16>()
        .map(|n| (remaining, Token::IntegerConstant(n)))
        .map_err(|_| input)
}

fn string_constant(input: &str) -> ParseResult<Token> {
    quoted_string().map(Token::StringConstant).parse(input)
}

fn symbol(input: &str) -> ParseResult<Token> {
    pred(any_char, |c| SYMBOLS.contains(*c))
        .map(Token::Symbol)
        .parse(input)
}

pub fn token(input: &str) -> ParseResult<Token> {
    either(
        either(keyword_or_identifier, integer_constant),
        either(string_constant, symbol),
    )
    .parse(input)
}

fn tokenize_impl<D: Deque<Token>>(remaining: &str, acc: D) -> Result<D, String> {
    if remaining.is_empty() {
        Ok(acc)
    } else {
        match token(remaining) {
            Ok((next, token)) => tokenize_impl(skip(next), acc.push_back(token)),
            Err(s) => {
                let preview = s.chars().take(ERROR_PREVIEW_LEN).collect::<String>();
                Err(format!("Unexpected input: {:?}", preview))
            }
        }
    }
}

pub fn tokenize<D: Deque<Token>>(input: &str) -> Result<D, String> {
    tokenize_impl(skip(input), D::empty())
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[allow(clippy::format_collect)]
pub fn tokens_to_xml<D: Deque<Token>>(tokens: &D) -> String {
    let body: String = tokens
        .iter()
        .map(|token| {
            let (tag, value) = match token.as_ref() {
                Token::Keyword(s) => ("keyword", escape_xml(s)),
                Token::Symbol(c) => ("symbol", escape_xml(&c.to_string())),
                Token::IntegerConstant(n) => ("integerConstant", n.to_string()),
                Token::StringConstant(s) => ("stringConstant", escape_xml(s)),
                Token::Identifier(s) => ("identifier", escape_xml(s)),
            };
            format!("<{}> {} </{}>\n", tag, value, tag)
        })
        .collect();
    format!("<tokens>\n{}</tokens>", body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use collections::deque::{BankersDeque, Deque};
    use collections::Empty;

    fn to_vec<D: Deque<Token>>(d: D) -> Vec<Token> {
        d.iter().map(|t| t.as_ref().clone()).collect()
    }

    #[test]
    fn skip_line_comment() {
        assert_eq!(skip("// comment\nfoo"), "foo");
        assert_eq!(skip("// comment at eof"), "");
        assert_eq!(skip("  // comment\n  bar"), "bar");
    }

    #[test]
    fn skip_block_comment() {
        assert_eq!(skip("/* comment */foo"), "foo");
        assert_eq!(skip("/** doc comment */foo"), "foo");
        assert_eq!(skip("/* multi\nline\n*/foo"), "foo");
        assert_eq!(skip("/* nested look */bar"), "bar");
    }

    #[test]
    fn keyword_parsed() {
        assert_eq!(
            keyword_or_identifier("class Foo"),
            Ok((" Foo", Token::Keyword("class".to_string())))
        );
        assert_eq!(
            keyword_or_identifier("return;"),
            Ok((";", Token::Keyword("return".to_string())))
        );
    }

    #[test]
    fn identifier_parsed() {
        assert_eq!(
            keyword_or_identifier("Main {"),
            Ok((" {", Token::Identifier("Main".to_string())))
        );
        assert_eq!(
            keyword_or_identifier("_foo123"),
            Ok(("", Token::Identifier("_foo123".to_string())))
        );
    }

    #[test]
    fn keyword_not_truncated_by_alphanumeric_suffix() {
        // "trueValue" should be an identifier, not keyword "true"
        assert_eq!(
            keyword_or_identifier("trueValue"),
            Ok(("", Token::Identifier("trueValue".to_string())))
        );
        assert_eq!(
            keyword_or_identifier("className"),
            Ok(("", Token::Identifier("className".to_string())))
        );
    }

    #[test]
    fn integer_constant_parsed() {
        assert_eq!(
            integer_constant("42;"),
            Ok((";", Token::IntegerConstant(42)))
        );
        assert_eq!(integer_constant("0 "), Ok((" ", Token::IntegerConstant(0))));
        assert!(integer_constant("abc").is_err());
    }

    #[test]
    fn string_constant_parsed() {
        assert_eq!(
            string_constant("\"hello world\""),
            Ok(("", Token::StringConstant("hello world".to_string())))
        );
        assert_eq!(
            string_constant("\"HOW MANY NUMBERS? \";"),
            Ok((";", Token::StringConstant("HOW MANY NUMBERS? ".to_string())))
        );
        assert!(string_constant("not a string").is_err());
    }

    #[test]
    fn symbols_parsed() {
        for ch in "{}()[].,;+-*/&|<>=~".chars() {
            assert_eq!(symbol(&ch.to_string()), Ok(("", Token::Symbol(ch))));
        }
        assert!(symbol("a").is_err());
    }

    #[test]
    fn tokenize_simple_class() {
        let tokens: BankersDeque<Token> = tokenize("class Main {}").unwrap();
        assert_eq!(
            to_vec(tokens),
            vec![
                Token::Keyword("class".to_string()),
                Token::Identifier("Main".to_string()),
                Token::Symbol('{'),
                Token::Symbol('}'),
            ]
        );
    }

    #[test]
    fn tokenize_strips_comments() {
        let src = "// line comment\nclass /* block */ Main {}";
        let tokens: BankersDeque<Token> = tokenize(src).unwrap();
        assert_eq!(
            to_vec(tokens),
            vec![
                Token::Keyword("class".to_string()),
                Token::Identifier("Main".to_string()),
                Token::Symbol('{'),
                Token::Symbol('}'),
            ]
        );
    }

    #[test]
    fn tokenize_string_and_integer() {
        let tokens =
            to_vec(tokenize::<BankersDeque<Token>>("let x = \"hi\"; let i = 42;").unwrap());
        assert_eq!(tokens[3], Token::StringConstant("hi".to_string()));
        assert_eq!(tokens[8], Token::IntegerConstant(42));
    }

    #[test]
    fn xml_escapes_special_chars() {
        let tokens = BankersDeque::empty()
            .push_back(Token::Symbol('<'))
            .push_back(Token::Symbol('>'))
            .push_back(Token::Symbol('&'));
        let xml = tokens_to_xml(&tokens);
        assert!(xml.contains("&lt;"));
        assert!(xml.contains("&gt;"));
        assert!(xml.contains("&amp;"));
        assert!(!xml.contains(" < "));
    }

    #[test]
    fn xml_format() {
        let tokens = BankersDeque::empty()
            .push_back(Token::Keyword("class".to_string()))
            .push_back(Token::IntegerConstant(10));
        let xml = tokens_to_xml(&tokens);
        assert_eq!(
            xml,
            "<tokens>\n<keyword> class </keyword>\n<integerConstant> 10 </integerConstant>\n</tokens>"
        );
    }
}

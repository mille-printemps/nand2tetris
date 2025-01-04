// Ok: a tuple of (<next string>, <extracted string>)
// Err: a string to be parsed
pub type ParseResult<'a, Output> = Result<(&'a str, Output), &'a str>;

// Pointer type of the parser that dynamically dispatches methods
pub struct BoxedParser<'a, Output> {
    parser: Box<dyn Parser<'a, Output> + 'a>,
}

impl<'a, Output> BoxedParser<'a, Output> {
    fn new<P>(parser: P) -> Self
    where
        P: Parser<'a, Output> + 'a,
    {
        BoxedParser {
            parser: Box::new(parser),
        }
    }
}

// Parser trait
pub trait Parser<'a, Output> {
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output>;

    fn map<F, NextOutput>(self, map_fn: F) -> BoxedParser<'a, NextOutput>
    where
        Self: Sized + 'a,
        Output: 'a,
        NextOutput: 'a,
        F: Fn(Output) -> NextOutput + 'a,
    {
        BoxedParser::new(map(self, map_fn))
    }

    fn pred<F>(self, pred_fn: F) -> BoxedParser<'a, Output>
    where
        Self: Sized + 'a,
        Output: 'a,
        F: Fn(&Output) -> bool + 'a,
    {
        BoxedParser::new(pred(self, pred_fn))
    }

    fn and_then<F, NextParser, NextOutput>(self, then_fn: F) -> BoxedParser<'a, NextOutput>
    where
        Self: Sized + 'a,
        Output: 'a,
        NextOutput: 'a,
        NextParser: Parser<'a, NextOutput> + 'a,
        F: Fn(Output) -> NextParser + 'a,
    {
        BoxedParser::new(and_then(self, then_fn))
    }
}

// Returns a parser that applies 'map_fn' to the result of the parser specified
pub fn map<'a, P, F, A, B>(parser: P, map_fn: F) -> impl Parser<'a, B>
where
    P: Parser<'a, A>,
    F: Fn(A) -> B,
{
    move |input| {
        parser
            .parse(input)
            .map(|(next_input, result)| (next_input, map_fn(result)))
    }
}

// Returns a parser that applies 'pred_fn' to the result of the parser specified,
// and then returns its result if the predicate satisfies
pub fn pred<'a, P, A, F>(parser: P, pred_fn: F) -> impl Parser<'a, A>
where
    P: Parser<'a, A>,
    F: Fn(&A) -> bool,
{
    move |input| {
        if let Ok((next_input, value)) = parser.parse(input) {
            if pred_fn(&value) {
                return Ok((next_input, value));
            }
        }
        Err(input)
    }
}

// Returns a parser that applies 'then_fn' to the result of the parser specified to get the next parser,
// and then, applies the next parser to the next input
pub fn and_then<'a, P, F, A, B, NextParser>(parser: P, then_fn: F) -> impl Parser<'a, B>
where
    P: Parser<'a, A>,
    NextParser: Parser<'a, B>,
    F: Fn(A) -> NextParser,
{
    move |input| match parser.parse(input) {
        Ok((next_input, result)) => then_fn(result).parse(next_input),
        Err(err) => Err(err),
    }
}

// Implementations of the parser trait
impl<'a, Output> Parser<'a, Output> for BoxedParser<'a, Output> {
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output> {
        self.parser.parse(input)
    }
}

impl<'a, F, Output> Parser<'a, Output> for F
where
    F: Fn(&'a str) -> ParseResult<Output>,
{
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output> {
        self(input)
    }
}

// Parsers

pub fn match_literal<'a>(expected: &'static str) -> impl Parser<'a, ()> {
    move |input: &'a str| match input.get(0..expected.len()) {
        Some(next) if next == expected => Ok((&input[expected.len()..], ())),
        _ => Err(input),
    }
}

pub fn number(input: &str) -> ParseResult<String> {
    let mut matched = String::new();
    let mut chars = input.chars();
    let mut has_dot = false;

    match chars.next() {
        Some(next) if next.is_ascii_digit() => matched.push(next),
        Some(next) if next == '.' => return Err(input),
        _ => return Err(input),
    }

    for next in chars {
        if next.is_ascii_digit() {
            matched.push(next);
        } else if next == '.' && !has_dot {
            matched.push(next);
            has_dot = true;
        } else {
            break;
        }
    }

    let next_index = matched.len();
    Ok((&input[next_index..], matched))
}

pub fn identifier(input: &str) -> ParseResult<String> {
    let mut matched = String::new();
    let mut chars = input.chars();

    match chars.next() {
        Some(next) if next.is_alphabetic() => matched.push(next),
        _ => return Err(input),
    }

    for next in chars {
        if next.is_alphanumeric()
            || next == '-'
            || next == '_'
            || next == '.'
            || next == '$'
            || next == ':'
        {
            matched.push(next);
        } else {
            break;
        }
    }

    let next_index = matched.len();
    Ok((&input[next_index..], matched))
}

pub fn pair<'a, P1, P2, R1, R2>(parser1: P1, parser2: P2) -> impl Parser<'a, (R1, R2)>
where
    P1: Parser<'a, R1>,
    P2: Parser<'a, R2>,
{
    move |input| {
        parser1.parse(input).and_then(|(next_input, result1)| {
            parser2
                .parse(next_input)
                .map(|(last_input, result2)| (last_input, (result1, result2)))
        })
    }
}

pub fn left<'a, P1, P2, R1, R2>(parser1: P1, parser2: P2) -> impl Parser<'a, R1>
where
    P1: Parser<'a, R1>,
    P2: Parser<'a, R2>,
{
    map(pair(parser1, parser2), |(left, _right)| left)
}

pub fn right<'a, P1, P2, R1, R2>(parser1: P1, parser2: P2) -> impl Parser<'a, R2>
where
    P1: Parser<'a, R1>,
    P2: Parser<'a, R2>,
{
    map(pair(parser1, parser2), |(_left, right)| right)
}

pub fn either<'a, P1, P2, A>(parser1: P1, parser2: P2) -> impl Parser<'a, A>
where
    P1: Parser<'a, A>,
    P2: Parser<'a, A>,
{
    move |input| match parser1.parse(input) {
        ok @ Ok(_) => ok,
        Err(_) => parser2.parse(input),
    }
}

pub fn one_or_more<'a, P, A>(parser: P) -> impl Parser<'a, Vec<A>>
where
    P: Parser<'a, A>,
{
    move |mut input| {
        let mut result = Vec::new();

        if let Ok((next_input, first_item)) = parser.parse(input) {
            input = next_input;
            result.push(first_item);
        } else {
            return Err(input);
        }

        while let Ok((next_input, next_item)) = parser.parse(input) {
            input = next_input;
            result.push(next_item);
        }

        Ok((input, result))
    }
}

pub fn zero_or_more<'a, P, A>(parser: P) -> impl Parser<'a, Vec<A>>
where
    P: Parser<'a, A>,
{
    move |mut input| {
        let mut result = Vec::new();

        while let Ok((next_input, next_item)) = parser.parse(input) {
            input = next_input;
            result.push(next_item);
        }

        Ok((input, result))
    }
}

pub fn any_char(input: &str) -> ParseResult<char> {
    match input.chars().next() {
        Some(next) => Ok((&input[next.len_utf8()..], next)),
        _ => Err(input),
    }
}

fn comment(input: &str) -> ParseResult<&str> {
    match input.find("//") {
        Some(index) => Ok((&input[0..index], &input[index + 2..])),
        _ => Err(input),
    }
}

pub fn whitespace_char<'a>() -> impl Parser<'a, char> {
    pred(any_char, |c| c.is_whitespace())
}

pub fn space1<'a>() -> impl Parser<'a, Vec<char>> {
    one_or_more(whitespace_char())
}

pub fn space0<'a>() -> impl Parser<'a, Vec<char>> {
    zero_or_more(whitespace_char())
}

pub fn quoted_string<'a>() -> impl Parser<'a, String> {
    right(
        match_literal("\""),
        left(
            zero_or_more(pred(any_char, |c| *c != '"')),
            match_literal("\""),
        ),
    )
    .map(|chars| chars.into_iter().collect())
}

pub enum Enclosure {
    Parentheses,
    CurlyBraces,
    SquareBrackets,
}

pub fn enclosed_string<'a>(enclosure: Enclosure) -> impl Parser<'a, String> {
    let (l, r) = match enclosure {
        Enclosure::Parentheses => ("(", ")"),
        Enclosure::CurlyBraces => ("{", "}"),
        Enclosure::SquareBrackets => ("[", "]"),
    };
    right(
        match_literal(l),
        left(
            zero_or_more(pred(any_char, move |c| *c != r.chars().next().unwrap())),
            match_literal(r),
        ),
    )
    .map(|chars| chars.into_iter().collect())
}

fn attribute_pair<'a>() -> impl Parser<'a, (String, String)> {
    pair(identifier, right(match_literal("="), quoted_string()))
}

pub fn attributes<'a>() -> impl Parser<'a, Vec<(String, String)>> {
    zero_or_more(right(space1(), attribute_pair()))
}

pub fn assignment_pair<'a, P>(parser: P) -> impl Parser<'a, (String, String)>
where
    P: Parser<'a, String>,
{
    pair(identifier, right(match_literal("="), parser))
}

pub fn assignments<'a, P>(parser: P) -> impl Parser<'a, Vec<(String, String)>>
where
    P: Parser<'a, String>,
{
    zero_or_more(right(space0(), assignment_pair(parser)))
}

pub fn simple_comment<'a>() -> impl Parser<'a, String> {
    zero_or_more(comment).map(|strs| strs.concat())
}

pub fn whitespace_wrap<'a, P, A>(parser: P) -> impl Parser<'a, A>
where
    P: Parser<'a, A>,
{
    right(space0(), left(parser, space0()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_parser() {
        let joe = match_literal("Hello Joe!");
        assert_eq!(Ok(("", ())), joe.parse("Hello Joe!"));
        assert_eq!(
            Ok((" Hello Robert!", ())),
            joe.parse("Hello Joe! Hello Robert!")
        );
        assert_eq!(Err("Hello Mike!"), joe.parse("Hello Mike!"));
    }

    #[test]
    fn identifier_parser() {
        assert_eq!(
            Ok(("", "i-am-an-identifier".to_string())),
            identifier("i-am-an-identifier")
        );
        assert_eq!(
            Ok((" entirely an identifier", "not".to_string())),
            identifier("not entirely an identifier")
        );
        assert_eq!(
            Err("!not at all an identifier"),
            identifier("!not at all an identifier")
        );
    }

    #[test]
    fn number_parser() {
        assert_eq!(Ok(("", "123".to_string())), number("123"));
        assert_eq!(Ok(("", "123.456".to_string())), number("123.456"));
        assert_eq!(Err(".123"), number(".123"));
    }

    #[test]
    fn right_combinator() {
        let tag_opener = right(match_literal("<"), identifier);
        assert_eq!(
            Ok(("/>", "my-first-element".to_string())),
            tag_opener.parse("<my-first-element/>")
        );
        assert_eq!(Err("oops"), tag_opener.parse("oops"));
        assert_eq!(Err("!oops"), tag_opener.parse("<!oops"));
    }

    #[test]
    fn one_or_more_combinator() {
        let parser = one_or_more(match_literal("ha"));
        assert_eq!(Ok(("", vec![(), (), ()])), parser.parse("hahaha"));
        assert_eq!(Err("ahah"), parser.parse("ahah"));
        assert_eq!(Err(""), parser.parse(""));
    }

    #[test]
    fn zero_or_more_combinator() {
        let parser = zero_or_more(match_literal("ha"));
        assert_eq!(Ok(("", vec![(), (), ()])), parser.parse("hahaha"));
        assert_eq!(Ok(("ahah", vec![])), parser.parse("ahah"));
        assert_eq!(Ok(("", vec![])), parser.parse(""));
    }

    #[test]
    fn predicate_combinator() {
        let parser = pred(any_char, |c| *c == 'o');
        assert_eq!(Ok(("mg", 'o')), parser.parse("omg"));
        assert_eq!(Err("lol"), parser.parse("lol"));
    }

    #[test]
    fn quoted_string_parser() {
        assert_eq!(
            Ok(("", "Hello Joe!".to_string())),
            quoted_string().parse("\"Hello Joe!\"")
        );
    }

    #[test]
    fn enclosed_string_parser() {
        assert_eq!(
            Ok(("", "one".to_string())),
            enclosed_string(Enclosure::Parentheses).parse("(one)")
        );

        assert_eq!(
            Ok(("", "two".to_string())),
            enclosed_string(Enclosure::CurlyBraces).parse("{two}")
        );

        assert_eq!(
            Ok(("", "three".to_string())),
            enclosed_string(Enclosure::SquareBrackets).parse("[three]")
        );
    }

    #[test]
    fn attribute_parser() {
        assert_eq!(
            Ok((
                "",
                vec![
                    ("one".to_string(), "1".to_string()),
                    ("two".to_string(), "2".to_string())
                ]
            )),
            attributes().parse(" one=\"1\" two=\"2\"")
        );
    }

    #[test]
    fn assignment_parser() {
        assert_eq!(
            Ok((
                "",
                vec![
                    ("R1".to_string(), "A1".to_string()),
                    ("R2".to_string(), "A2".to_string())
                ]
            )),
            assignments(identifier).parse("   R1=A1   R2=A2")
        )
    }

    #[test]
    fn simple_comment_parser() {
        assert_eq!(
            Ok((" R1=A1 ", " Comment".to_string())),
            simple_comment().parse(" R1=A1 // Comment")
        );
        assert_eq!(
            Ok((" R1=A1 ", "".to_string())),
            simple_comment().parse(" R1=A1 ")
        );
    }
}

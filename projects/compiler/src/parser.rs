// Jack's parser, built on lib/parser's combinators over a token stream (Tokens<'a> = &'a [Token])

use collections::deque::{BankersDeque, Deque};
use collections::Empty;
use parser::parser::{context, either, left, map, pair, right, zero_or_more, Parser};
use tokenizer::token::Token;

use crate::ast::*;

pub type Tokens<'a> = &'a [Token];
pub type ParseResult<'a, Output> = parser::parser::ParseResult<'a, Output, Tokens<'a>, String>;

// Token-level primitives

fn symbol<'a>(expected: char) -> impl Parser<'a, (), Tokens<'a>, String> {
    context(
        move |input: Tokens<'a>| match input.first() {
            Some(Token::Symbol(found)) if *found == expected => Ok((&input[1..], ())),
            _ => Err(input),
        },
        format!("'{}'", expected),
    )
}

fn keyword<'a>(expected: &'static str) -> impl Parser<'a, (), Tokens<'a>, String> {
    context(
        move |input: Tokens<'a>| match input.first() {
            Some(Token::Keyword(found)) if found == expected => Ok((&input[1..], ())),
            _ => Err(input),
        },
        format!("keyword '{}'", expected),
    )
}

fn identifier(input: Tokens) -> ParseResult<String> {
    match input.first() {
        Some(Token::Identifier(name)) => Ok((&input[1..], name.clone())),
        other => Err(format!("expected identifier, found {:?}", other)),
    }
}

fn int_const(input: Tokens) -> ParseResult<u16> {
    match input.first() {
        Some(Token::IntegerConstant(value)) => Ok((&input[1..], *value)),
        other => Err(format!("expected integer constant, found {:?}", other)),
    }
}

fn str_const(input: Tokens) -> ParseResult<String> {
    match input.first() {
        Some(Token::StringConstant(value)) => Ok((&input[1..], value.clone())),
        other => Err(format!("expected string constant, found {:?}", other)),
    }
}

fn operator(input: Tokens) -> ParseResult<char> {
    match input.first() {
        Some(Token::Symbol(character)) if "+-*/&|<>=".contains(*character) => {
            Ok((&input[1..], *character))
        }
        // consumed by zero_or_more in parse_expression, never surfaced
        other => Err(format!("expected operator, found {:?}", other)),
    }
}

// Expressions: term <-> expression is the mutual recursion

fn parse_expression(input: Tokens) -> ParseResult<Expr> {
    map(
        pair(parse_term, zero_or_more(pair(operator, parse_term))),
        |(first, tail): (Expr, Vec<(char, Expr)>)| {
            tail.into_iter().fold(first, |left, (op, right)| {
                Expr::Binary(op, Box::new(left), Box::new(right))
            })
        },
    )
    .parse(input)
}

fn parse_term(input: Tokens) -> ParseResult<Expr> {
    context(
        either(
            either(
                map(int_const, Expr::IntConst),
                map(str_const, Expr::StrConst),
            ),
            either(
                either(
                    either(
                        map(keyword("true"), |_| Expr::True),
                        map(keyword("false"), |_| Expr::False),
                    ),
                    either(
                        map(keyword("null"), |_| Expr::Null),
                        map(keyword("this"), |_| Expr::This),
                    ),
                ),
                either(parenthesized_expr, either(unary_term, identifier_term)),
            ),
        ),
        "term".to_string(),
    )
    .parse(input)
}

// The recursive step: a term can contain a whole expression

fn parenthesized_expr(input: Tokens) -> ParseResult<Expr> {
    right(symbol('('), left(parse_expression, symbol(')'))).parse(input)
}

fn unary_term(input: Tokens) -> ParseResult<Expr> {
    either(
        map(right(symbol('-'), parse_term), |operand| {
            Expr::Unary('-', Box::new(operand))
        }),
        map(right(symbol('~'), parse_term), |operand| {
            Expr::Unary('~', Box::new(operand))
        }),
    )
    .parse(input)
}

// var, var[expr], var(args), var.name(args)
// tell these apart needs one token of lookahead past the identifier

fn identifier_term(input: Tokens) -> ParseResult<Expr> {
    let (rest, name) = identifier(input)?;
    match rest.first() {
        Some(Token::Symbol('[')) => map(
            right(symbol('['), left(parse_expression, symbol(']'))),
            move |index| Expr::Index(name.clone(), Box::new(index)),
        )
        .parse(rest),
        Some(Token::Symbol('(')) | Some(Token::Symbol('.')) => {
            map(call_suffix(name), Expr::Call).parse(rest)
        }
        _ => Ok((rest, Expr::Var(name))),
    }
}

fn call_suffix<'a>(name: String) -> impl Parser<'a, SubroutineCall, Tokens<'a>, String> {
    move |input: Tokens<'a>| match input.first() {
        Some(Token::Symbol('(')) => {
            let name = name.clone();
            map(
                right(symbol('('), left(expression_list, symbol(')'))),
                move |arguments| SubroutineCall::Simple(name.clone(), arguments),
            )
            .parse(input)
        }
        Some(Token::Symbol('.')) => {
            let (rest, _) = symbol('.').parse(input)?;
            let (rest, method) = identifier(rest)?;
            let name = name.clone();
            map(
                right(symbol('('), left(expression_list, symbol(')'))),
                move |arguments| SubroutineCall::Qualified(name.clone(), method.clone(), arguments),
            )
            .parse(rest)
        }
        other => Err(format!("expected '(' or '.', found {:?}", other)),
    }
}

fn expression_list(input: Tokens) -> ParseResult<BankersDeque<Expr>> {
    match input.first() {
        Some(Token::Symbol(')')) => Ok((input, BankersDeque::empty())),
        _ => map(
            pair(
                parse_expression,
                zero_or_more(right(symbol(','), parse_expression)),
            ),
            |(first, rest): (Expr, Vec<Expr>)| {
                rest.into_iter().fold(
                    BankersDeque::empty().push_back(first),
                    |deque, expression| deque.push_back(expression),
                )
            },
        )
        .parse(input),
    }
}

// Statements: the other kind of recursion — blocks containing statements

// zero_or_more can't tell "no more statements" apart from "this statement started matching and then failed"
// It discards either kind of failure and just stops, so a statement that fails partway through
// (e.g. `let x` with no `= value;`) would silently vanish instead of being reported,
// and the caller would see a confusing failure at whatever comes after instead.

fn parse_statements(input: Tokens) -> ParseResult<BankersDeque<Statement>> {
    parse_statements_acc(input, BankersDeque::empty())
}

fn parse_statements_acc(
    input: Tokens,
    accumulated: BankersDeque<Statement>,
) -> ParseResult<BankersDeque<Statement>> {
    match input.first() {
        Some(Token::Keyword(word))
            if matches!(word.as_str(), "let" | "if" | "while" | "do" | "return") =>
        {
            let (rest, statement) = parse_statement(input)?;
            parse_statements_acc(rest, accumulated.push_back(statement))
        }
        _ => Ok((input, accumulated)),
    }
}

fn parse_statement(input: Tokens) -> ParseResult<Statement> {
    match input.first() {
        Some(Token::Keyword(word)) if word == "let" => parse_let(input),
        Some(Token::Keyword(word)) if word == "if" => parse_if(input),
        Some(Token::Keyword(word)) if word == "while" => parse_while(input),
        Some(Token::Keyword(word)) if word == "do" => parse_do(input),
        Some(Token::Keyword(word)) if word == "return" => parse_return(input),
        // consumed by zero_or_more in parse_statements, never surfaced
        other => Err(format!("expected statement, found {:?}", other)),
    }
}

fn parse_let(input: Tokens) -> ParseResult<Statement> {
    let (rest, _) = keyword("let").parse(input)?;
    let (rest, var) = identifier(rest)?;
    let (rest, index) = match rest.first() {
        Some(Token::Symbol('[')) => {
            let (rest, expression) =
                right(symbol('['), left(parse_expression, symbol(']'))).parse(rest)?;
            (rest, Some(expression))
        }
        _ => (rest, None),
    };
    let (rest, _) = symbol('=').parse(rest)?;
    let (rest, value) = parse_expression(rest)?;
    let (rest, _) = symbol(';').parse(rest)?;
    Ok((rest, Statement::Let { var, index, value }))
}

fn parse_if(input: Tokens) -> ParseResult<Statement> {
    let (rest, _) = keyword("if").parse(input)?;
    let (rest, _) = symbol('(').parse(rest)?;
    let (rest, condition) = parse_expression(rest)?;
    let (rest, _) = symbol(')').parse(rest)?;
    let (rest, _) = symbol('{').parse(rest)?;
    let (rest, then_body) = parse_statements(rest)?;
    let (rest, _) = symbol('}').parse(rest)?;
    let (rest, else_body) = match rest.first() {
        Some(Token::Keyword(word)) if word == "else" => {
            let (rest, _) = symbol('{').parse(&rest[1..])?;
            let (rest, statements) = parse_statements(rest)?;
            let (rest, _) = symbol('}').parse(rest)?;
            (rest, Some(statements))
        }
        _ => (rest, None),
    };
    Ok((
        rest,
        Statement::If {
            condition,
            then_body,
            else_body,
        },
    ))
}

fn parse_while(input: Tokens) -> ParseResult<Statement> {
    let (rest, _) = keyword("while").parse(input)?;
    let (rest, _) = symbol('(').parse(rest)?;
    let (rest, condition) = parse_expression(rest)?;
    let (rest, _) = symbol(')').parse(rest)?;
    let (rest, _) = symbol('{').parse(rest)?;
    let (rest, body) = parse_statements(rest)?;
    let (rest, _) = symbol('}').parse(rest)?;
    Ok((rest, Statement::While { condition, body }))
}

fn parse_do(input: Tokens) -> ParseResult<Statement> {
    let (rest, _) = keyword("do").parse(input)?;
    let (rest, name) = identifier(rest)?;
    let (rest, call) = call_suffix(name).parse(rest)?;
    let (rest, _) = symbol(';').parse(rest)?;
    Ok((rest, Statement::Do(call)))
}

fn parse_return(input: Tokens) -> ParseResult<Statement> {
    let (rest, _) = keyword("return").parse(input)?;
    let (rest, value) = match rest.first() {
        Some(Token::Symbol(';')) => (rest, None),
        _ => {
            let (rest, expression) = parse_expression(rest)?;
            (rest, Some(expression))
        }
    };
    let (rest, _) = symbol(';').parse(rest)?;
    Ok((rest, Statement::Return(value)))
}

// Types and comma-separated lists

fn parse_type(input: Tokens) -> ParseResult<Type> {
    context(
        either(
            either(
                map(keyword("int"), |_| Type::Int),
                map(keyword("char"), |_| Type::Char),
            ),
            either(
                map(keyword("boolean"), |_| Type::Boolean),
                map(identifier, Type::ClassName),
            ),
        ),
        "type".to_string(),
    )
    .parse(input)
}

fn return_type(input: Tokens) -> ParseResult<Option<Type>> {
    match input.first() {
        Some(Token::Keyword(word)) if word == "void" => Ok((&input[1..], None)),
        _ => map(parse_type, Some).parse(input),
    }
}

// identifier (',' identifier)*
fn var_names(input: Tokens) -> ParseResult<BankersDeque<String>> {
    map(
        pair(identifier, zero_or_more(right(symbol(','), identifier))),
        |(first, rest): (String, Vec<String>)| {
            rest.into_iter()
                .fold(BankersDeque::empty().push_back(first), |deque, name| {
                    deque.push_back(name)
                })
        },
    )
    .parse(input)
}

fn parameter(input: Tokens) -> ParseResult<Parameter> {
    map(pair(parse_type, identifier), |(typ, name)| Parameter {
        typ,
        name,
    })
    .parse(input)
}

// (type identifier (',' type identifier)*)?
fn parameter_list(input: Tokens) -> ParseResult<BankersDeque<Parameter>> {
    match input.first() {
        Some(Token::Symbol(')')) => Ok((input, BankersDeque::empty())),
        _ => map(
            pair(parameter, zero_or_more(right(symbol(','), parameter))),
            |(first, rest): (Parameter, Vec<Parameter>)| {
                rest.into_iter()
                    .fold(BankersDeque::empty().push_back(first), |deque, param| {
                        deque.push_back(param)
                    })
            },
        )
        .parse(input),
    }
}

// Declarations

// 'var' type var_names ';'
fn var_dec(input: Tokens) -> ParseResult<VarDec> {
    map(
        right(
            keyword("var"),
            left(pair(parse_type, var_names), symbol(';')),
        ),
        |(typ, names)| VarDec { typ, names },
    )
    .parse(input)
}

// ('static'|'field') type var_names ';'
fn class_var_dec(input: Tokens) -> ParseResult<ClassVarDec> {
    map(
        pair(
            either(
                map(keyword("static"), |_| VarKind::Static),
                map(keyword("field"), |_| VarKind::Field),
            ),
            left(pair(parse_type, var_names), symbol(';')),
        ),
        |(kind, (typ, names))| ClassVarDec { kind, typ, names },
    )
    .parse(input)
}

// '{' var_dec* statements '}'
fn subroutine_body(input: Tokens) -> ParseResult<(BankersDeque<VarDec>, BankersDeque<Statement>)> {
    right(
        symbol('{'),
        left(
            pair(
                map(zero_or_more(var_dec), |decs: Vec<VarDec>| {
                    decs.into_iter()
                        .fold(BankersDeque::empty(), |deque, dec| deque.push_back(dec))
                }),
                parse_statements,
            ),
            symbol('}'),
        ),
    )
    .parse(input)
}

fn subroutine_kind(input: Tokens) -> ParseResult<SubroutineKind> {
    context(
        either(
            either(
                map(keyword("constructor"), |_| SubroutineKind::Constructor),
                map(keyword("function"), |_| SubroutineKind::Function),
            ),
            map(keyword("method"), |_| SubroutineKind::Method),
        ),
        "subroutine kind".to_string(),
    )
    .parse(input)
}

// subroutine_kind return_type identifier '(' parameter_list ')' subroutine_body
fn subroutine_dec(input: Tokens) -> ParseResult<SubroutineDec> {
    let (rest, kind) = subroutine_kind(input)?;
    let (rest, _) = return_type(rest)?;
    let (rest, name) = identifier(rest)?;
    let (rest, _) = symbol('(').parse(rest)?;
    let (rest, params) = parameter_list(rest)?;
    let (rest, _) = symbol(')').parse(rest)?;
    let (rest, (locals, body)) = subroutine_body(rest)?;
    Ok((
        rest,
        SubroutineDec {
            kind,
            name,
            params,
            locals,
            body,
        },
    ))
}

// (class_var_dec | subroutine_dec)* '}'
// A class_var_dec/subroutine_dec that starts matching and then fails (e.g. a field decl missing its ';')
// Must not be silently discarded by a zero_or_more
fn class_body(
    input: Tokens,
) -> ParseResult<(BankersDeque<ClassVarDec>, BankersDeque<SubroutineDec>)> {
    class_body_acc(input, BankersDeque::empty(), BankersDeque::empty())
}

fn class_body_acc(
    input: Tokens,
    var_decs: BankersDeque<ClassVarDec>,
    subroutines: BankersDeque<SubroutineDec>,
) -> ParseResult<(BankersDeque<ClassVarDec>, BankersDeque<SubroutineDec>)> {
    match input.first() {
        Some(Token::Keyword(word)) if matches!(word.as_str(), "static" | "field") => {
            let (rest, dec) = class_var_dec(input)?;
            class_body_acc(rest, var_decs.push_back(dec), subroutines)
        }
        Some(Token::Keyword(word))
            if matches!(word.as_str(), "constructor" | "function" | "method") =>
        {
            let (rest, dec) = subroutine_dec(input)?;
            class_body_acc(rest, var_decs, subroutines.push_back(dec))
        }
        _ => {
            let (rest, _) = symbol('}').parse(input)?;
            Ok((rest, (var_decs, subroutines)))
        }
    }
}

// 'class' identifier '{' class_body
pub fn parse_class(tokens: &[Token]) -> Result<Class, String> {
    let (rest, _) = keyword("class").parse(tokens)?;
    let (rest, name) = identifier(rest)?;
    let (rest, _) = symbol('{').parse(rest)?;
    let (rest, (var_decs, subroutines)) = class_body(rest)?;
    if !rest.is_empty() {
        return Err(format!(
            "unexpected tokens after class: {:?}",
            &rest[..rest.len().min(3)]
        ));
    }
    Ok(Class {
        name,
        var_decs,
        subroutines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokenizer::token::tokenize;

    fn tokens(source: &str) -> Vec<Token> {
        let deque: BankersDeque<Token> = tokenize(source).unwrap();
        deque.iter().map(|token| token.as_ref().clone()).collect()
    }

    // Expr/Statement/Class don't derive PartialEq (only Debug), and adding
    // it isn't this module's call — comparing Debug output is enough to
    // check shape.
    fn debug(value: &impl std::fmt::Debug) -> String {
        format!("{:?}", value)
    }

    #[test]
    fn parses_nested_parenthesized_arithmetic() {
        let source = tokens("(1 + 2) * (3 - x)");
        let (rest, expression) = parse_expression(&source).unwrap();
        assert!(rest.is_empty());
        assert_eq!(
            debug(&expression),
            debug(&Expr::Binary(
                '*',
                Box::new(Expr::Binary(
                    '+',
                    Box::new(Expr::IntConst(1)),
                    Box::new(Expr::IntConst(2)),
                )),
                Box::new(Expr::Binary(
                    '-',
                    Box::new(Expr::IntConst(3)),
                    Box::new(Expr::Var("x".to_string())),
                )),
            ))
        );
    }

    #[test]
    fn parses_nested_calls_and_indexing() {
        // A qualified call whose argument is an indexed access whose index
        // is itself a binary expression — three recursion levels deep.
        let source = tokens("Array.new(a[1 + 2])");
        let (rest, expression) = parse_expression(&source).unwrap();
        assert!(rest.is_empty());
        assert_eq!(
            debug(&expression),
            debug(&Expr::Call(SubroutineCall::Qualified(
                "Array".to_string(),
                "new".to_string(),
                BankersDeque::empty().push_back(Expr::Index(
                    "a".to_string(),
                    Box::new(Expr::Binary(
                        '+',
                        Box::new(Expr::IntConst(1)),
                        Box::new(Expr::IntConst(2)),
                    )),
                )),
            )))
        );
    }

    #[test]
    fn parses_nested_if_else_blocks() {
        let source = tokens(
            "if (x > 0) { if (y > 0) { let z = 1; } else { let z = 2; } } else { let z = 3; }",
        );
        let (rest, statement) = parse_statement(&source).unwrap();
        assert!(rest.is_empty());
        assert!(matches!(statement, Statement::If { .. }));
        let rendered = debug(&statement);
        assert!(rendered.contains("IntConst(1)"));
        assert!(rendered.contains("IntConst(2)"));
        assert!(rendered.contains("IntConst(3)"));
    }

    #[test]
    fn parses_while_with_nested_expression() {
        let source = tokens("while (~(x < 10)) { let x = x + 1; }");
        let (rest, statement) = parse_statement(&source).unwrap();
        assert!(rest.is_empty());
        assert!(matches!(statement, Statement::While { .. }));
    }

    #[test]
    fn rejects_malformed_input() {
        let source = tokens("(1 +");
        assert!(parse_expression(&source).is_err());
    }

    #[test]
    fn parses_a_full_class() {
        let source = tokens(
            r#"
            class Point {
                field int x, y;
                static int count;

                constructor Point new(int ax, int ay) {
                    let x = ax;
                    let y = ay;
                    return this;
                }

                method int getX() {
                    return x;
                }

                method void move(int dx, int dy) {
                    var int newX;
                    let newX = x + dx;
                    let x = newX;
                    let y = y + dy;
                    return;
                }
            }
            "#,
        );
        let class = parse_class(&source).unwrap();
        assert_eq!(class.name, "Point");
        assert_eq!(class.var_decs.len(), 2);
        assert_eq!(class.subroutines.len(), 3);
        let constructor = class.subroutines.iter().next().unwrap();
        assert_eq!(constructor.as_ref().name, "new");
        assert!(matches!(
            constructor.as_ref().kind,
            SubroutineKind::Constructor
        ));
        assert_eq!(constructor.as_ref().params.len(), 2);
    }

    #[test]
    fn class_error_names_what_was_expected() {
        // Missing the closing ';' after the field declaration.
        let source = tokens("class Point { field int x }");
        let error = parse_class(&source).unwrap_err();
        assert!(error.contains("';'"), "error was: {}", error);
    }

    #[test]
    fn statement_error_names_the_missing_piece() {
        let source = tokens("if (x > 0) { let x = 1; }");
        // Deliberately parse as a lone statement with a dangling 'else' missing its block.
        let broken = tokens("let x");
        let error = parse_statement(&broken).unwrap_err();
        assert!(error.contains("'='"), "error was: {}", error);
        let _ = source; // silence unused warning if reordered
    }
}

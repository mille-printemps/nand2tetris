use collections::deque::{BankersDeque, Deque};
use collections::Empty;
use tokenizer::token::Token;

use crate::ast::*;

type Tokens<'a> = &'a [Token];
type ParseResult<'a, T> = Result<(Tokens<'a>, T), String>;

// Primitives

fn expect_keyword(tokens: Tokens, expected: String) -> ParseResult<()> {
    match tokens.first() {
        Some(Token::Keyword(keyword)) if *keyword == expected => Ok((&tokens[1..], ())),
        other => Err(format!("Expected keyword '{}', got {:?}", expected, other)),
    }
}

fn expect_symbol(tokens: Tokens, expected: char) -> ParseResult<()> {
    match tokens.first() {
        Some(Token::Symbol(character)) if *character == expected => Ok((&tokens[1..], ())),
        other => Err(format!("Expected '{}', got {:?}", expected, other)),
    }
}

fn parse_identifier(tokens: Tokens) -> ParseResult<String> {
    match tokens.first() {
        Some(Token::Identifier(identifier)) => Ok((&tokens[1..], identifier.clone())),
        other => Err(format!("Expected identifier, got {:?}", other)),
    }
}

fn parse_type(tokens: Tokens) -> ParseResult<Type> {
    match tokens.first() {
        Some(Token::Keyword(keyword)) if keyword == "int" => Ok((&tokens[1..], Type::Int)),
        Some(Token::Keyword(keyword)) if keyword == "char" => Ok((&tokens[1..], Type::Char)),
        Some(Token::Keyword(keyword)) if keyword == "boolean" => Ok((&tokens[1..], Type::Boolean)),
        Some(Token::Identifier(identifier)) => {
            Ok((&tokens[1..], Type::ClassName(identifier.clone())))
        }
        other => Err(format!("Expected type, got {:?}", other)),
    }
}

fn parse_return_type(tokens: Tokens) -> ParseResult<Option<Type>> {
    match tokens.first() {
        Some(Token::Keyword(keyword)) if keyword == "void" => Ok((&tokens[1..], None)),
        _ => parse_type(tokens).map(|(remaining, typ)| (remaining, Some(typ))),
    }
}

// Comma-separated lists

fn parse_var_names_rest(
    tokens: Tokens,
    names: BankersDeque<String>,
) -> ParseResult<BankersDeque<String>> {
    match tokens.first() {
        Some(Token::Symbol(',')) => {
            let (remaining, name) = parse_identifier(&tokens[1..])?;
            parse_var_names_rest(remaining, names.push_back(name))
        }
        _ => Ok((tokens, names)),
    }
}

fn parse_var_names(tokens: Tokens) -> ParseResult<BankersDeque<String>> {
    let (remaining, first) = parse_identifier(tokens)?;
    parse_var_names_rest(remaining, BankersDeque::empty().push_back(first))
}

fn parse_parameter_list_rest(
    tokens: Tokens,
    parameters: BankersDeque<Parameter>,
) -> ParseResult<BankersDeque<Parameter>> {
    match tokens.first() {
        Some(Token::Symbol(',')) => {
            let (remaining, typ) = parse_type(&tokens[1..])?;
            let (remaining, name) = parse_identifier(remaining)?;
            parse_parameter_list_rest(remaining, parameters.push_back(Parameter { typ, name }))
        }
        _ => Ok((tokens, parameters)),
    }
}

fn parse_parameter_list(tokens: Tokens) -> ParseResult<BankersDeque<Parameter>> {
    match tokens.first() {
        Some(Token::Symbol(')')) => Ok((tokens, BankersDeque::empty())),
        _ => {
            let (remaining, typ) = parse_type(tokens)?;
            let (remaining, name) = parse_identifier(remaining)?;
            parse_parameter_list_rest(
                remaining,
                BankersDeque::empty().push_back(Parameter { typ, name }),
            )
        }
    }
}

fn parse_expression_list_rest(
    tokens: Tokens,
    expressions: BankersDeque<Expr>,
) -> ParseResult<BankersDeque<Expr>> {
    match tokens.first() {
        Some(Token::Symbol(',')) => {
            let (remaining, expression) = parse_expression(&tokens[1..])?;
            parse_expression_list_rest(remaining, expressions.push_back(expression))
        }
        _ => Ok((tokens, expressions)),
    }
}

fn parse_expression_list(tokens: Tokens) -> ParseResult<BankersDeque<Expr>> {
    match tokens.first() {
        Some(Token::Symbol(')')) => Ok((tokens, BankersDeque::empty())),
        _ => {
            let (remaining, first) = parse_expression(tokens)?;
            parse_expression_list_rest(remaining, BankersDeque::empty().push_back(first))
        }
    }
}

// Expressions

// Called with `tokens` starting at `(` or `.` (after the leading name was consumed).
fn parse_call_suffix(name: String, tokens: Tokens) -> ParseResult<SubroutineCall> {
    match tokens.first() {
        Some(Token::Symbol('(')) => {
            let (remaining, arguments) = parse_expression_list(&tokens[1..])?;
            let (remaining, _) = expect_symbol(remaining, ')')?;
            Ok((remaining, SubroutineCall::Simple(name, arguments)))
        }
        Some(Token::Symbol('.')) => {
            let (remaining, method) = parse_identifier(&tokens[1..])?;
            let (remaining, _) = expect_symbol(remaining, '(')?;
            let (remaining, arguments) = parse_expression_list(remaining)?;
            let (remaining, _) = expect_symbol(remaining, ')')?;
            Ok((
                remaining,
                SubroutineCall::Qualified(name, method, arguments),
            ))
        }
        other => Err(format!("Expected '(' or '.', got {:?}", other)),
    }
}

fn parse_term(tokens: Tokens) -> ParseResult<Expr> {
    match tokens.first() {
        Some(Token::IntegerConstant(value)) => Ok((&tokens[1..], Expr::IntConst(*value))),
        Some(Token::StringConstant(value)) => Ok((&tokens[1..], Expr::StrConst(value.clone()))),
        Some(Token::Keyword(keyword)) if keyword == "true" => Ok((&tokens[1..], Expr::True)),
        Some(Token::Keyword(keyword)) if keyword == "false" => Ok((&tokens[1..], Expr::False)),
        Some(Token::Keyword(keyword)) if keyword == "null" => Ok((&tokens[1..], Expr::Null)),
        Some(Token::Keyword(keyword)) if keyword == "this" => Ok((&tokens[1..], Expr::This)),
        Some(Token::Symbol('(')) => {
            let (remaining, expression) = parse_expression(&tokens[1..])?;
            let (remaining, _) = expect_symbol(remaining, ')')?;
            Ok((remaining, expression))
        }
        Some(Token::Symbol('-')) => {
            let (remaining, operand) = parse_term(&tokens[1..])?;
            Ok((remaining, Expr::Unary('-', Box::new(operand))))
        }
        Some(Token::Symbol('~')) => {
            let (remaining, operand) = parse_term(&tokens[1..])?;
            Ok((remaining, Expr::Unary('~', Box::new(operand))))
        }
        Some(Token::Identifier(identifier)) => {
            let name = identifier.clone();
            match tokens.get(1) {
                Some(Token::Symbol('[')) => {
                    let (remaining, index) = parse_expression(&tokens[2..])?;
                    let (remaining, _) = expect_symbol(remaining, ']')?;
                    Ok((remaining, Expr::Index(name, Box::new(index))))
                }
                Some(Token::Symbol('(')) | Some(Token::Symbol('.')) => {
                    let (remaining, call) = parse_call_suffix(name, &tokens[1..])?;
                    Ok((remaining, Expr::Call(call)))
                }
                _ => Ok((&tokens[1..], Expr::Var(name))),
            }
        }
        other => Err(format!("Expected term, got {:?}", other)),
    }
}

fn parse_expression_rest(tokens: Tokens, left: Expr) -> ParseResult<Expr> {
    match tokens.first() {
        Some(Token::Symbol(character)) if "+-*/&|<>=".contains(*character) => {
            let operator = *character;
            let (remaining, right) = parse_term(&tokens[1..])?;
            parse_expression_rest(
                remaining,
                Expr::Binary(operator, Box::new(left), Box::new(right)),
            )
        }
        _ => Ok((tokens, left)),
    }
}

fn parse_expression(tokens: Tokens) -> ParseResult<Expr> {
    let (remaining, first) = parse_term(tokens)?;
    parse_expression_rest(remaining, first)
}

// Statements

fn parse_let(tokens: Tokens) -> ParseResult<Statement> {
    let (remaining, _) = expect_keyword(tokens, "let".to_string())?;
    let (remaining, var) = parse_identifier(remaining)?;
    let (remaining, index) = match remaining.first() {
        Some(Token::Symbol('[')) => {
            let (after_index, expression) = parse_expression(&remaining[1..])?;
            let (after_bracket, _) = expect_symbol(after_index, ']')?;
            (after_bracket, Some(expression))
        }
        _ => (remaining, None),
    };
    let (remaining, _) = expect_symbol(remaining, '=')?;
    let (remaining, value) = parse_expression(remaining)?;
    let (remaining, _) = expect_symbol(remaining, ';')?;
    Ok((remaining, Statement::Let { var, index, value }))
}

fn parse_if(tokens: Tokens) -> ParseResult<Statement> {
    let (remaining, _) = expect_keyword(tokens, "if".to_string())?;
    let (remaining, _) = expect_symbol(remaining, '(')?;
    let (remaining, condition) = parse_expression(remaining)?;
    let (remaining, _) = expect_symbol(remaining, ')')?;
    let (remaining, _) = expect_symbol(remaining, '{')?;
    let (remaining, then_body) = parse_statements(remaining)?;
    let (remaining, _) = expect_symbol(remaining, '}')?;
    let (remaining, else_body) = match remaining.first() {
        Some(Token::Keyword(keyword)) if keyword == "else" => {
            let (after_brace, _) = expect_symbol(&remaining[1..], '{')?;
            let (after_stmts, statements) = parse_statements(after_brace)?;
            let (after_close, _) = expect_symbol(after_stmts, '}')?;
            (after_close, Some(statements))
        }
        _ => (remaining, None),
    };
    Ok((
        remaining,
        Statement::If {
            condition,
            then_body,
            else_body,
        },
    ))
}

fn parse_while(tokens: Tokens) -> ParseResult<Statement> {
    let (remaining, _) = expect_keyword(tokens, "while".to_string())?;
    let (remaining, _) = expect_symbol(remaining, '(')?;
    let (remaining, condition) = parse_expression(remaining)?;
    let (remaining, _) = expect_symbol(remaining, ')')?;
    let (remaining, _) = expect_symbol(remaining, '{')?;
    let (remaining, body) = parse_statements(remaining)?;
    let (remaining, _) = expect_symbol(remaining, '}')?;
    Ok((remaining, Statement::While { condition, body }))
}

fn parse_do(tokens: Tokens) -> ParseResult<Statement> {
    let (remaining, _) = expect_keyword(tokens, "do".to_string())?;
    let (remaining, name) = parse_identifier(remaining)?;
    let (remaining, call) = parse_call_suffix(name, remaining)?;
    let (remaining, _) = expect_symbol(remaining, ';')?;
    Ok((remaining, Statement::Do(call)))
}

fn parse_return(tokens: Tokens) -> ParseResult<Statement> {
    let (remaining, _) = expect_keyword(tokens, "return".to_string())?;
    let (remaining, value) = match remaining.first() {
        Some(Token::Symbol(';')) => (remaining, None),
        _ => {
            let (after_expr, expression) = parse_expression(remaining)?;
            (after_expr, Some(expression))
        }
    };
    let (remaining, _) = expect_symbol(remaining, ';')?;
    Ok((remaining, Statement::Return(value)))
}

fn parse_statement(tokens: Tokens) -> ParseResult<Statement> {
    match tokens.first() {
        Some(Token::Keyword(keyword)) if keyword == "let" => parse_let(tokens),
        Some(Token::Keyword(keyword)) if keyword == "if" => parse_if(tokens),
        Some(Token::Keyword(keyword)) if keyword == "while" => parse_while(tokens),
        Some(Token::Keyword(keyword)) if keyword == "do" => parse_do(tokens),
        Some(Token::Keyword(keyword)) if keyword == "return" => parse_return(tokens),
        other => Err(format!("Expected statement keyword, got {:?}", other)),
    }
}

fn parse_statements_rest(
    tokens: Tokens,
    statements: BankersDeque<Statement>,
) -> ParseResult<BankersDeque<Statement>> {
    match tokens.first() {
        Some(Token::Keyword(keyword))
            if matches!(keyword.as_str(), "let" | "if" | "while" | "do" | "return") =>
        {
            let (remaining, statement) = parse_statement(tokens)?;
            parse_statements_rest(remaining, statements.push_back(statement))
        }
        _ => Ok((tokens, statements)),
    }
}

fn parse_statements(tokens: Tokens) -> ParseResult<BankersDeque<Statement>> {
    parse_statements_rest(tokens, BankersDeque::empty())
}

// Declarations

fn parse_var_dec(tokens: Tokens) -> ParseResult<VarDec> {
    let (remaining, _) = expect_keyword(tokens, "var".to_string())?;
    let (remaining, typ) = parse_type(remaining)?;
    let (remaining, names) = parse_var_names(remaining)?;
    let (remaining, _) = expect_symbol(remaining, ';')?;
    Ok((remaining, VarDec { typ, names }))
}

fn parse_var_decs_rest(
    tokens: Tokens,
    declarations: BankersDeque<VarDec>,
) -> ParseResult<BankersDeque<VarDec>> {
    match tokens.first() {
        Some(Token::Keyword(keyword)) if keyword == "var" => {
            let (remaining, variable_declaration) = parse_var_dec(tokens)?;
            parse_var_decs_rest(remaining, declarations.push_back(variable_declaration))
        }
        _ => Ok((tokens, declarations)),
    }
}

fn parse_class_var_dec(tokens: Tokens) -> ParseResult<ClassVarDec> {
    let (kind, remaining) = match tokens.first() {
        Some(Token::Keyword(keyword)) if keyword == "static" => (VarKind::Static, &tokens[1..]),
        Some(Token::Keyword(keyword)) if keyword == "field" => (VarKind::Field, &tokens[1..]),
        other => return Err(format!("Expected 'static' or 'field', got {:?}", other)),
    };
    let (remaining, typ) = parse_type(remaining)?;
    let (remaining, names) = parse_var_names(remaining)?;
    let (remaining, _) = expect_symbol(remaining, ';')?;
    Ok((remaining, ClassVarDec { kind, typ, names }))
}

fn parse_subroutine_body(
    tokens: Tokens,
) -> ParseResult<(BankersDeque<VarDec>, BankersDeque<Statement>)> {
    let (remaining, _) = expect_symbol(tokens, '{')?;
    let (remaining, locals) = parse_var_decs_rest(remaining, BankersDeque::empty())?;
    let (remaining, statements) = parse_statements(remaining)?;
    let (remaining, _) = expect_symbol(remaining, '}')?;
    Ok((remaining, (locals, statements)))
}

fn parse_subroutine_dec(tokens: Tokens) -> ParseResult<SubroutineDec> {
    let (kind, remaining) = match tokens.first() {
        Some(Token::Keyword(keyword)) if keyword == "constructor" => {
            (SubroutineKind::Constructor, &tokens[1..])
        }
        Some(Token::Keyword(keyword)) if keyword == "function" => {
            (SubroutineKind::Function, &tokens[1..])
        }
        Some(Token::Keyword(keyword)) if keyword == "method" => {
            (SubroutineKind::Method, &tokens[1..])
        }
        other => return Err(format!("Expected subroutine kind, got {:?}", other)),
    };
    let (remaining, _) = parse_return_type(remaining)?;
    let (remaining, name) = parse_identifier(remaining)?;
    let (remaining, _) = expect_symbol(remaining, '(')?;
    let (remaining, params) = parse_parameter_list(remaining)?;
    let (remaining, _) = expect_symbol(remaining, ')')?;
    let (remaining, (locals, body)) = parse_subroutine_body(remaining)?;
    Ok((
        remaining,
        SubroutineDec {
            kind,
            name,
            params,
            locals,
            body,
        },
    ))
}

// Class

fn parse_class_body(
    tokens: Tokens,
    class_var_decs: BankersDeque<ClassVarDec>,
    subroutines: BankersDeque<SubroutineDec>,
) -> ParseResult<(BankersDeque<ClassVarDec>, BankersDeque<SubroutineDec>)> {
    match tokens.first() {
        Some(Token::Keyword(keyword)) if matches!(keyword.as_str(), "static" | "field") => {
            let (remaining, class_var_dec) = parse_class_var_dec(tokens)?;
            parse_class_body(
                remaining,
                class_var_decs.push_back(class_var_dec),
                subroutines,
            )
        }
        Some(Token::Keyword(keyword))
            if matches!(keyword.as_str(), "constructor" | "function" | "method") =>
        {
            let (remaining, subroutine) = parse_subroutine_dec(tokens)?;
            parse_class_body(remaining, class_var_decs, subroutines.push_back(subroutine))
        }
        Some(Token::Symbol('}')) => Ok((&tokens[1..], (class_var_decs, subroutines))),
        other => Err(format!("Unexpected token in class body: {:?}", other)),
    }
}

pub fn parse_class(tokens: &[Token]) -> Result<Class, String> {
    let (remaining, _) = expect_keyword(tokens, "class".to_string())?;
    let (remaining, name) = parse_identifier(remaining)?;
    let (remaining, _) = expect_symbol(remaining, '{')?;
    let (remaining, (var_decs, subroutines)) =
        parse_class_body(remaining, BankersDeque::empty(), BankersDeque::empty())?;
    if !remaining.is_empty() {
        return Err(format!(
            "Unexpected tokens after class: {:?}",
            &remaining[..remaining.len().min(3)]
        ));
    }
    Ok(Class {
        name,
        var_decs,
        subroutines,
    })
}

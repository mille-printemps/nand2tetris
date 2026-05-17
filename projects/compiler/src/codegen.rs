use collections::catdeque::CatenableDeque;
use collections::deque::{BankersDeque, Deque};
use collections::Empty;

use crate::ast::*;
use crate::symbol_table::{Symbol, SymbolTable};

type Code = CatenableDeque<String>;

fn type_to_string(typ: &Type) -> String {
    match typ {
        Type::Int => "int".to_string(),
        Type::Char => "char".to_string(),
        Type::Boolean => "boolean".to_string(),
        Type::ClassName(name) => name.clone(),
    }
}

fn push_symbol(symbol: &Symbol) -> String {
    match symbol.kind {
        VarKind::Static => format!("push static {}", symbol.index),
        VarKind::Field => format!("push this {}", symbol.index),
        VarKind::Arg => format!("push argument {}", symbol.index),
        VarKind::Var => format!("push local {}", symbol.index),
    }
}

fn pop_symbol(symbol: &Symbol) -> String {
    match symbol.kind {
        VarKind::Static => format!("pop static {}", symbol.index),
        VarKind::Field => format!("pop this {}", symbol.index),
        VarKind::Arg => format!("pop argument {}", symbol.index),
        VarKind::Var => format!("pop local {}", symbol.index),
    }
}

// Expressions

fn compile_call(
    call: &SubroutineCall,
    table: &SymbolTable,
    class_name: &str,
    label_index: u16,
) -> (Code, u16) {
    match call {
        SubroutineCall::Simple(name, arguments) => {
            // Implicit method call on `this`
            let receiver_code = Code::empty().push_back("push pointer 0".to_string());
            let (arguments_code, label_index) =
                compile_expressions(arguments, table, class_name, label_index);
            let call_instruction = format!("call {}.{} {}", class_name, name, arguments.len() + 1);
            (
                receiver_code
                    .append(&arguments_code)
                    .push_back(call_instruction),
                label_index,
            )
        }
        SubroutineCall::Qualified(receiver, method, arguments) => {
            match table.lookup(receiver) {
                Some(symbol) => {
                    // Method call on an object stored in a variable
                    let receiver_type = symbol.typ.clone();
                    let receiver_code = Code::empty().push_back(push_symbol(symbol));
                    let (arguments_code, label_index) =
                        compile_expressions(arguments, table, class_name, label_index);
                    let call_instruction =
                        format!("call {}.{} {}", receiver_type, method, arguments.len() + 1);
                    (
                        receiver_code
                            .append(&arguments_code)
                            .push_back(call_instruction),
                        label_index,
                    )
                }
                None => {
                    // Function or constructor call on a class
                    let (arguments_code, label_index) =
                        compile_expressions(arguments, table, class_name, label_index);
                    let call_instruction =
                        format!("call {}.{} {}", receiver, method, arguments.len());
                    (arguments_code.push_back(call_instruction), label_index)
                }
            }
        }
    }
}

fn compile_expressions(
    expressions: &BankersDeque<Expr>,
    table: &SymbolTable,
    class_name: &str,
    label_index: u16,
) -> (Code, u16) {
    expressions.iter().fold(
        (Code::empty(), label_index),
        |(code, label_index), expression_ref| {
            let (expression_code, label_index) =
                compile_expression(expression_ref.as_ref(), table, class_name, label_index);
            (code.append(&expression_code), label_index)
        },
    )
}

fn compile_expression(
    expression: &Expr,
    table: &SymbolTable,
    class_name: &str,
    label_index: u16,
) -> (Code, u16) {
    match expression {
        Expr::IntConst(value) => (
            Code::empty().push_back(format!("push constant {}", value)),
            label_index,
        ),
        Expr::StrConst(value) => {
            let init_code = Code::empty()
                .push_back(format!("push constant {}", value.len()))
                .push_back("call String.new 1".to_string());
            let char_code = value.chars().fold(Code::empty(), |code, character| {
                code.push_back(format!("push constant {}", character as u16))
                    .push_back("call String.appendChar 2".to_string())
            });
            (init_code.append(&char_code), label_index)
        }
        Expr::True => (
            Code::empty()
                .push_back("push constant 1".to_string())
                .push_back("neg".to_string()),
            label_index,
        ),
        Expr::False | Expr::Null => (
            Code::empty().push_back("push constant 0".to_string()),
            label_index,
        ),
        Expr::This => (
            Code::empty().push_back("push pointer 0".to_string()),
            label_index,
        ),
        Expr::Var(name) => match table.lookup(name) {
            Some(symbol) => (Code::empty().push_back(push_symbol(symbol)), label_index),
            None => panic!("Undefined variable: {}", name),
        },
        Expr::Index(name, index_expression) => {
            let base_code = match table.lookup(name) {
                Some(symbol) => Code::empty().push_back(push_symbol(symbol)),
                None => panic!("Undefined variable: {}", name),
            };
            let (index_code, label_index) =
                compile_expression(index_expression, table, class_name, label_index);
            let access_code = Code::empty()
                .push_back("add".to_string())
                .push_back("pop pointer 1".to_string())
                .push_back("push that 0".to_string());
            (
                base_code.append(&index_code).append(&access_code),
                label_index,
            )
        }
        Expr::Call(call) => compile_call(call, table, class_name, label_index),
        Expr::Unary(operator, operand) => {
            let (operand_code, label_index) =
                compile_expression(operand, table, class_name, label_index);
            let instruction = match operator {
                '-' => "neg",
                '~' => "not",
                other => panic!("Unknown unary operator: {}", other),
            };
            (operand_code.push_back(instruction.to_string()), label_index)
        }
        Expr::Binary(operator, left, right) => {
            let (left_code, label_index) = compile_expression(left, table, class_name, label_index);
            let (right_code, label_index) =
                compile_expression(right, table, class_name, label_index);
            let instruction = match operator {
                '+' => "add".to_string(),
                '-' => "sub".to_string(),
                '&' => "and".to_string(),
                '|' => "or".to_string(),
                '<' => "lt".to_string(),
                '>' => "gt".to_string(),
                '=' => "eq".to_string(),
                '*' => "call Math.multiply 2".to_string(),
                '/' => "call Math.divide 2".to_string(),
                other => panic!("Unknown binary operator: {}", other),
            };
            (
                left_code.append(&right_code).push_back(instruction),
                label_index,
            )
        }
    }
}

// Statements

fn compile_statements(
    statements: &BankersDeque<Statement>,
    table: &SymbolTable,
    class_name: &str,
    label_index: u16,
) -> (Code, u16) {
    statements.iter().fold(
        (Code::empty(), label_index),
        |(code, label_index), statement_ref| {
            let (statement_code, label_index) =
                compile_statement(statement_ref.as_ref(), table, class_name, label_index);
            (code.append(&statement_code), label_index)
        },
    )
}

fn compile_statement(
    statement: &Statement,
    table: &SymbolTable,
    class_name: &str,
    label_index: u16,
) -> (Code, u16) {
    match statement {
        Statement::Let {
            var,
            index: None,
            value,
        } => {
            let (value_code, label_index) =
                compile_expression(value, table, class_name, label_index);
            let store = match table.lookup(var) {
                Some(symbol) => pop_symbol(symbol),
                None => panic!("Undefined variable: {}", var),
            };
            (value_code.push_back(store), label_index)
        }
        Statement::Let {
            var,
            index: Some(index_expression),
            value,
        } => {
            // Compute target address (base + index), then store expression result there.
            // Use temp 0 to hold the value across the pointer manipulation.
            let base_code = match table.lookup(var) {
                Some(symbol) => Code::empty().push_back(push_symbol(symbol)),
                None => panic!("Undefined variable: {}", var),
            };
            let (index_code, label_index) =
                compile_expression(index_expression, table, class_name, label_index);
            let (value_code, label_index) =
                compile_expression(value, table, class_name, label_index);
            let store_code = Code::empty()
                .push_back("add".to_string())
                .append(&value_code)
                .push_back("pop temp 0".to_string())
                .push_back("pop pointer 1".to_string())
                .push_back("push temp 0".to_string())
                .push_back("pop that 0".to_string());
            (
                base_code.append(&index_code).append(&store_code),
                label_index,
            )
        }
        Statement::If {
            condition,
            then_body,
            else_body,
        } => {
            let false_label = format!("IF_FALSE{}", label_index);
            let end_label = format!("IF_END{}", label_index);
            let label_index = label_index + 1;
            let (condition_code, label_index) =
                compile_expression(condition, table, class_name, label_index);
            let (then_code, label_index) =
                compile_statements(then_body, table, class_name, label_index);
            let jump_to_false = Code::empty()
                .push_back("not".to_string())
                .push_back(format!("if-goto {}", false_label));
            match else_body {
                None => {
                    let label_code = Code::empty().push_back(format!("label {}", false_label));
                    (
                        condition_code
                            .append(&jump_to_false)
                            .append(&then_code)
                            .append(&label_code),
                        label_index,
                    )
                }
                Some(else_statements) => {
                    let (else_code, label_index) =
                        compile_statements(else_statements, table, class_name, label_index);
                    let false_label_code =
                        Code::empty().push_back(format!("label {}", false_label));
                    let end_label_code = Code::empty().push_back(format!("label {}", end_label));
                    let goto_end = Code::empty().push_back(format!("goto {}", end_label));
                    (
                        condition_code
                            .append(&jump_to_false)
                            .append(&then_code)
                            .append(&goto_end)
                            .append(&false_label_code)
                            .append(&else_code)
                            .append(&end_label_code),
                        label_index,
                    )
                }
            }
        }
        Statement::While { condition, body } => {
            let top_label = format!("WHILE_EXP{}", label_index);
            let end_label = format!("WHILE_END{}", label_index);
            let label_index = label_index + 1;
            let (condition_code, label_index) =
                compile_expression(condition, table, class_name, label_index);
            let (body_code, label_index) = compile_statements(body, table, class_name, label_index);
            let header = Code::empty()
                .push_back(format!("label {}", top_label))
                .append(&condition_code)
                .push_back("not".to_string())
                .push_back(format!("if-goto {}", end_label));
            let footer = Code::empty()
                .push_back(format!("goto {}", top_label))
                .push_back(format!("label {}", end_label));
            (header.append(&body_code).append(&footer), label_index)
        }
        Statement::Do(call) => {
            let (call_code, label_index) = compile_call(call, table, class_name, label_index);
            // Discard the return value of void subroutine calls
            (call_code.push_back("pop temp 0".to_string()), label_index)
        }
        Statement::Return(None) => (
            Code::empty()
                .push_back("push constant 0".to_string())
                .push_back("return".to_string()),
            label_index,
        ),
        Statement::Return(Some(value)) => {
            let (value_code, label_index) =
                compile_expression(value, table, class_name, label_index);
            (value_code.push_back("return".to_string()), label_index)
        }
    }
}

// Subroutines

fn build_subroutine_table(
    subroutine: &SubroutineDec,
    class_table: &SymbolTable,
    class_name: &str,
) -> SymbolTable {
    let table = class_table.reset_subroutine();

    // Methods receive `this` as implicit argument 0
    let table = match subroutine.kind {
        SubroutineKind::Method => {
            table.define("this".to_string(), class_name.to_string(), VarKind::Arg)
        }
        _ => table,
    };

    let table = subroutine
        .params
        .iter()
        .fold(table, |table, parameter_ref| {
            let parameter = parameter_ref.as_ref();
            table.define(
                parameter.name.clone(),
                type_to_string(&parameter.typ),
                VarKind::Arg,
            )
        });

    subroutine.locals.iter().fold(table, |table, var_dec_ref| {
        let var_dec = var_dec_ref.as_ref();
        var_dec.names.iter().fold(table, |table, name_ref| {
            table.define(
                name_ref.as_ref().clone(),
                type_to_string(&var_dec.typ),
                VarKind::Var,
            )
        })
    })
}

fn compile_subroutine(
    subroutine: &SubroutineDec,
    class_table: &SymbolTable,
    class_name: &str,
) -> Code {
    let table = build_subroutine_table(subroutine, class_table, class_name);
    let function_declaration = format!(
        "function {}.{} {}",
        class_name, subroutine.name, table.local_count
    );

    let prologue = match subroutine.kind {
        SubroutineKind::Constructor => Code::empty()
            .push_back(function_declaration)
            .push_back(format!("push constant {}", class_table.field_count))
            .push_back("call Memory.alloc 1".to_string())
            .push_back("pop pointer 0".to_string()),
        SubroutineKind::Method => Code::empty()
            .push_back(function_declaration)
            .push_back("push argument 0".to_string())
            .push_back("pop pointer 0".to_string()),
        SubroutineKind::Function => Code::empty().push_back(function_declaration),
    };

    let (body_code, _) = compile_statements(&subroutine.body, &table, class_name, 0);
    prologue.append(&body_code)
}

// Class

pub fn compile_class(class: &Class) -> Code {
    let class_table = class
        .var_decs
        .iter()
        .fold(SymbolTable::new(), |table, class_var_dec_ref| {
            let class_var_dec = class_var_dec_ref.as_ref();
            class_var_dec.names.iter().fold(table, |table, name_ref| {
                table.define(
                    name_ref.as_ref().clone(),
                    type_to_string(&class_var_dec.typ),
                    class_var_dec.kind.clone(),
                )
            })
        });

    class
        .subroutines
        .iter()
        .fold(Code::empty(), |code, subroutine_ref| {
            let subroutine_code =
                compile_subroutine(subroutine_ref.as_ref(), &class_table, &class.name);
            code.append(&subroutine_code)
        })
}

use collections::deque::BankersDeque;

#[derive(Clone, PartialEq, Debug)]
pub enum Type {
    Int,
    Char,
    Boolean,
    ClassName(String),
}

#[derive(Clone, PartialEq, Debug)]
pub enum VarKind {
    Static,
    Field,
    Arg,
    Var,
}

#[derive(Clone, Debug)]
pub struct ClassVarDec {
    pub kind: VarKind,
    pub typ: Type,
    pub names: BankersDeque<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum SubroutineKind {
    Constructor,
    Function,
    Method,
}

#[derive(Clone, Debug)]
pub struct Parameter {
    pub typ: Type,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct VarDec {
    pub typ: Type,
    pub names: BankersDeque<String>,
}

#[derive(Clone, Debug)]
pub enum Expr {
    IntConst(u16),
    StrConst(String),
    True,
    False,
    Null,
    This,
    Var(String),
    Index(String, Box<Expr>),
    Call(SubroutineCall),
    Unary(char, Box<Expr>),
    Binary(char, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Debug)]
pub enum SubroutineCall {
    /// `subroutineName(args)` — implicitly a method call on `this`
    Simple(String, BankersDeque<Expr>),
    /// `receiver.subroutineName(args)` — method on an object or function on a class
    Qualified(String, String, BankersDeque<Expr>),
}

#[derive(Clone, Debug)]
pub enum Statement {
    Let {
        var: String,
        index: Option<Expr>,
        value: Expr,
    },
    If {
        condition: Expr,
        then_body: BankersDeque<Statement>,
        else_body: Option<BankersDeque<Statement>>,
    },
    While {
        condition: Expr,
        body: BankersDeque<Statement>,
    },
    Do(SubroutineCall),
    Return(Option<Expr>),
}

#[derive(Clone, Debug)]
pub struct SubroutineDec {
    pub kind: SubroutineKind,
    pub name: String,
    pub params: BankersDeque<Parameter>,
    pub locals: BankersDeque<VarDec>,
    pub body: BankersDeque<Statement>,
}

#[derive(Clone, Debug)]
pub struct Class {
    pub name: String,
    pub var_decs: BankersDeque<ClassVarDec>,
    pub subroutines: BankersDeque<SubroutineDec>,
}

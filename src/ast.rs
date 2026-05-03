/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: A high-fidelity port of the historic AWK language from C to Rust.
 */

#[derive(Debug, Clone)]
pub enum BinaryOperator {
    Add, Sub, Mul, Div, Mod, Pow,
    Eq, Neq, Lt, Lte, Gt, Gte,
    And, Or,
    Match, NotMatch, In,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Field(Box<Expr>),
    NumberLiteral(f64),
    StringLiteral(String),
    RegexLiteral(String),
    Variable(String),
    ArrayAccess(String, Vec<Expr>), // a["key1", "key2"]
    FunctionCall(String, Vec<Expr>), // length($1)
    Getline(Option<String>, Option<Box<Expr>>), // getline var < file
    BinaryOp(Box<Expr>, BinaryOperator, Box<Expr>),
    Concat(Vec<Expr>),
    UnaryMinus(Box<Expr>),
    UnaryPlus(Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>), // cond ? true_expr : false_expr
    PreInc(Box<Expr>),
    PreDec(Box<Expr>),
    PostInc(Box<Expr>),
    PostDec(Box<Expr>),
    Not(Box<Expr>), // !expr
}

#[derive(Debug, Clone)]
pub enum Statement {
    Print(Vec<Expr>, Option<(String, Expr)>),
    Printf(Vec<Expr>, Option<(String, Expr)>),
    Assign(String, Expr),
    AssignArray(String, Vec<Expr>, Expr), // a[key1, key2] = value
    AssignField(Box<Expr>, Expr), // $i = value
    Delete(String, Option<Vec<Expr>>), // delete a[key] or delete a
    IfElse(Expr, Vec<Statement>, Option<Vec<Statement>>),
    ForIn(String, String, Vec<Statement>),
    For(Option<Box<Statement>>, Option<Expr>, Option<Box<Statement>>, Vec<Statement>),
    While(Expr, Vec<Statement>),
    DoWhile(Vec<Statement>, Expr),
    Break,
    Continue,
    Next,
    NextFile,
    Return(Option<Expr>),
    Exit(Option<Expr>),
    Expr(Expr),
}


#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Expr(Expr),
    Begin,
    End,
    BeginFile,
    EndFile,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub pattern: Option<Pattern>,
    pub action: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub rules: Vec<Rule>,
    pub functions: Vec<FunctionDecl>,
}

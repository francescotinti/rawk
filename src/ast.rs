/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: A high-fidelity port of the historic AWK language from C to Rust.
 */

/// Operatori binari supportati da AWK: aritmetici, di confronto, logici, regex-match
/// e l'operatore d'appartenenza array (`In`).
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    And,
    Or,
    Match,
    NotMatch,
    In,
}

/// Sorgente da cui leggere per la statement `getline`.
///
/// - `Main`: input principale (file argv o stdin).
/// - `File`: redirezione `getline < file`.
/// - `Pipe`: redirezione `"cmd" | getline`.
#[derive(Debug, Clone, PartialEq)]
pub enum GetlineSource {
    Main,
    File(Box<Expr>),
    Pipe(Box<Expr>),
}

/// Nodo espressione dell'AST AWK. Coprire l'intero linguaggio richiede
/// varianti per letterali, accessi a campo/variabile/array, chiamate
/// di funzione, operatori e i tre tipi di `getline`.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Field(Box<Expr>),
    NumberLiteral(f64),
    StringLiteral(Vec<u8>),
    RegexLiteral(Vec<u8>),
    Variable(String),
    ArrayAccess(String, Vec<Expr>),         // a["key1", "key2"]
    FunctionCall(String, Vec<Expr>),        // length($1)
    Getline(Option<String>, GetlineSource), // getline var < file / "cmd" | getline
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

/// Nodo statement dell'AST AWK. Le varianti `Break`/`Continue`/`Next`/`NextFile`/
/// `Return`/`Exit` propagano flow-control tramite `FlowControl` nel runner.
#[derive(Debug, Clone)]
pub enum Statement {
    Print(Vec<Expr>, Option<(String, Expr)>),
    Printf(Vec<Expr>, Option<(String, Expr)>),
    Assign(String, Expr),
    AssignArray(String, Vec<Expr>, Expr), // a[key1, key2] = value
    AssignField(Box<Expr>, Expr),         // $i = value
    Delete(String, Option<Vec<Expr>>),    // delete a[key] or delete a
    IfElse(Expr, Vec<Statement>, Option<Vec<Statement>>),
    ForIn(String, String, Vec<Statement>),
    For(
        Option<Box<Statement>>,
        Option<Expr>,
        Option<Box<Statement>>,
        Vec<Statement>,
    ),
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

/// Dichiarazione di una funzione utente AWK (`function nome(args) { body }`).
#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Statement>,
}

/// Pattern di una regola AWK. `Expr` matcha quando l'espressione è truthy
/// per la riga corrente; `Begin`/`End` eseguono prima/dopo l'input;
/// `BeginFile`/`EndFile` agli stessi confini per ciascun file argv.
#[derive(Debug, Clone)]
pub enum Pattern {
    Expr(Expr),
    Begin,
    End,
    BeginFile,
    EndFile,
}

/// Singola regola AWK: coppia opzionale `pattern { action }`.
/// Se `pattern` è `None`, l'azione viene eseguita per ogni record.
#[derive(Debug, Clone)]
pub struct Rule {
    pub pattern: Option<Pattern>,
    pub action: Vec<Statement>,
}

/// Programma AWK completo: una lista ordinata di regole più le
/// funzioni utente dichiarate (visibili globalmente, ordine irrilevante).
#[derive(Debug, Clone)]
pub struct Program {
    pub rules: Vec<Rule>,
    pub functions: Vec<FunctionDecl>,
}

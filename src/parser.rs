/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: A high-fidelity port of the historic AWK language from C to Rust.
 */

use crate::ast::{
    BinaryOperator, Expr, FunctionDecl, Pattern, Program, Rule as AstRule, Statement,
};
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "awk.pest"]
pub struct AwkParser;

pub fn parse(input: &str) -> anyhow::Result<Program> {
    let mut parsed = AwkParser::parse(Rule::program, input)
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    let program_pair = parsed.next().unwrap();

    let mut rules = Vec::new();
    let mut functions = Vec::new();
    for pair in program_pair.into_inner() {
        if pair.as_rule() == Rule::rule {
            rules.push(parse_rule(pair));
        } else if pair.as_rule() == Rule::function_decl {
            functions.push(parse_function_decl(pair));
        }
    }

    Ok(Program { rules, functions })
}

fn parse_function_decl(pair: Pair<Rule>) -> FunctionDecl {
    let mut inners = pair.into_inner();
    let name = inners.next().unwrap().as_str().to_string();

    let mut params = Vec::new();
    let mut body = Vec::new();

    if let Some(next_pair) = inners.next() {
        if next_pair.as_rule() == Rule::ident_list {
            for ident in next_pair.into_inner() {
                params.push(ident.as_str().to_string());
            }
            body = parse_action_block(inners.next().unwrap());
        } else {
            body = parse_action_block(next_pair);
        }
    }

    FunctionDecl { name, params, body }
}

fn parse_rule(pair: Pair<Rule>) -> AstRule {
    let mut pattern = None;
    let mut action = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::pattern => {
                pattern = Some(parse_pattern(inner));
            }
            Rule::action_block => {
                action = parse_action_block(inner);
            }
            _ => {}
        }
    }

    // If no action block but we have a pattern, standard awk does print $0
    if action.is_empty() && pattern.is_some() {
        action.push(Statement::Print(
            vec![Expr::Field(Box::new(Expr::StringLiteral("0".to_string())))],
            None,
        ));
    }

    AstRule { pattern, action }
}

fn parse_pattern(pair: Pair<Rule>) -> Pattern {
    let s = pair.as_str().trim();
    if s == "BEGIN" {
        return Pattern::Begin;
    } else if s == "END" {
        return Pattern::End;
    } else if s == "BEGINFILE" {
        return Pattern::BeginFile;
    } else if s == "ENDFILE" {
        return Pattern::EndFile;
    }

    let inner = pair.into_inner().next().unwrap();
    if inner.as_rule() == Rule::regex_pattern {
        let re = inner.into_inner().next().unwrap().as_str();
        Pattern::Expr(Expr::RegexLiteral(re.to_string()))
    } else {
        Pattern::Expr(parse_expr(inner))
    }
}

fn parse_action_block(pair: Pair<Rule>) -> Vec<Statement> {
    let mut stmts = Vec::new();
    for stmt in pair.into_inner() {
        stmts.push(parse_statement(stmt));
    }
    stmts
}

fn parse_block_or_stmt(pair: Pair<Rule>) -> Vec<Statement> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::action_block => parse_action_block(inner),
        Rule::statement => vec![parse_statement(inner)],
        _ => vec![],
    }
}

fn parse_statement(pair: Pair<Rule>) -> Statement {
    let inner = pair.clone().into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::if_stmt => {
            let mut inners = inner.into_inner();
            let expr = parse_expr(inners.next().unwrap());
            let true_block = parse_block_or_stmt(inners.next().unwrap());
            let false_block = inners.next().map(parse_block_or_stmt);
            Statement::IfElse(expr, true_block, false_block)
        }
        Rule::while_stmt => {
            let mut inners = inner.into_inner();
            let expr = parse_expr(inners.next().unwrap());
            let block = parse_block_or_stmt(inners.next().unwrap());
            Statement::While(expr, block)
        }
        Rule::do_while_stmt => {
            let mut inners = inner.into_inner();
            let block = parse_block_or_stmt(inners.next().unwrap());
            let expr = parse_expr(inners.next().unwrap());
            Statement::DoWhile(block, expr)
        }
        Rule::break_stmt => Statement::Break,
        Rule::continue_stmt => Statement::Continue,
        Rule::nextfile_stmt => Statement::NextFile,
        Rule::next_stmt => Statement::Next,
        Rule::return_stmt => {
            let mut inners = inner.into_inner();
            if let Some(expr_inner) = inners.next() {
                Statement::Return(Some(parse_expr(expr_inner)))
            } else {
                Statement::Return(None)
            }
        }
        Rule::exit_stmt => {
            let mut inners = inner.into_inner();
            if let Some(expr_inner) = inners.next() {
                Statement::Exit(Some(parse_expr(expr_inner)))
            } else {
                Statement::Exit(None)
            }
        }
        Rule::for_in_stmt => {
            let mut inners = inner.into_inner();
            let var = inners.next().unwrap().as_str().to_string();
            let arr = inners.next().unwrap().as_str().to_string();
            let block = parse_block_or_stmt(inners.next().unwrap());
            Statement::ForIn(var, arr, block)
        }
        Rule::for_stmt => {
            let inners = inner.into_inner();
            let mut init = None;
            let mut cond = None;
            let mut step = None;
            let mut block = Vec::new();

            for inner in inners {
                match inner.as_rule() {
                    Rule::for_init => {
                        let node = inner.into_inner().next().unwrap();
                        let s = if node.as_rule() == Rule::assign_stmt {
                            parse_assign_stmt(node)
                        } else {
                            Statement::Expr(parse_expr(node))
                        };
                        init = Some(Box::new(s));
                    }
                    Rule::for_cond => {
                        cond = Some(parse_expr(inner.into_inner().next().unwrap()));
                    }
                    Rule::for_step => {
                        let node = inner.into_inner().next().unwrap();
                        let s = if node.as_rule() == Rule::assign_stmt {
                            parse_assign_stmt(node)
                        } else {
                            Statement::Expr(parse_expr(node))
                        };
                        step = Some(Box::new(s));
                    }
                    Rule::block_or_stmt => {
                        block = parse_block_or_stmt(inner);
                    }
                    _ => {}
                }
            }
            Statement::For(init, cond, step, block)
        }
        Rule::delete_stmt => {
            let mut inners = inner.into_inner();
            let arr_name = inners.next().unwrap().as_str().to_string();
            let mut keys = None;
            if let Some(expr_list) = inners.next() {
                let mut k = Vec::new();
                for e in expr_list.into_inner() {
                    k.push(parse_expr(e));
                }
                keys = Some(k);
            }
            Statement::Delete(arr_name, keys)
        }
        Rule::print_stmt | Rule::printf_stmt => {
            let mut exprs = Vec::new();
            let mut redirect = None;
            let is_printf = inner.as_rule() == Rule::printf_stmt;
            for p in inner.into_inner() {
                match p.as_rule() {
                    Rule::expr_list | Rule::print_expr_list => {
                        for e in p.into_inner() {
                            exprs.push(parse_expr(e));
                        }
                    }
                    Rule::redirect => {
                        let p_str = p.as_str();
                        let mut r_inners = p.into_inner();
                        let op = if p_str.starts_with(">>") {
                            ">>".to_string()
                        } else if p_str.starts_with(">") {
                            ">".to_string()
                        } else {
                            "|".to_string()
                        };
                        let r_expr = parse_expr(r_inners.next().unwrap());
                        redirect = Some((op, r_expr));
                    }
                    _ => {}
                }
            }
            if exprs.is_empty() && !is_printf {
                exprs.push(Expr::Field(Box::new(Expr::StringLiteral("0".to_string()))));
            }
            if is_printf {
                Statement::Printf(exprs, redirect)
            } else {
                Statement::Print(exprs, redirect)
            }
        }
        Rule::assign_stmt => parse_assign_stmt(inner),
        Rule::expr_stmt => {
            let e = parse_expr(inner.into_inner().next().unwrap());
            Statement::Expr(e)
        }
        _ => unreachable!(
            "Unhandled statement rule: pair={:?}, inner={:?}",
            pair.as_rule(),
            inner.as_rule()
        ),
    }
}

fn parse_assign_stmt(inner: Pair<Rule>) -> Statement {
    let mut inners = inner.into_inner();
    let target = inners.next().unwrap();
    let op_str = inners.next().unwrap().as_str();
    let mut expr = parse_expr(inners.next().unwrap());

    let target_inner = target.clone().into_inner().next().unwrap();
    let op = match op_str {
        "+=" => Some(BinaryOperator::Add),
        "-=" => Some(BinaryOperator::Sub),
        "*=" => Some(BinaryOperator::Mul),
        "/=" => Some(BinaryOperator::Div),
        _ => None,
    };

    let target_expr = match target_inner.as_rule() {
        Rule::ident => Expr::Variable(target_inner.as_str().to_string()),
        Rule::array_access => {
            let mut a_inners = target_inner.clone().into_inner();
            let arr_name = a_inners.next().unwrap().as_str().to_string();
            let mut keys = Vec::new();
            for key_pair in a_inners.next().unwrap().into_inner() {
                keys.push(parse_expr(key_pair));
            }
            Expr::ArrayAccess(arr_name, keys)
        }
        Rule::field => {
            let p = target_inner.into_inner().next().unwrap();
            Expr::Field(Box::new(parse_primary(p)))
        }
        _ => Expr::Variable("err".to_string()),
    };

    if let Some(op) = op {
        expr = Expr::BinaryOp(Box::new(target_expr.clone()), op, Box::new(expr));
    }

    match target_expr {
        Expr::Variable(v) => Statement::Assign(v, expr),
        Expr::ArrayAccess(arr, ks) => Statement::AssignArray(arr, ks, expr),
        Expr::Field(e) => Statement::AssignField(e, expr),
        _ => Statement::Expr(expr),
    }
}

fn parse_expr(pair: Pair<Rule>) -> Expr {
    parse_ternary_expr(pair.into_inner().next().unwrap())
}

fn parse_ternary_expr(pair: Pair<Rule>) -> Expr {
    let mut inners = pair.into_inner();
    let logical_or = parse_logical_or(inners.next().unwrap());
    if let Some(true_expr_pair) = inners.next() {
        let false_expr_pair = inners.next().unwrap();
        Expr::Ternary(
            Box::new(logical_or),
            Box::new(parse_expr(true_expr_pair)),
            Box::new(parse_expr(false_expr_pair)),
        )
    } else {
        logical_or
    }
}

fn parse_logical_or(pair: Pair<Rule>) -> Expr {
    let mut inners = pair.into_inner();
    let mut lhs = parse_logical_and(inners.next().unwrap());
    while let Some(_) = inners.next() {
        // op_or
        let rhs = parse_logical_and(inners.next().unwrap());
        lhs = Expr::BinaryOp(Box::new(lhs), BinaryOperator::Or, Box::new(rhs));
    }
    lhs
}

fn parse_logical_and(pair: Pair<Rule>) -> Expr {
    let mut inners = pair.into_inner();
    let mut lhs = parse_in_expr(inners.next().unwrap());
    while let Some(_) = inners.next() {
        // op_and
        let rhs = parse_in_expr(inners.next().unwrap());
        lhs = Expr::BinaryOp(Box::new(lhs), BinaryOperator::And, Box::new(rhs));
    }
    lhs
}

fn parse_in_expr(pair: Pair<Rule>) -> Expr {
    let mut inners = pair.into_inner();
    let lhs = parse_match_expr(inners.next().unwrap());
    if let Some(_) = inners.next() {
        // op_in
        let rhs_ident = inners.next().unwrap();
        Expr::BinaryOp(
            Box::new(lhs),
            BinaryOperator::In,
            Box::new(Expr::Variable(rhs_ident.as_str().to_string())),
        )
    } else {
        lhs
    }
}

fn parse_match_expr(pair: Pair<Rule>) -> Expr {
    let mut inners = pair.into_inner();
    let mut lhs = parse_rel_expr(inners.next().unwrap());
    while let Some(op) = inners.next() {
        let rhs = parse_rel_expr(inners.next().unwrap());
        let bop = match op.as_rule() {
            Rule::op_match => BinaryOperator::Match,
            Rule::op_not_match => BinaryOperator::NotMatch,
            _ => unreachable!(),
        };
        lhs = Expr::BinaryOp(Box::new(lhs), bop, Box::new(rhs));
    }
    lhs
}

fn parse_rel_expr(pair: Pair<Rule>) -> Expr {
    let mut inners = pair.into_inner();
    let mut lhs = parse_concat_expr(inners.next().unwrap());
    while let Some(op) = inners.next() {
        let rhs = parse_concat_expr(inners.next().unwrap());
        let bop = match op.as_rule() {
            Rule::op_eq => BinaryOperator::Eq,
            Rule::op_neq => BinaryOperator::Neq,
            Rule::op_lt => BinaryOperator::Lt,
            Rule::op_le => BinaryOperator::Lte,
            Rule::op_gt => BinaryOperator::Gt,
            Rule::op_ge => BinaryOperator::Gte,
            _ => unreachable!(),
        };
        lhs = Expr::BinaryOp(Box::new(lhs), bop, Box::new(rhs));
    }
    lhs
}

fn parse_concat_expr(pair: Pair<Rule>) -> Expr {
    let parts: Vec<Expr> = pair.into_inner().map(parse_add_expr).collect();
    if parts.len() == 1 {
        parts.into_iter().next().unwrap()
    } else {
        Expr::Concat(parts)
    }
}

fn parse_add_expr(pair: Pair<Rule>) -> Expr {
    let mut inners = pair.into_inner();
    let mut lhs = parse_mul_expr(inners.next().unwrap());
    while let Some(op) = inners.next() {
        let rhs = parse_mul_expr(inners.next().unwrap());
        let bop = match op.as_rule() {
            Rule::op_add => BinaryOperator::Add,
            Rule::op_sub => BinaryOperator::Sub,
            _ => unreachable!(),
        };
        lhs = Expr::BinaryOp(Box::new(lhs), bop, Box::new(rhs));
    }
    lhs
}

fn parse_mul_expr(pair: Pair<Rule>) -> Expr {
    let mut inners = pair.into_inner();
    let mut lhs = parse_pow_expr(inners.next().unwrap());
    while let Some(op) = inners.next() {
        let rhs = parse_pow_expr(inners.next().unwrap());
        let bop = match op.as_rule() {
            Rule::op_mul => BinaryOperator::Mul,
            Rule::op_div => BinaryOperator::Div,
            Rule::op_mod => BinaryOperator::Mod,
            _ => unreachable!(),
        };
        lhs = Expr::BinaryOp(Box::new(lhs), bop, Box::new(rhs));
    }
    lhs
}

fn parse_pow_expr(pair: Pair<Rule>) -> Expr {
    let mut inners = pair.into_inner();
    let mut lhs = parse_term(inners.next().unwrap());
    if let Some(_op) = inners.next() {
        let rhs = parse_pow_expr(inners.next().unwrap());
        lhs = Expr::BinaryOp(Box::new(lhs), BinaryOperator::Pow, Box::new(rhs));
    }
    lhs
}

fn parse_term(term: Pair<Rule>) -> Expr {
    let mut term_inners = term.into_inner();
    let mut primary_pair = term_inners.next().unwrap();

    let mut prefix = None;
    if primary_pair.as_rule() != Rule::primary {
        prefix = Some(primary_pair.as_rule());
        primary_pair = term_inners.next().unwrap_or_else(|| {
            panic!(
                "Expected primary after prefix {:?}, but got nothing",
                prefix.unwrap()
            )
        });
    }

    let mut base_expr = parse_primary(primary_pair);

    if let Some(postfix_pair) = term_inners.next() {
        if postfix_pair.as_rule() == Rule::op_inc {
            base_expr = Expr::PostInc(Box::new(base_expr));
        } else if postfix_pair.as_rule() == Rule::op_dec {
            base_expr = Expr::PostDec(Box::new(base_expr));
        }
    }

    if let Some(pre) = prefix {
        base_expr = match pre {
            Rule::op_inc => Expr::PreInc(Box::new(base_expr)),
            Rule::op_dec => Expr::PreDec(Box::new(base_expr)),
            Rule::op_not => Expr::Not(Box::new(base_expr)),
            Rule::op_minus => Expr::UnaryMinus(Box::new(base_expr)),
            Rule::op_plus => Expr::UnaryPlus(Box::new(base_expr)),
            _ => base_expr,
        };
    }

    base_expr
}

pub fn decode_string_escapes(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.peek().copied() {
            Some('n') => {
                chars.next();
                out.push('\n');
            }
            Some('t') => {
                chars.next();
                out.push('\t');
            }
            Some('r') => {
                chars.next();
                out.push('\r');
            }
            Some('\\') => {
                chars.next();
                out.push('\\');
            }
            Some('"') => {
                chars.next();
                out.push('"');
            }
            Some('/') => {
                chars.next();
                out.push('/');
            }
            Some('a') => {
                chars.next();
                out.push('\x07');
            }
            Some('b') => {
                chars.next();
                out.push('\x08');
            }
            Some('f') => {
                chars.next();
                out.push('\x0c');
            }
            Some('v') => {
                chars.next();
                out.push('\x0b');
            }
            Some('x') => {
                chars.next();
                let mut hex = String::new();
                for _ in 0..2 {
                    if let Some(&h) = chars.peek() {
                        if h.is_ascii_hexdigit() {
                            hex.push(h);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }
                if !hex.is_empty() {
                    let val = u32::from_str_radix(&hex, 16).unwrap_or(0);
                    out.push(char::from_u32(val).unwrap_or('\0'));
                } else {
                    out.push('\\');
                    out.push('x');
                }
            }
            Some(d) if d.is_digit(8) => {
                let mut oct = String::new();
                for _ in 0..3 {
                    if let Some(&o) = chars.peek() {
                        if o.is_digit(8) {
                            oct.push(o);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }
                let val = u32::from_str_radix(&oct, 8).unwrap_or(0) % 256;
                out.push(char::from_u32(val).unwrap_or('\0'));
            }
            Some(other) => {
                chars.next();
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

fn parse_primary(primary_pair: Pair<Rule>) -> Expr {
    let mut inner = primary_pair.into_inner().next().unwrap();
    if inner.as_rule() == Rule::non_getline_primary {
        inner = inner.into_inner().next().unwrap();
    }
    parse_primary_inner(inner)
}

fn parse_primary_inner(inner: Pair<Rule>) -> Expr {
    match inner.as_rule() {
        Rule::number => Expr::NumberLiteral(inner.as_str().parse::<f64>().unwrap_or(0.0)),
        Rule::string_literal => Expr::StringLiteral(decode_string_escapes(
            inner.into_inner().next().unwrap().as_str(),
        )),
        Rule::regex_pattern => {
            let re = inner.into_inner().next().unwrap().as_str();
            Expr::RegexLiteral(re.to_string())
        }
        Rule::ident => Expr::Variable(inner.as_str().to_string()),
        Rule::getline_expr => {
            let actual = inner.into_inner().next().unwrap();
            if actual.as_rule() == Rule::plain_getline {
                let inners = actual.into_inner();
                let mut var_name = None;
                let mut file_expr = None;
                for p in inners {
                    if p.as_rule() == Rule::ident {
                        var_name = Some(p.as_str().to_string());
                    } else if p.as_rule() == Rule::expr {
                        file_expr = Some(Box::new(parse_expr(p)));
                    }
                }
                let source = if let Some(fe) = file_expr {
                    crate::ast::GetlineSource::File(fe)
                } else {
                    crate::ast::GetlineSource::Main
                };
                Expr::Getline(var_name, source)
            } else {
                // pipe_getline
                let mut inners = actual.into_inner();
                let cmd_primary = inners.next().unwrap(); // non_getline_primary
                let cmd_inner = cmd_primary.into_inner().next().unwrap();
                let cmd_expr = parse_primary_inner(cmd_inner);
                let mut var_name = None;
                for p in inners {
                    if p.as_rule() == Rule::ident {
                        var_name = Some(p.as_str().to_string());
                    }
                }
                Expr::Getline(
                    var_name,
                    crate::ast::GetlineSource::Pipe(Box::new(cmd_expr)),
                )
            }
        }
        Rule::field => {
            let p = inner.into_inner().next().unwrap();
            Expr::Field(Box::new(parse_primary(p)))
        }
        Rule::array_access => {
            let mut inners = inner.into_inner();
            let ident = inners.next().unwrap().as_str().to_string();
            let mut keys = Vec::new();
            for key_pair in inners.next().unwrap().into_inner() {
                keys.push(parse_expr(key_pair));
            }
            Expr::ArrayAccess(ident, keys)
        }
        Rule::func_call => {
            let mut inners = inner.into_inner();
            let func_name_str = inners.next().unwrap().as_str();
            let ident = func_name_str[..func_name_str.len() - 1].to_string();
            let mut args = Vec::new();
            if let Some(expr_list) = inners.next() {
                for e in expr_list.into_inner() {
                    args.push(parse_expr(e));
                }
            }
            Expr::FunctionCall(ident, args)
        }
        Rule::expr => parse_expr(inner),
        _ => unreachable!("Unexpected primary inner: {:?}", inner.as_rule()),
    }
}

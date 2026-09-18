//! Validate operations before executing BEGIN, including unreachable branches.
use crate::ast::{Expr, GetlineSource, Pattern, Program, Statement};
use anyhow::{Result, bail};

pub fn validate(program: &Program, safe: bool) -> Result<()> {
    for rule in &program.rules {
        match &rule.pattern {
            Some(Pattern::Expr(expr)) => expression(expr, safe)?,
            Some(Pattern::Range(a, b)) => {
                expression(a, safe)?;
                expression(b, safe)?;
            }
            _ => {}
        }
        statements(&rule.action, safe)?;
        control_context(
            &rule.action,
            false,
            0,
            matches!(
                rule.pattern,
                Some(Pattern::Begin | Pattern::End | Pattern::BeginFile | Pattern::EndFile)
            ),
        )?;
    }
    for function in &program.functions {
        statements(&function.body, safe)?;
        control_context(&function.body, true, 0, false)?;
    }
    Ok(())
}

fn expression(expr: &Expr, safe: bool) -> Result<()> {
    match expr {
        Expr::FunctionCall(name, args) => {
            if safe && name == "system" {
                bail!("system is unsafe in safe mode");
            }
            let bounds = match name.as_str() {
                "sin" | "cos" | "exp" | "log" | "sqrt" | "int" | "tolower" | "toupper"
                | "system" | "close" => Some((1, 1)),
                "index" | "atan2" | "and" | "or" | "xor" | "lshift" | "rshift" | "match" => {
                    Some((2, 2))
                }
                "substr" | "split" | "sub" | "gsub" => Some((2, 3)),
                "length" | "srand" | "fflush" => Some((0, 1)),
                "rand" | "systime" => Some((0, 0)),
                "strftime" => Some((0, 3)),
                "sprintf" => Some((1, usize::MAX)),
                _ => None,
            };
            if let Some((min, max)) = bounds
                && !(min..=max).contains(&args.len())
            {
                bail!("invalid number of arguments to {name}: {}", args.len());
            }
            for arg in args {
                expression(arg, safe)?;
            }
        }
        Expr::Getline(_, source) => match source {
            GetlineSource::Main => {}
            GetlineSource::File(expr) => expression(expr, safe)?,
            GetlineSource::Pipe(expr) => {
                if safe {
                    bail!("command pipe is unsafe in safe mode");
                }
                expression(expr, safe)?;
            }
        },
        Expr::Field(e)
        | Expr::UnaryMinus(e)
        | Expr::UnaryPlus(e)
        | Expr::PreInc(e)
        | Expr::PreDec(e)
        | Expr::PostInc(e)
        | Expr::PostDec(e)
        | Expr::Not(e) => expression(e, safe)?,
        Expr::BinaryOp(a, _, b) | Expr::Assign(a, _, b) => {
            expression(a, safe)?;
            expression(b, safe)?;
        }
        Expr::Ternary(a, b, c) => {
            expression(a, safe)?;
            expression(b, safe)?;
            expression(c, safe)?;
        }
        Expr::Concat(args) | Expr::ArrayAccess(_, args) => {
            for arg in args {
                expression(arg, safe)?;
            }
        }
        Expr::RegexLiteral(pattern) => {
            crate::ere::Ere::new(pattern).map_err(anyhow::Error::msg)?;
        }
        Expr::NumberLiteral(_) | Expr::StringLiteral(_) | Expr::Variable(_) => {}
    }
    Ok(())
}

fn statements(body: &[Statement], safe: bool) -> Result<()> {
    for statement in body {
        match statement {
            Statement::Print(args, redirect) | Statement::Printf(args, redirect) => {
                if let Some((_, target)) = redirect {
                    if safe {
                        bail!("output redirection is unsafe in safe mode");
                    }
                    expression(target, safe)?;
                }
                for arg in args {
                    expression(arg, safe)?;
                }
            }
            Statement::Expr(e) => expression(e, safe)?,
            Statement::Delete(_, Some(keys)) => {
                for k in keys {
                    expression(k, safe)?;
                }
            }
            Statement::IfElse(e, a, b) => {
                expression(e, safe)?;
                statements(a, safe)?;
                if let Some(b) = b {
                    statements(b, safe)?;
                }
            }
            Statement::While(e, b) | Statement::DoWhile(b, e) => {
                expression(e, safe)?;
                statements(b, safe)?;
            }
            Statement::ForIn(_, _, b) => statements(b, safe)?,
            Statement::For(init, cond, step, b) => {
                for stmt in [init, step].into_iter().flatten() {
                    statements(std::slice::from_ref(stmt), safe)?;
                }
                if let Some(e) = cond {
                    expression(e, safe)?;
                }
                statements(b, safe)?;
            }
            Statement::Return(Some(e)) | Statement::Exit(Some(e)) => expression(e, safe)?,
            _ => {}
        }
    }
    Ok(())
}

/// Collect array usage, including parameters forwarded to another function.
pub fn array_parameters(
    program: &Program,
) -> std::collections::HashMap<String, std::collections::HashSet<String>> {
    use std::collections::{HashMap, HashSet};
    let mut inferred: HashMap<String, HashSet<String>> = HashMap::new();
    loop {
        let previous = inferred.clone();
        for function in &program.functions {
            let mut expressions = Vec::new();
            let mut names = Vec::new();
            collect_statements(&function.body, &mut expressions, &mut names);
            for expr in expressions {
                match expr {
                    Expr::ArrayAccess(name, _) => names.push(name),
                    Expr::BinaryOp(_, crate::ast::BinaryOperator::In, right) => {
                        if let Expr::Variable(name) = right.as_ref() {
                            names.push(name);
                        }
                    }
                    Expr::FunctionCall(name, args) => {
                        if name == "split" {
                            if let Some(Expr::Variable(name)) = args.get(1) {
                                names.push(name);
                            }
                        } else if let Some(callee) =
                            program.functions.iter().find(|f| f.name == *name)
                        {
                            for (param, arg) in callee.params.iter().zip(args) {
                                if previous.get(name).is_some_and(|set| set.contains(param))
                                    && let Expr::Variable(name) = arg
                                {
                                    names.push(name);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            inferred.insert(
                function.name.clone(),
                names
                    .into_iter()
                    .filter(|n| function.params.contains(n))
                    .cloned()
                    .collect(),
            );
        }
        if inferred == previous {
            return inferred;
        }
    }
}

fn collect_expr<'a>(expr: &'a Expr, out: &mut Vec<&'a Expr>) {
    out.push(expr);
    match expr {
        Expr::Field(e)
        | Expr::UnaryMinus(e)
        | Expr::UnaryPlus(e)
        | Expr::Not(e)
        | Expr::PreInc(e)
        | Expr::PostInc(e)
        | Expr::PreDec(e)
        | Expr::PostDec(e) => collect_expr(e, out),
        Expr::BinaryOp(a, _, b) | Expr::Assign(a, _, b) => {
            collect_expr(a, out);
            collect_expr(b, out);
        }
        Expr::Concat(args) | Expr::FunctionCall(_, args) | Expr::ArrayAccess(_, args) => {
            for e in args {
                collect_expr(e, out);
            }
        }
        Expr::Ternary(a, b, c) => {
            collect_expr(a, out);
            collect_expr(b, out);
            collect_expr(c, out);
        }
        Expr::Getline(_, GetlineSource::File(e) | GetlineSource::Pipe(e)) => collect_expr(e, out),
        _ => {}
    }
}
fn collect_statements<'a>(
    body: &'a [Statement],
    out: &mut Vec<&'a Expr>,
    names: &mut Vec<&'a String>,
) {
    for stmt in body {
        match stmt {
            Statement::Expr(e) | Statement::Return(Some(e)) | Statement::Exit(Some(e)) => {
                collect_expr(e, out)
            }
            Statement::Print(args, redirect) | Statement::Printf(args, redirect) => {
                for e in args {
                    collect_expr(e, out);
                }
                if let Some((_, e)) = redirect {
                    collect_expr(e, out);
                }
            }
            Statement::Delete(name, keys) => {
                names.push(name);
                if let Some(keys) = keys {
                    for e in keys {
                        collect_expr(e, out);
                    }
                }
            }
            Statement::ForIn(_, name, body) => {
                names.push(name);
                collect_statements(body, out, names);
            }
            Statement::IfElse(e, a, b) => {
                collect_expr(e, out);
                collect_statements(a, out, names);
                if let Some(b) = b {
                    collect_statements(b, out, names);
                }
            }
            Statement::While(e, b) | Statement::DoWhile(b, e) => {
                collect_expr(e, out);
                collect_statements(b, out, names);
            }
            Statement::For(a, e, b, body) => {
                for stmt in [a, b].into_iter().flatten() {
                    collect_statements(std::slice::from_ref(stmt), out, names);
                }
                if let Some(e) = e {
                    collect_expr(e, out);
                }
                collect_statements(body, out, names);
            }
            _ => {}
        }
    }
}

fn control_context(body: &[Statement], function: bool, loops: usize, special: bool) -> Result<()> {
    for stmt in body {
        match stmt {
            Statement::Next if function || special => bail!("next is not allowed in this context"),
            Statement::Return(_) if !function => bail!("return outside a function"),
            Statement::Break | Statement::Continue if loops == 0 => {
                bail!("loop control outside a loop")
            }
            Statement::While(_, body)
            | Statement::DoWhile(body, _)
            | Statement::ForIn(_, _, body)
            | Statement::For(_, _, _, body) => control_context(body, function, loops + 1, special)?,
            Statement::IfElse(_, a, b) => {
                control_context(a, function, loops, special)?;
                if let Some(b) = b {
                    control_context(b, function, loops, special)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: A high-fidelity port of the historic AWK language from C to Rust.
 */

use crate::ast::{BinaryOperator, Expr, Pattern, Program, Statement};
use regex::Regex;
use crate::cli::Config;
use crate::parser;
use crate::types::EvalContext;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

pub enum CompiledPattern {
    Regex(Regex),
    Begin,
    End,
}

pub struct CompiledRule {
    pattern: Option<CompiledPattern>,
    action: Vec<Statement>,
}

#[derive(Debug, PartialEq)]
pub enum FlowControl {
    None,
    Break,
    Continue,
    Next,
    Return(crate::types::AwkValue),
    Exit(i32),
}

pub fn run(config: Config) -> anyhow::Result<()> {
    let fs = if config.csv {
        ","
    } else if let Some(ref fs) = config.field_separator {
        fs.as_str()
    } else {
        " "
    };

    let mut context = EvalContext::new(fs);

    context.set_var("ARGC", crate::types::AwkValue::Number(config.input_files.len() as f64 + 1.0));
    context.set_array_var("ARGV", "0", crate::types::AwkValue::String("rawk".to_string()));
    for (i, file) in config.input_files.iter().enumerate() {
        context.set_array_var("ARGV", &format!("{}", i + 1), crate::types::AwkValue::String(file.clone()));
    }
    for (key, val) in std::env::vars() {
        context.set_array_var("ENVIRON", &key, crate::types::AwkValue::String(val));
    }
    context.set_var("OFS", crate::types::AwkValue::String(" ".to_string()));
    context.set_var("ORS", crate::types::AwkValue::String("\n".to_string()));

    let mut program_text = String::new();
    if !config.program_files.is_empty() {
        for pf in &config.program_files {
            let content = std::fs::read_to_string(pf)?;
            program_text.push_str(&content);
            program_text.push('\n');
        }
    } else if let Some(ref p) = config.program {
        program_text.push_str(p);
    }

    let program = parser::parse(&program_text)?;

    let mut compiled_rules = Vec::new();
    for rule in &program.rules {
        let pattern = match &rule.pattern {
            Some(Pattern::Regex(re_str)) => Some(CompiledPattern::Regex(Regex::new(re_str).unwrap())),
            Some(Pattern::Begin) => Some(CompiledPattern::Begin),
            Some(Pattern::End) => Some(CompiledPattern::End),
            None => None,
        };
        compiled_rules.push(CompiledRule {
            pattern,
            action: rule.action.clone(),
        });
    }

    for f in program.functions {
        context.functions.insert(f.name, (f.params, f.body));
    }

    // Execute BEGIN blocks
    let fc = execute_special_blocks(&compiled_rules, &mut context, true);
    if let FlowControl::Exit(code) = fc {
        std::process::exit(code);
    }

    if config.input_files.is_empty() {
        let stdin = io::stdin();
        let reader = stdin.lock();
        if let FlowControl::Exit(code) = process_lines(reader, &mut context, &compiled_rules)? {
            std::process::exit(code);
        }
    } else {
        for filename in &config.input_files {
            context.set_var("FILENAME", crate::types::AwkValue::String(filename.clone()));
            if filename == "-" {
                let stdin = io::stdin();
                let reader = stdin.lock();
                if let FlowControl::Exit(code) = process_lines(reader, &mut context, &compiled_rules)? {
                    std::process::exit(code);
                }
            } else {
                let file = File::open(filename)?;
                let reader = BufReader::new(file);
                if let FlowControl::Exit(code) = process_lines(reader, &mut context, &compiled_rules)? {
                    std::process::exit(code);
                }
            }
            context.fnr = 0;
        }
    }

    // Execute END blocks
    let fc = execute_special_blocks(&compiled_rules, &mut context, false);
    if let FlowControl::Exit(code) = fc {
        std::process::exit(code);
    }

    Ok(())
}

fn execute_special_blocks(rules: &[CompiledRule], context: &mut EvalContext, is_begin: bool) -> FlowControl {
    for rule in rules {
        let is_match = match &rule.pattern {
            Some(CompiledPattern::Begin) if is_begin => true,
            Some(CompiledPattern::End) if !is_begin => true,
            _ => false,
        };
        
        if is_match {
            let fc = execute_action(&rule.action, context);
            if let FlowControl::Exit(_) = fc {
                return fc;
            }
        }
    }
    FlowControl::None
}

fn process_lines<R: BufRead>(mut reader: R, context: &mut EvalContext, rules: &[CompiledRule]) -> anyhow::Result<FlowControl> {
    let mut buffer = Vec::new();

    loop {
        buffer.clear();
        let rs_val = context.get_var("RS").as_string();
        let delim = if rs_val.is_empty() { b'\n' } else { rs_val.as_bytes()[0] };
        
        let bytes_read = reader.read_until(delim, &mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        let line = String::from_utf8_lossy(&buffer);
        let line_str = line.trim_end_matches(&[delim as char, '\r', '\n'][..]);
        
        context.update_record(line_str);

        // Execute rules
        for rule in rules {
            let should_execute = match &rule.pattern {
                Some(CompiledPattern::Regex(re)) => re.is_match(&context.record),
                Some(CompiledPattern::Begin) | Some(CompiledPattern::End) => false,
                None => true,
            };
            
            if should_execute {
                let fc = execute_action(&rule.action, context);
                if fc == FlowControl::Next {
                    break; // break the rule loop, process next line
                }
                if let FlowControl::Exit(_) = fc {
                    return Ok(fc);
                }
            }
        }
    }

    Ok(FlowControl::None)
}

fn eval_expr(expr: &Expr, context: &mut EvalContext) -> crate::types::AwkValue {
    match expr {
        Expr::Field(e) => {
            let idx = eval_expr(e, context).as_number() as usize;
            context.get_field(idx)
        }
        Expr::StringLiteral(s) => crate::types::AwkValue::String(s.clone()),
        Expr::Variable(v) => context.get_var(v),
        Expr::ArrayAccess(arr_name, key_exprs) => {
            let mut keys_str = Vec::new();
            for k in key_exprs {
                keys_str.push(eval_expr(k, context).as_string());
            }
            let subsep = context.get_var("SUBSEP").as_string();
            let key = keys_str.join(&subsep);
            context.get_array_var(arr_name, &key)
        }
        Expr::Getline(var_opt, file_opt) => {
            let mut line = String::new();
            let mut read_success = false;

            if let Some(file_expr) = file_opt {
                let filename = eval_expr(file_expr, context).as_string();
                if !context.in_files.contains_key(&filename) {
                    if let Ok(file) = std::fs::File::open(&filename) {
                        context.in_files.insert(filename.clone(), Box::new(std::io::BufReader::new(file)));
                    }
                }
                
                if let Some(reader) = context.in_files.get_mut(&filename) {
                    if let Ok(n) = reader.read_line(&mut line) {
                        if n > 0 { read_success = true; }
                    }
                }
            } else {
                if let Ok(n) = std::io::stdin().read_line(&mut line) {
                    if n > 0 { read_success = true; }
                }
            }

            if read_success {
                let line_str = line.trim_end_matches(&['\r', '\n'][..]).to_string();
                if let Some(var) = var_opt {
                    context.set_var(var, crate::types::AwkValue::String(line_str));
                } else {
                    context.update_record(&line_str);
                }
                crate::types::AwkValue::Number(1.0)
            } else {
                crate::types::AwkValue::Number(0.0)
            }
        }
        Expr::FunctionCall(name, args) => {
            match name.as_str() {
                "length" => {
                    let s = if args.is_empty() {
                        context.record.clone()
                    } else {
                        eval_expr(&args[0], context).as_string()
                    };
                    crate::types::AwkValue::Number(s.len() as f64)
                }
                "tolower" => {
                    let s = if args.is_empty() { String::new() } else { eval_expr(&args[0], context).as_string() };
                    crate::types::AwkValue::String(s.to_lowercase())
                }
                "toupper" => {
                    let s = if args.is_empty() { String::new() } else { eval_expr(&args[0], context).as_string() };
                    crate::types::AwkValue::String(s.to_uppercase())
                }
                "substr" => {
                    let s = eval_expr(&args[0], context).as_string();
                    let start = eval_expr(&args[1], context).as_number() as usize;
                    let len = if args.len() > 2 {
                        eval_expr(&args[2], context).as_number() as usize
                    } else {
                        s.len()
                    };
                    let start_idx = if start > 0 { start - 1 } else { 0 };
                    let sub = s.chars().skip(start_idx).take(len).collect();
                    crate::types::AwkValue::String(sub)
                }
                "index" => {
                    let s = eval_expr(&args[0], context).as_string();
                    let t = eval_expr(&args[1], context).as_string();
                    let idx = s.find(&t).map(|i| i + 1).unwrap_or(0);
                    crate::types::AwkValue::Number(idx as f64)
                }
                "sin" => crate::types::AwkValue::Number(eval_expr(&args[0], context).as_number().sin()),
                "cos" => crate::types::AwkValue::Number(eval_expr(&args[0], context).as_number().cos()),
                "exp" => crate::types::AwkValue::Number(eval_expr(&args[0], context).as_number().exp()),
                "log" => crate::types::AwkValue::Number(eval_expr(&args[0], context).as_number().ln()),
                "sqrt" => crate::types::AwkValue::Number(eval_expr(&args[0], context).as_number().sqrt()),
                "int" => crate::types::AwkValue::Number(eval_expr(&args[0], context).as_number().trunc()),
                "atan2" => {
                    let y = eval_expr(&args[0], context).as_number();
                    let x = eval_expr(&args[1], context).as_number();
                    crate::types::AwkValue::Number(y.atan2(x))
                }
                "rand" => {
                    use rand::RngExt;
                    let r: f64 = context.rng.random();
                    crate::types::AwkValue::Number(r)
                }
                "srand" => {
                    use rand::SeedableRng;
                    let prev_seed = context.get_var("RAND_SEED").as_number() as u64;
                    let new_seed = if args.is_empty() {
                        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
                    } else {
                        eval_expr(&args[0], context).as_number() as u64
                    };
                    context.rng = rand::rngs::StdRng::seed_from_u64(new_seed);
                    context.set_var("RAND_SEED", crate::types::AwkValue::Number(new_seed as f64));
                    crate::types::AwkValue::Number(prev_seed as f64)
                }
                "systime" => {
                    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                    crate::types::AwkValue::Number(t as f64)
                }
                "strftime" => {
                    let format = if args.is_empty() { "%Y-%m-%d %H:%M:%S".to_string() } else { eval_expr(&args[0], context).as_string() };
                    let timestamp = if args.len() > 1 {
                        eval_expr(&args[1], context).as_number() as i64
                    } else {
                        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64
                    };
                    if let Some(dt) = chrono::DateTime::from_timestamp(timestamp, 0) {
                        crate::types::AwkValue::String(dt.format(&format).to_string())
                    } else {
                        crate::types::AwkValue::String("".to_string())
                    }
                }
                "and" => {
                    let v1 = eval_expr(&args[0], context).as_number() as i64;
                    let v2 = eval_expr(&args[1], context).as_number() as i64;
                    crate::types::AwkValue::Number((v1 & v2) as f64)
                }
                "or" => {
                    let v1 = eval_expr(&args[0], context).as_number() as i64;
                    let v2 = eval_expr(&args[1], context).as_number() as i64;
                    crate::types::AwkValue::Number((v1 | v2) as f64)
                }
                "xor" => {
                    let v1 = eval_expr(&args[0], context).as_number() as i64;
                    let v2 = eval_expr(&args[1], context).as_number() as i64;
                    crate::types::AwkValue::Number((v1 ^ v2) as f64)
                }
                "lshift" => {
                    let v1 = eval_expr(&args[0], context).as_number() as i64;
                    let v2 = eval_expr(&args[1], context).as_number() as i64;
                    crate::types::AwkValue::Number((v1 << v2) as f64)
                }
                "rshift" => {
                    let v1 = eval_expr(&args[0], context).as_number() as i64;
                    let v2 = eval_expr(&args[1], context).as_number() as i64;
                    crate::types::AwkValue::Number((v1 >> v2) as f64)
                }
                "sprintf" => {
                    if args.is_empty() { return crate::types::AwkValue::String("".to_string()); }
                    let format = eval_expr(&args[0], context).as_string();
                    let mut result = String::new();
                    let mut chars = format.chars().peekable();
                    let mut arg_idx = 1;
                    while let Some(c) = chars.next() {
                        if c == '%' {
                            if let Some(mut format_char) = chars.next() {
                                while format_char.is_ascii_digit() || format_char == '.' || format_char == '-' {
                                    format_char = chars.next().unwrap_or(' ');
                                }
                                let val = if arg_idx < args.len() { eval_expr(&args[arg_idx], context) } else { crate::types::AwkValue::String("".to_string()) };
                                arg_idx += 1;
                                result.push_str(&val.as_string());
                            }
                        } else if c == '\\' {
                            if let Some(esc) = chars.next() {
                                match esc {
                                    'n' => result.push('\n'),
                                    't' => result.push('\t'),
                                    '\\' => result.push('\\'),
                                    _ => { result.push('\\'); result.push(esc); }
                                }
                            }
                        } else {
                            result.push(c);
                        }
                    }
                    crate::types::AwkValue::String(result)
                }
                "match" => {
                    let s = eval_expr(&args[0], context).as_string();
                    let re_str = eval_expr(&args[1], context).as_string();
                    if let Ok(re) = Regex::new(&re_str) {
                        if let Some(m) = re.find(&s) {
                            context.set_var("RSTART", crate::types::AwkValue::Number(m.start() as f64 + 1.0));
                            context.set_var("RLENGTH", crate::types::AwkValue::Number(m.len() as f64));
                            crate::types::AwkValue::Number(m.start() as f64 + 1.0)
                        } else {
                            context.set_var("RSTART", crate::types::AwkValue::Number(0.0));
                            context.set_var("RLENGTH", crate::types::AwkValue::Number(-1.0));
                            crate::types::AwkValue::Number(0.0)
                        }
                    } else {
                        crate::types::AwkValue::Number(0.0)
                    }
                }
                "split" => {
                    let s = eval_expr(&args[0], context).as_string();
                    let arr_name = if let Expr::Variable(v) = &args[1] { v.clone() } else { "err".to_string() };
                    let fs = if args.len() > 2 { eval_expr(&args[2], context).as_string() } else { context.fs.clone() };
                    let re = Regex::new(&fs).unwrap_or(Regex::new(" ").unwrap());
                    let parts: Vec<&str> = re.split(&s).filter(|x| !x.is_empty()).collect();
                    let count = parts.len();
                    for (i, p) in parts.iter().enumerate() {
                        let key = format!("{}", i + 1);
                        context.set_array_var(&arr_name, &key, crate::types::AwkValue::String(p.to_string()));
                    }
                    crate::types::AwkValue::Number(count as f64)
                }
                "sub" | "gsub" => {
                    let r = eval_expr(&args[0], context).as_string();
                    let s = eval_expr(&args[1], context).as_string();
                    let is_gsub = name == "gsub";
                    
                    let target_val = if args.len() > 2 { eval_expr(&args[2], context).as_string() } else { context.record.clone() };
                    let re = Regex::new(&r).unwrap_or(Regex::new("").unwrap());
                    
                    let mut changed = false;
                    let new_val = if is_gsub {
                        let res = re.replace_all(&target_val, s.as_str());
                        if res != target_val { changed = true; }
                        res.to_string()
                    } else {
                        let res = re.replace(&target_val, s.as_str());
                        if res != target_val { changed = true; }
                        res.to_string()
                    };

                    if args.len() > 2 {
                        if let Expr::Variable(v) = &args[2] {
                            context.set_var(v, crate::types::AwkValue::String(new_val));
                        }
                    } else {
                        context.update_record(&new_val);
                    }
                    
                    crate::types::AwkValue::Number(if changed { 1.0 } else { 0.0 })
                }
                _ => {
                    // Check if it's a user-defined function
                    if let Some((params, body)) = context.functions.get(name).cloned() {
                        let mut local_scope = std::collections::HashMap::new();
                        for (i, param) in params.iter().enumerate() {
                            let arg_val = if i < args.len() {
                                eval_expr(&args[i], context)
                            } else {
                                crate::types::AwkValue::Uninitialized
                            };
                            local_scope.insert(param.clone(), arg_val);
                        }
                        context.local_scopes.push(local_scope);
                        let fc = execute_action(&body, context);
                        context.local_scopes.pop();
                        if let FlowControl::Return(val) = fc {
                            return val;
                        }
                        crate::types::AwkValue::Uninitialized
                    } else {
                        eprintln!("awk: unknown function {}", name);
                        std::process::exit(1);
                    }
                }
            }
        }
        Expr::Ternary(cond, true_expr, false_expr) => {
            if eval_expr(cond, context).is_truthy() {
                eval_expr(true_expr, context)
            } else {
                eval_expr(false_expr, context)
            }
        }
        Expr::PreInc(e) => {
            let val = eval_expr(e, context);
            let new_val = val.add(&crate::types::AwkValue::Number(1.0));
            if let Expr::Variable(v) = &**e {
                context.set_var(v, new_val.clone());
            } else if let Expr::ArrayAccess(arr, ks) = &**e {
                let mut keys_str = Vec::new();
                for k in ks { keys_str.push(eval_expr(k, context).as_string()); }
                let key = keys_str.join(&context.get_var("SUBSEP").as_string());
                context.set_array_var(arr, &key, new_val.clone());
            }
            new_val
        }
        Expr::PostInc(e) => {
            let val = eval_expr(e, context);
            let new_val = val.add(&crate::types::AwkValue::Number(1.0));
            if let Expr::Variable(v) = &**e {
                context.set_var(v, new_val);
            } else if let Expr::ArrayAccess(arr, ks) = &**e {
                let mut keys_str = Vec::new();
                for k in ks { keys_str.push(eval_expr(k, context).as_string()); }
                let key = keys_str.join(&context.get_var("SUBSEP").as_string());
                context.set_array_var(arr, &key, new_val);
            }
            val
        }
        Expr::PreDec(e) => {
            let val = eval_expr(e, context);
            let new_val = val.sub(&crate::types::AwkValue::Number(1.0));
            if let Expr::Variable(v) = &**e {
                context.set_var(v, new_val.clone());
            } else if let Expr::ArrayAccess(arr, ks) = &**e {
                let mut keys_str = Vec::new();
                for k in ks { keys_str.push(eval_expr(k, context).as_string()); }
                let key = keys_str.join(&context.get_var("SUBSEP").as_string());
                context.set_array_var(arr, &key, new_val.clone());
            }
            new_val
        }
        Expr::PostDec(e) => {
            let val = eval_expr(e, context);
            let new_val = val.sub(&crate::types::AwkValue::Number(1.0));
            if let Expr::Variable(v) = &**e {
                context.set_var(v, new_val);
            } else if let Expr::ArrayAccess(arr, ks) = &**e {
                let mut keys_str = Vec::new();
                for k in ks { keys_str.push(eval_expr(k, context).as_string()); }
                let key = keys_str.join(&context.get_var("SUBSEP").as_string());
                context.set_array_var(arr, &key, new_val);
            }
            val
        }
        Expr::Not(e) => {
            let val = eval_expr(e, context);
            crate::types::AwkValue::Number(if val.is_truthy() { 0.0 } else { 1.0 })
        }
        Expr::BinaryOp(lhs, op, rhs) => {
            let l_val = eval_expr(lhs, context);
            let r_val = eval_expr(rhs, context);
            match op {
                BinaryOperator::Add => l_val.add(&r_val),
                BinaryOperator::Sub => l_val.sub(&r_val),
                BinaryOperator::Mul => l_val.mul(&r_val),
                BinaryOperator::Div => l_val.div(&r_val),
                BinaryOperator::Eq => l_val.is_eq(&r_val),
                BinaryOperator::Neq => crate::types::AwkValue::Number(if l_val.is_eq(&r_val).as_number() == 1.0 { 0.0 } else { 1.0 }),
                BinaryOperator::Lt => l_val.is_lt(&r_val),
                BinaryOperator::Gt => l_val.is_gt(&r_val),
                BinaryOperator::Lte => crate::types::AwkValue::Number(if l_val.is_gt(&r_val).as_number() == 1.0 { 0.0 } else { 1.0 }),
                BinaryOperator::Gte => crate::types::AwkValue::Number(if l_val.is_lt(&r_val).as_number() == 1.0 { 0.0 } else { 1.0 }),
                BinaryOperator::And => crate::types::AwkValue::Number(if l_val.is_truthy() && r_val.is_truthy() { 1.0 } else { 0.0 }),
                BinaryOperator::Or => crate::types::AwkValue::Number(if l_val.is_truthy() || r_val.is_truthy() { 1.0 } else { 0.0 }),
                BinaryOperator::Match => {
                    let re_str = r_val.as_string();
                    if let Ok(re) = Regex::new(&re_str) {
                        crate::types::AwkValue::Number(if re.is_match(&l_val.as_string()) { 1.0 } else { 0.0 })
                    } else {
                        crate::types::AwkValue::Number(0.0)
                    }
                }
                BinaryOperator::NotMatch => {
                    let re_str = r_val.as_string();
                    if let Ok(re) = Regex::new(&re_str) {
                        crate::types::AwkValue::Number(if re.is_match(&l_val.as_string()) { 0.0 } else { 1.0 })
                    } else {
                        crate::types::AwkValue::Number(1.0)
                    }
                }
                BinaryOperator::In => {
                    let key = l_val.as_string();
                    let arr_name = if let Expr::Variable(v) = &**rhs { v.clone() } else { "".to_string() };
                    crate::types::AwkValue::Number(if context.arrays.get(&arr_name).map(|a| a.contains_key(&key)).unwrap_or(false) { 1.0 } else { 0.0 })
                }
                BinaryOperator::Concat => {
                    let s = format!("{}{}", l_val.as_string(), r_val.as_string());
                    crate::types::AwkValue::String(s)
                }
            }
        }
    }
}

fn execute_action(action: &[Statement], context: &mut EvalContext) -> FlowControl {
    for stmt in action {
        match stmt {
            Statement::Break => return FlowControl::Break,
            Statement::Continue => return FlowControl::Continue,
            Statement::Next => return FlowControl::Next,
            Statement::Return(expr_opt) => {
                let val = if let Some(expr) = expr_opt {
                    eval_expr(expr, context)
                } else {
                    crate::types::AwkValue::Uninitialized
                };
                return FlowControl::Return(val);
            }
            Statement::Exit(expr_opt) => {
                let code = if let Some(expr) = expr_opt {
                    eval_expr(expr, context).as_number() as i32
                } else {
                    0
                };
                return FlowControl::Exit(code);
            }
            Statement::While(cond, block) => {
                while eval_expr(cond, context).is_truthy() {
                    let fc = execute_action(block, context);
                    if fc == FlowControl::Break { break; }
                    if fc == FlowControl::Continue { continue; }
                    if fc == FlowControl::Next { return fc; }
                    if let FlowControl::Exit(_) = fc { return fc; }
                }
            }
            Statement::DoWhile(block, cond) => {
                loop {
                    let fc = execute_action(block, context);
                    if fc == FlowControl::Break { break; }
                    if fc == FlowControl::Next { return fc; }
                    if let FlowControl::Exit(_) = fc { return fc; }
                    if !eval_expr(cond, context).is_truthy() { break; }
                }
            }
            Statement::ForIn(key_name, arr_name, block) => {
                let keys: Vec<String> = context.arrays
                    .get(arr_name)
                    .map(|arr| arr.keys().cloned().collect())
                    .unwrap_or_default();
                    
                for key in keys {
                    context.set_var(key_name, crate::types::AwkValue::String(key));
                    let fc = execute_action(block, context);
                    if fc == FlowControl::Break { break; }
                    if fc == FlowControl::Continue { continue; }
                    if fc == FlowControl::Next { return fc; }
                    if let FlowControl::Exit(_) = fc { return fc; }
                }
            }
            Statement::For(init, cond, step, block) => {
                if let Some(i) = init {
                    execute_action(&[i.as_ref().clone()], context);
                }
                loop {
                    if let Some(c) = cond {
                        if !eval_expr(c, context).is_truthy() {
                            break;
                        }
                    }
                    let fc = execute_action(block, context);
                    if fc == FlowControl::Break { break; }
                    if matches!(fc, FlowControl::Return(_)) || fc == FlowControl::Next { return fc; }
                    if let FlowControl::Exit(_) = fc { return fc; }
                    // FlowControl::Continue just continues
                    if let Some(s) = step {
                        execute_action(&[s.as_ref().clone()], context);
                    }
                }
            }
            Statement::IfElse(cond, true_branch, false_branch) => {
                let cond_val = eval_expr(cond, context);
                if cond_val.is_truthy() {
                    let fc = execute_action(true_branch, context);
                    if fc != FlowControl::None { return fc; }
                } else if let Some(fb) = false_branch {
                    let fc = execute_action(fb, context);
                    if fc != FlowControl::None { return fc; }
                }
            }
            Statement::Printf(exprs, redirect) => {
                let formatted = eval_expr(&Expr::FunctionCall("sprintf".to_string(), exprs.clone()), context).as_string();
                handle_output(&formatted, redirect, context);
            }
            Statement::Print(exprs, redirect) => {
                let mut out = Vec::new();
                for e in exprs {
                    out.push(eval_expr(e, context).as_string());
                }
                let ofs = context.get_var("OFS").as_string();
                let ors = context.get_var("ORS").as_string();
                let output = out.join(&ofs) + &ors;
                handle_output(&output, redirect, context);
            }
            Statement::Assign(var_name, expr) => {
                let val = eval_expr(expr, context);
                context.set_var(var_name, val);
            }
            Statement::AssignArray(arr_name, key_exprs, val_expr) => {
                let mut keys_str = Vec::new();
                for k in key_exprs { keys_str.push(eval_expr(k, context).as_string()); }
                let key = keys_str.join(&context.get_var("SUBSEP").as_string());
                let val = eval_expr(val_expr, context);
                context.set_array_var(arr_name, &key, val);
            }
            Statement::Delete(arr_name, keys_opt) => {
                if let Some(keys) = keys_opt {
                    let mut keys_str = Vec::new();
                    for k in keys { keys_str.push(eval_expr(k, context).as_string()); }
                    let key = keys_str.join(&context.get_var("SUBSEP").as_string());
                    if let Some(arr) = context.arrays.get_mut(arr_name) {
                        arr.remove(&key);
                    }
                } else {
                    context.arrays.remove(arr_name);
                }
            }
            Statement::Expr(e) => {
                eval_expr(e, context);
            }
        }
    }
    FlowControl::None
}

fn handle_output(output: &str, redirect: &Option<(String, Expr)>, context: &mut EvalContext) {
    if let Some((op, file_expr)) = redirect {
        let filename = eval_expr(file_expr, context).as_string();
        use std::fs::OpenOptions;
        use std::io::Write;
        let file = context.out_files.entry(filename.clone()).or_insert_with(|| {
            if op == ">>" {
                Box::new(OpenOptions::new().create(true).append(true).open(&filename).unwrap()) as Box<dyn std::io::Write>
            } else if op == "|" {
                use std::process::{Command, Stdio};
                let child = Command::new("sh")
                    .arg("-c")
                    .arg(&filename)
                    .stdin(Stdio::piped())
                    .spawn()
                    .unwrap();
                Box::new(child.stdin.unwrap()) as Box<dyn std::io::Write>
            } else {
                Box::new(OpenOptions::new().create(true).write(true).truncate(true).open(&filename).unwrap()) as Box<dyn std::io::Write>
            }
        });
        write!(file, "{}", output).unwrap();
    } else {
        print!("{}", output);
    }
}

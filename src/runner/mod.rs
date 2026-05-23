/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: A high-fidelity port of the historic AWK language from C to Rust.
 */

use crate::ast::GetlineSource;
use crate::ast::{BinaryOperator, Expr, Pattern, Statement};
use crate::types::AwkValue;

use crate::cli::Config;
use crate::parser;
use crate::types::EvalContext;
use anyhow::Context;
use std::fs::File;
use std::io::{BufRead, BufReader};

mod builtins;
mod fmt;
mod io;

pub enum CompiledPattern {
    Expr(Expr),
    Begin,
    End,
    BeginFile,
    EndFile,
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
    NextFile,
    Return(AwkValue),
    Exit(i32),
}

pub fn run(config: Config) -> anyhow::Result<i32> {
    let fs = if config.csv {
        ","
    } else if let Some(ref fs) = config.field_separator {
        fs.as_str()
    } else {
        " "
    };

    let mut context = EvalContext::new(fs);

    context.set_var(
        "ARGC",
        AwkValue::Number(config.input_files.len() as f64 + 1.0),
    );
    context.set_array_var("ARGV", b"0", AwkValue::from_str_num(b"rawk".to_vec()));
    for (i, file) in config.input_files.iter().enumerate() {
        let key_str = format!("{}", i + 1);
        context.set_array_var(
            "ARGV",
            key_str.as_bytes(),
            AwkValue::from_str_num(file.clone().into_bytes()),
        );
    }
    for (key, val) in std::env::vars() {
        context.set_array_var(
            "ENVIRON",
            key.as_bytes(),
            AwkValue::from_str_num(val.into_bytes()),
        );
    }
    context.set_var("OFS", AwkValue::String(b" ".to_vec()));
    context.set_var("ORS", AwkValue::String(b"\n".to_vec()));
    context.set_var("RS", AwkValue::String(b"\n".to_vec()));

    let mut program_text = String::new();
    if !config.program_files.is_empty() {
        for pf in &config.program_files {
            let content = std::fs::read_to_string(pf)
                .with_context(|| format!("lettura programfile '{pf}'"))?;
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
            Some(Pattern::Expr(e)) => Some(CompiledPattern::Expr(e.clone())),
            Some(Pattern::Begin) => Some(CompiledPattern::Begin),
            Some(Pattern::End) => Some(CompiledPattern::End),
            Some(Pattern::BeginFile) => Some(CompiledPattern::BeginFile),
            Some(Pattern::EndFile) => Some(CompiledPattern::EndFile),
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

    for v in &config.variables {
        if let Some(eq_pos) = v.find('=') {
            let name = v[..eq_pos].to_string();
            let raw_value = &v[eq_pos + 1..];
            let decoded = crate::parser::decode_string_escapes(raw_value);
            context.set_var(&name, AwkValue::from_str_num(decoded.into_bytes()));
        } else {
            eprintln!("rawk: invalid -v assignment '{}': expected name=value", v);
            return Ok(2);
        }
    }

    // Execute BEGIN blocks
    let fc = execute_special_blocks(&compiled_rules, &mut context, SpecialBlock::Begin);
    if let FlowControl::Exit(code) = fc {
        return Ok(code);
    }

    let argc_val = context.get_var("ARGC").as_number() as i64;
    let mut files_to_process: Vec<String> = Vec::new();
    for i in 1..argc_val {
        if let Some(arr) = context.arrays.get("ARGV")
            && let Some(val) = arr.get(i.to_string().as_bytes())
        {
            let filename = val.as_string();
            if !filename.is_empty() {
                // Path file: resta String (design R3).
                files_to_process.push(String::from_utf8_lossy(&filename).into_owned());
            }
        }
    }

    if files_to_process.is_empty() {
        context.set_var("FILENAME", AwkValue::String(b"-".to_vec()));
        if let FlowControl::Exit(code) =
            execute_special_blocks(&compiled_rules, &mut context, SpecialBlock::BeginFile)
        {
            return Ok(code);
        }
        let stdin = std::io::stdin();
        let reader = stdin.lock();
        if let FlowControl::Exit(code) = process_lines(reader, &mut context, &compiled_rules)? {
            return Ok(code);
        }
        if let FlowControl::Exit(code) =
            execute_special_blocks(&compiled_rules, &mut context, SpecialBlock::EndFile)
        {
            return Ok(code);
        }
    } else {
        for filename in files_to_process {
            context.set_var("FILENAME", AwkValue::String(filename.clone().into_bytes()));
            if let FlowControl::Exit(code) =
                execute_special_blocks(&compiled_rules, &mut context, SpecialBlock::BeginFile)
            {
                return Ok(code);
            }
            if filename == "-" {
                let stdin = std::io::stdin();
                let reader = stdin.lock();
                if let FlowControl::Exit(code) =
                    process_lines(reader, &mut context, &compiled_rules)?
                {
                    return Ok(code);
                }
            } else {
                let file = File::open(&filename)?;
                let reader = BufReader::new(file);
                if let FlowControl::Exit(code) =
                    process_lines(reader, &mut context, &compiled_rules)?
                {
                    return Ok(code);
                }
            }
            if let FlowControl::Exit(code) =
                execute_special_blocks(&compiled_rules, &mut context, SpecialBlock::EndFile)
            {
                return Ok(code);
            }
            context.fnr = 0;
        }
    }

    // Execute END blocks
    let fc = execute_special_blocks(&compiled_rules, &mut context, SpecialBlock::End);
    if let FlowControl::Exit(code) = fc {
        return Ok(code);
    }

    // Final cleanup: flush tutto, wait() su pipe children
    io::flush_and_close_all(&mut context);

    Ok(0)
}

#[derive(Debug, PartialEq, Eq)]
pub enum SpecialBlock {
    Begin,
    End,
    BeginFile,
    EndFile,
}

fn execute_special_blocks(
    rules: &[CompiledRule],
    context: &mut EvalContext,
    block_type: SpecialBlock,
) -> FlowControl {
    for rule in rules {
        let is_match = match &rule.pattern {
            Some(CompiledPattern::Begin) if block_type == SpecialBlock::Begin => true,
            Some(CompiledPattern::End) if block_type == SpecialBlock::End => true,
            Some(CompiledPattern::BeginFile) if block_type == SpecialBlock::BeginFile => true,
            Some(CompiledPattern::EndFile) if block_type == SpecialBlock::EndFile => true,
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

fn run_rules_on_record(rules: &[CompiledRule], context: &mut EvalContext) -> FlowControl {
    for rule in rules {
        let should_execute = match &rule.pattern {
            Some(CompiledPattern::Expr(e)) => eval_expr(e, context).is_truthy(),
            Some(CompiledPattern::Begin)
            | Some(CompiledPattern::End)
            | Some(CompiledPattern::BeginFile)
            | Some(CompiledPattern::EndFile) => false,
            None => true,
        };

        if should_execute {
            let fc = execute_action(&rule.action, context);
            if fc == FlowControl::Next {
                break; // break the rule loop, process next line
            }
            if fc == FlowControl::NextFile {
                return FlowControl::NextFile;
            }
            if matches!(fc, FlowControl::Exit(_)) {
                return fc;
            }
            if context.nextfile_pending {
                return FlowControl::NextFile;
            }
            if let Some(code) = context.exit_pending {
                return FlowControl::Exit(code);
            }
        }
    }
    FlowControl::None
}

fn process_single_byte<R: BufRead>(
    mut reader: R,
    delim: u8,
    context: &mut EvalContext,
    rules: &[CompiledRule],
) -> anyhow::Result<FlowControl> {
    let mut buffer = Vec::new();

    loop {
        buffer.clear();
        let bytes_read = reader.read_until(delim, &mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        // Strip trailing record terminator (delim, plus optional CR for "\r\n") on raw bytes.
        let mut end = buffer.len();
        let mut rt_bytes: Vec<u8> = Vec::new();
        if end > 0 && buffer[end - 1] == delim {
            end -= 1;
            rt_bytes.push(delim);
            if delim == b'\n' && end > 0 && buffer[end - 1] == b'\r' {
                end -= 1;
                rt_bytes.clear();
                rt_bytes.extend_from_slice(b"\r\n");
            }
        } else if delim == b'\n' && end >= 2 && &buffer[end - 2..end] == b"\r\n" {
            end -= 2;
            rt_bytes.extend_from_slice(b"\r\n");
        } else if delim == b'\n' && end > 0 && buffer[end - 1] == b'\n' {
            end -= 1;
            rt_bytes.push(b'\n');
        }
        let line_bytes = &buffer[..end];

        context.set_var("RT", AwkValue::String(rt_bytes));
        context.update_record(line_bytes);

        let fc = run_rules_on_record(rules, context);
        if matches!(fc, FlowControl::Exit(_)) {
            return Ok(fc);
        }
        if fc == FlowControl::NextFile {
            return Ok(FlowControl::None);
        }
    }

    Ok(FlowControl::None)
}

fn process_paragraph<R: BufRead>(
    mut reader: R,
    context: &mut EvalContext,
    rules: &[CompiledRule],
) -> anyhow::Result<FlowControl> {
    // PHASE7.2→7.4 BRIDGE: paragraph mode legge ancora via read_to_string (str);
    // migrazione byte-aware competenza di 7.4.
    let mut all = String::new();
    reader.read_to_string(&mut all)?;
    let trimmed = all.trim_start_matches('\n');
    let re = regex::Regex::new(r"\n\n+").unwrap();
    let mut last_end = 0;
    for mat in re.find_iter(trimmed) {
        let record = &trimmed[last_end..mat.start()];
        if !record.is_empty() {
            context.set_var("RT", AwkValue::String(mat.as_str().as_bytes().to_vec()));
            context.update_record(record.as_bytes());
            let fc = run_rules_on_record(rules, context);
            if matches!(fc, FlowControl::Exit(_)) {
                return Ok(fc);
            }
            if fc == FlowControl::NextFile {
                return Ok(FlowControl::None);
            }
        }
        last_end = mat.end();
    }
    let last = trimmed[last_end..].trim_end_matches('\n');
    if !last.is_empty() {
        context.set_var("RT", AwkValue::String(Vec::new()));
        context.update_record(last.as_bytes());
        let fc = run_rules_on_record(rules, context);
        if matches!(fc, FlowControl::Exit(_)) {
            return Ok(fc);
        }
        if fc == FlowControl::NextFile {
            return Ok(FlowControl::None);
        }
    }
    Ok(FlowControl::None)
}

fn process_regex_rs<R: BufRead>(
    mut reader: R,
    rs: &str,
    context: &mut EvalContext,
    rules: &[CompiledRule],
) -> anyhow::Result<FlowControl> {
    // PHASE7.2→7.4 BRIDGE: RS-regex mode legge ancora via read_to_string (str);
    // migrazione byte-aware competenza di 7.4.
    let mut all = String::new();
    reader.read_to_string(&mut all)?;
    let re = match regex::Regex::new(rs) {
        Ok(r) => r,
        Err(_) => return Ok(FlowControl::None),
    };
    let mut last_end = 0;
    for mat in re.find_iter(&all) {
        let record = &all[last_end..mat.start()];
        context.set_var("RT", AwkValue::String(mat.as_str().as_bytes().to_vec()));
        context.update_record(record.as_bytes());
        let fc = run_rules_on_record(rules, context);
        if matches!(fc, FlowControl::Exit(_)) {
            return Ok(fc);
        }
        if fc == FlowControl::NextFile {
            return Ok(FlowControl::None);
        }
        last_end = mat.end();
    }
    let last = &all[last_end..];
    if !last.is_empty() {
        context.set_var("RT", AwkValue::String(Vec::new()));
        context.update_record(last.as_bytes());
        let fc = run_rules_on_record(rules, context);
        if matches!(fc, FlowControl::Exit(_)) {
            return Ok(fc);
        }
        if fc == FlowControl::NextFile {
            return Ok(FlowControl::None);
        }
    }
    Ok(FlowControl::None)
}

fn process_lines<R: BufRead>(
    reader: R,
    context: &mut EvalContext,
    rules: &[CompiledRule],
) -> anyhow::Result<FlowControl> {
    let rs_val = context.get_var("RS").as_string();
    let res = if rs_val.is_empty() {
        process_paragraph(reader, context, rules)
    } else if rs_val.len() == 1 {
        process_single_byte(reader, rs_val[0], context, rules)
    } else {
        // PHASE7.2→7.4 BRIDGE: RS-regex compilato su &str finché 7.4 non porta regex::bytes.
        process_regex_rs(reader, &String::from_utf8_lossy(&rs_val), context, rules)
    };
    context.nextfile_pending = false;
    res
}

/// Compone la chiave di un array AWK: valuta i sotto-indici e li unisce con SUBSEP.
/// Phase 7.3: chiave byte-pulita, nessuna conversione lossy.
fn eval_array_key(key_exprs: &[Expr], context: &mut EvalContext) -> Vec<u8> {
    let mut parts: Vec<Vec<u8>> = Vec::new();
    for k in key_exprs {
        parts.push(eval_expr(k, context).as_string());
    }
    let subsep = context.get_var("SUBSEP").as_string();
    parts.join(subsep.as_slice())
}

fn eval_expr(expr: &Expr, context: &mut EvalContext) -> AwkValue {
    match expr {
        Expr::Field(e) => {
            let idx = eval_expr(e, context).as_number() as usize;
            context.get_field(idx)
        }
        Expr::NumberLiteral(n) => AwkValue::Number(*n),
        Expr::StringLiteral(s) => AwkValue::String(s.clone()),
        Expr::Concat(parts) => {
            let convfmt = context.convfmt.clone();
            let mut s: Vec<u8> = Vec::new();
            for e in parts {
                s.extend(eval_expr(e, context).as_string_convfmt(&convfmt));
            }
            AwkValue::String(s)
        }
        Expr::RegexLiteral(re) => {
            // PHASE7.2→7.4 BRIDGE: regex match su &str finché 7.4 non porta regex::bytes.
            let record = String::from_utf8_lossy(&context.get_field(0).as_string()).into_owned();
            let regex = context.compile_or_get_regex(&String::from_utf8_lossy(re));
            AwkValue::Number(if regex.is_match(&record) { 1.0 } else { 0.0 })
        }
        Expr::Variable(v) => context.get_var(v),
        Expr::ArrayAccess(arr_name, key_exprs) => {
            let key = eval_array_key(key_exprs, context);
            context.get_array_var(arr_name, &key)
        }
        Expr::Getline(var_opt, source) => {
            let mut line: Vec<u8> = Vec::new();
            let mut read_success = false;

            match source {
                GetlineSource::Main => {
                    let mut stdin = std::io::stdin().lock();
                    if let Ok(n) = stdin.read_until(b'\n', &mut line)
                        && n > 0
                    {
                        read_success = true;
                    }
                }
                GetlineSource::File(file_expr) => {
                    // Path file: resta String (design R3).
                    let filename =
                        String::from_utf8_lossy(&eval_expr(file_expr, context).as_string())
                            .into_owned();
                    io::ensure_input_file(&filename, context);
                    if let Some(stream) = context.in_files.get_mut(&filename)
                        && let Ok(n) = stream.reader().read_until(b'\n', &mut line)
                        && n > 0
                    {
                        read_success = true;
                    }
                }
                GetlineSource::Pipe(cmd_expr) => {
                    let cmd = String::from_utf8_lossy(&eval_expr(cmd_expr, context).as_string())
                        .into_owned();
                    if !io::ensure_input_pipe(&cmd, context) {
                        return AwkValue::Number(-1.0);
                    }
                    if let Some(stream) = context.in_files.get_mut(&cmd) {
                        match stream.reader().read_until(b'\n', &mut line) {
                            Ok(0) => read_success = false,
                            Ok(_) => read_success = true,
                            Err(_) => return AwkValue::Number(-1.0),
                        }
                    }
                }
            }

            if read_success {
                // Strip trailing \n and optional \r on bytes.
                if line.last() == Some(&b'\n') {
                    line.pop();
                }
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                if let Some(var) = var_opt {
                    context.set_var(var, AwkValue::from_str_num(line));
                } else {
                    context.update_record(&line);
                }
                AwkValue::Number(1.0)
            } else {
                AwkValue::Number(0.0)
            }
        }
        Expr::FunctionCall(name, args) => {
            if let Some(val) = builtins::dispatch_builtin(name, args, context) {
                return val;
            }
            // Not a builtin: user-defined function fallback (semantica invariata).
            if let Some((params, body)) = context.functions.get(name).cloned() {
                let mut local_scope = std::collections::HashMap::new();
                for (i, param) in params.iter().enumerate() {
                    let arg_val = if i < args.len() {
                        eval_expr(&args[i], context)
                    } else {
                        AwkValue::Uninitialized
                    };
                    local_scope.insert(param.clone(), arg_val);
                }
                context.local_scopes.push(local_scope);
                let fc = execute_action(&body, context);
                context.local_scopes.pop();
                if let FlowControl::Return(val) = fc {
                    return val;
                }
                if matches!(fc, FlowControl::Exit(_) | FlowControl::NextFile) {
                    if let FlowControl::Exit(code) = fc {
                        context.exit_pending = Some(code);
                    } else {
                        context.nextfile_pending = true;
                    }
                }
                AwkValue::Uninitialized
            } else {
                eprintln!(
                    "rawk: warning: unknown function '{}' (returning empty)",
                    name
                );
                AwkValue::Uninitialized
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
            let new_val = val.add(&AwkValue::Number(1.0));
            if let Expr::Variable(v) = &**e {
                context.set_var(v, new_val.clone());
            } else if let Expr::ArrayAccess(arr, ks) = &**e {
                let key = eval_array_key(ks, context);
                context.set_array_var(arr, &key, new_val.clone());
            }
            new_val
        }
        Expr::PostInc(e) => {
            let val = eval_expr(e, context);
            let new_val = val.add(&AwkValue::Number(1.0));
            if let Expr::Variable(v) = &**e {
                context.set_var(v, new_val);
            } else if let Expr::ArrayAccess(arr, ks) = &**e {
                let key = eval_array_key(ks, context);
                context.set_array_var(arr, &key, new_val);
            }
            val
        }
        Expr::PreDec(e) => {
            let val = eval_expr(e, context);
            let new_val = val.sub(&AwkValue::Number(1.0));
            if let Expr::Variable(v) = &**e {
                context.set_var(v, new_val.clone());
            } else if let Expr::ArrayAccess(arr, ks) = &**e {
                let key = eval_array_key(ks, context);
                context.set_array_var(arr, &key, new_val.clone());
            }
            new_val
        }
        Expr::PostDec(e) => {
            let val = eval_expr(e, context);
            let new_val = val.sub(&AwkValue::Number(1.0));
            if let Expr::Variable(v) = &**e {
                context.set_var(v, new_val);
            } else if let Expr::ArrayAccess(arr, ks) = &**e {
                let key = eval_array_key(ks, context);
                context.set_array_var(arr, &key, new_val);
            }
            val
        }
        Expr::Not(e) => {
            let val = eval_expr(e, context);
            AwkValue::Number(if val.is_truthy() { 0.0 } else { 1.0 })
        }
        Expr::UnaryMinus(e) => {
            let val = eval_expr(e, context).as_number();
            AwkValue::Number(-val)
        }
        Expr::UnaryPlus(e) => {
            let val = eval_expr(e, context).as_number();
            AwkValue::Number(val)
        }
        Expr::BinaryOp(lhs, op, rhs) => {
            let l_val = eval_expr(lhs, context);
            let r_val = eval_expr(rhs, context);
            match op {
                BinaryOperator::Add => l_val.add(&r_val),
                BinaryOperator::Sub => l_val.sub(&r_val),
                BinaryOperator::Mul => l_val.mul(&r_val),
                BinaryOperator::Div => l_val.div(&r_val),
                BinaryOperator::Mod => l_val.rem(&r_val),
                BinaryOperator::Pow => l_val.pow(&r_val),
                BinaryOperator::Eq => l_val.is_eq(&r_val),
                BinaryOperator::Neq => {
                    AwkValue::Number(if l_val.is_eq(&r_val).as_number() == 1.0 {
                        0.0
                    } else {
                        1.0
                    })
                }
                BinaryOperator::Lt => l_val.is_lt(&r_val),
                BinaryOperator::Gt => l_val.is_gt(&r_val),
                BinaryOperator::Lte => {
                    AwkValue::Number(if l_val.is_gt(&r_val).as_number() == 1.0 {
                        0.0
                    } else {
                        1.0
                    })
                }
                BinaryOperator::Gte => {
                    AwkValue::Number(if l_val.is_lt(&r_val).as_number() == 1.0 {
                        0.0
                    } else {
                        1.0
                    })
                }
                BinaryOperator::And => {
                    AwkValue::Number(if l_val.is_truthy() && r_val.is_truthy() {
                        1.0
                    } else {
                        0.0
                    })
                }
                BinaryOperator::Or => AwkValue::Number(if l_val.is_truthy() || r_val.is_truthy() {
                    1.0
                } else {
                    0.0
                }),
                BinaryOperator::Match => {
                    // PHASE7.2→7.4 BRIDGE: regex match su &str finché 7.4 non porta regex::bytes.
                    let re_str = if let Expr::RegexLiteral(re) = &**rhs {
                        String::from_utf8_lossy(re).into_owned()
                    } else {
                        String::from_utf8_lossy(&r_val.as_string()).into_owned()
                    };
                    let re = context.compile_or_get_regex(&re_str);
                    let subject = String::from_utf8_lossy(&l_val.as_string()).into_owned();
                    AwkValue::Number(if re.is_match(&subject) { 1.0 } else { 0.0 })
                }
                BinaryOperator::NotMatch => {
                    // PHASE7.2→7.4 BRIDGE: regex match su &str finché 7.4 non porta regex::bytes.
                    let re_str = if let Expr::RegexLiteral(re) = &**rhs {
                        String::from_utf8_lossy(re).into_owned()
                    } else {
                        String::from_utf8_lossy(&r_val.as_string()).into_owned()
                    };
                    let re = context.compile_or_get_regex(&re_str);
                    let subject = String::from_utf8_lossy(&l_val.as_string()).into_owned();
                    AwkValue::Number(if re.is_match(&subject) { 0.0 } else { 1.0 })
                }
                BinaryOperator::In => {
                    let key = l_val.as_string();
                    let arr_name = if let Expr::Variable(v) = &**rhs {
                        v.clone()
                    } else {
                        "".to_string()
                    };
                    AwkValue::Number(
                        if context
                            .arrays
                            .get(&arr_name)
                            .map(|a| a.contains_key(key.as_slice()))
                            .unwrap_or(false)
                        {
                            1.0
                        } else {
                            0.0
                        },
                    )
                }
            }
        }
    }
}

fn execute_action(action: &[Statement], context: &mut EvalContext) -> FlowControl {
    for stmt in action {
        if context.nextfile_pending {
            return FlowControl::NextFile;
        }
        if let Some(code) = context.exit_pending {
            return FlowControl::Exit(code);
        }

        match stmt {
            Statement::Break => return FlowControl::Break,
            Statement::Continue => return FlowControl::Continue,
            Statement::Next => return FlowControl::Next,
            Statement::NextFile => return FlowControl::NextFile,
            Statement::Return(expr_opt) => {
                let val = if let Some(expr) = expr_opt {
                    eval_expr(expr, context)
                } else {
                    AwkValue::Uninitialized
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
                    if fc == FlowControl::Break {
                        break;
                    }
                    if fc == FlowControl::Continue {
                        continue;
                    }
                    if fc == FlowControl::Next || fc == FlowControl::NextFile {
                        return fc;
                    }
                    if let FlowControl::Exit(_) = fc {
                        return fc;
                    }
                }
            }
            Statement::DoWhile(block, cond) => loop {
                let fc = execute_action(block, context);
                if fc == FlowControl::Break {
                    break;
                }
                if fc == FlowControl::Next || fc == FlowControl::NextFile {
                    return fc;
                }
                if let FlowControl::Exit(_) = fc {
                    return fc;
                }
                if !eval_expr(cond, context).is_truthy() {
                    break;
                }
            },
            Statement::ForIn(key_name, arr_name, block) => {
                let keys: Vec<Vec<u8>> = context
                    .arrays
                    .get(arr_name)
                    .map(|arr| arr.keys().cloned().collect())
                    .unwrap_or_default();

                for key in keys {
                    context.set_var(key_name, AwkValue::String(key));
                    let fc = execute_action(block, context);
                    if fc == FlowControl::Break {
                        break;
                    }
                    if fc == FlowControl::Continue {
                        continue;
                    }
                    if fc == FlowControl::Next || fc == FlowControl::NextFile {
                        return fc;
                    }
                    if let FlowControl::Exit(_) = fc {
                        return fc;
                    }
                }
            }
            Statement::For(init, cond, step, block) => {
                if let Some(i) = init {
                    execute_action(&[i.as_ref().clone()], context);
                }
                loop {
                    if let Some(c) = cond
                        && !eval_expr(c, context).is_truthy()
                    {
                        break;
                    }
                    let fc = execute_action(block, context);
                    if fc == FlowControl::Break {
                        break;
                    }
                    if matches!(fc, FlowControl::Return(_))
                        || fc == FlowControl::Next
                        || fc == FlowControl::NextFile
                    {
                        return fc;
                    }
                    if let FlowControl::Exit(_) = fc {
                        return fc;
                    }
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
                    if fc != FlowControl::None {
                        return fc;
                    }
                } else if let Some(fb) = false_branch {
                    let fc = execute_action(fb, context);
                    if fc != FlowControl::None {
                        return fc;
                    }
                }
            }
            Statement::Printf(exprs, redirect) => {
                if !exprs.is_empty() {
                    let format_str = eval_expr(&exprs[0], context).as_string();
                    let args: Vec<AwkValue> =
                        exprs[1..].iter().map(|e| eval_expr(e, context)).collect();
                    // PHASE7.2→7.5 BRIDGE: awk_sprintf opera ancora su &str.
                    let formatted = fmt::awk_sprintf(&String::from_utf8_lossy(&format_str), &args);
                    if let Err(e) = io::handle_output(&formatted, redirect, context) {
                        eprintln!("rawk: {e:#}");
                        return FlowControl::Exit(2);
                    }
                }
            }
            Statement::Print(exprs, redirect) => {
                let mut out: Vec<Vec<u8>> = Vec::new();
                for e in exprs {
                    out.push(eval_expr(e, context).as_string_convfmt(&context.ofmt));
                }
                let ofs = context.get_var("OFS").as_string();
                let ors = context.get_var("ORS").as_string();
                let mut output = out.join(ofs.as_slice());
                output.extend_from_slice(&ors);
                // PHASE7.2→7.6 BRIDGE: output byte-esatto è competenza di 7.6.
                if let Err(e) =
                    io::handle_output(&String::from_utf8_lossy(&output), redirect, context)
                {
                    eprintln!("rawk: {e:#}");
                    return FlowControl::Exit(2);
                }
            }
            Statement::Assign(var_name, expr) => {
                let val = eval_expr(expr, context);
                context.set_var(var_name, val);
            }
            Statement::AssignArray(arr_name, key_exprs, val_expr) => {
                let key = eval_array_key(key_exprs, context);
                let val = eval_expr(val_expr, context);
                context.set_array_var(arr_name, &key, val);
            }
            Statement::AssignField(field_expr, val_expr) => {
                let f_idx = eval_expr(field_expr, context).as_number() as usize;
                let val = eval_expr(val_expr, context);
                context.set_field(f_idx, val);
            }
            Statement::Delete(arr_name, keys_opt) => {
                if let Some(keys) = keys_opt {
                    let key = eval_array_key(keys, context);
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

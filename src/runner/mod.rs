/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: A high-fidelity port of the historic AWK language from C to Rust.
 */

use crate::ast::GetlineSource;
use crate::ast::{BinaryOperator, Expr, Pattern, Statement};
use crate::types::{AwkValue, InputStream, OutputStream};

use crate::cli::Config;
use crate::parser;
use crate::types::EvalContext;
use anyhow::Context;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

mod builtins;

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
    context.set_array_var("ARGV", "0", AwkValue::from_str_num("rawk".to_string()));
    for (i, file) in config.input_files.iter().enumerate() {
        context.set_array_var(
            "ARGV",
            &format!("{}", i + 1),
            AwkValue::from_str_num(file.clone()),
        );
    }
    for (key, val) in std::env::vars() {
        context.set_array_var("ENVIRON", &key, AwkValue::from_str_num(val));
    }
    context.set_var("OFS", AwkValue::String(" ".to_string()));
    context.set_var("ORS", AwkValue::String("\n".to_string()));
    context.set_var("RS", AwkValue::String("\n".to_string()));

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
            context.set_var(&name, AwkValue::from_str_num(decoded));
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
    let mut files_to_process = Vec::new();
    for i in 1..argc_val {
        if let Some(arr) = context.arrays.get("ARGV")
            && let Some(val) = arr.get(&i.to_string())
        {
            let filename = val.as_string();
            if !filename.is_empty() {
                files_to_process.push(filename);
            }
        }
    }

    if files_to_process.is_empty() {
        context.set_var("FILENAME", AwkValue::String("-".to_string()));
        if let FlowControl::Exit(code) =
            execute_special_blocks(&compiled_rules, &mut context, SpecialBlock::BeginFile)
        {
            return Ok(code);
        }
        let stdin = io::stdin();
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
            context.set_var("FILENAME", AwkValue::String(filename.clone()));
            if let FlowControl::Exit(code) =
                execute_special_blocks(&compiled_rules, &mut context, SpecialBlock::BeginFile)
            {
                return Ok(code);
            }
            if filename == "-" {
                let stdin = io::stdin();
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
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let streams: Vec<OutputStream> = context.out_files.drain().map(|(_, v)| v).collect();
    for stream in streams {
        match stream {
            OutputStream::File(_) => {}
            OutputStream::Pipe { stdin, mut child } => {
                drop(stdin);
                let _ = child.wait();
            }
        }
    }

    let in_streams: Vec<InputStream> = context.in_files.drain().map(|(_, v)| v).collect();
    for stream in in_streams {
        if let InputStream::Pipe { stdout, mut child } = stream {
            drop(stdout);
            let _ = child.wait();
        }
    }

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

        let line = String::from_utf8_lossy(&buffer);
        let mut line_str = line.as_ref();
        let mut rt_str = String::new();

        if line_str.ends_with(delim as char) {
            line_str = &line_str[..line_str.len() - 1];
            rt_str = String::from_utf8_lossy(&[delim]).to_string();
            if delim == b'\n' && line_str.ends_with('\r') {
                line_str = &line_str[..line_str.len() - 1];
                rt_str = "\r\n".to_string();
            }
        } else if delim == b'\n' && line_str.ends_with("\r\n") {
            line_str = &line_str[..line_str.len() - 2];
            rt_str = "\r\n".to_string();
        } else if delim == b'\n' && line_str.ends_with('\n') {
            line_str = &line_str[..line_str.len() - 1];
            rt_str = "\n".to_string();
        }

        context.set_var("RT", AwkValue::String(rt_str));
        context.update_record(line_str);

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
    let mut all = String::new();
    reader.read_to_string(&mut all)?;
    let trimmed = all.trim_start_matches('\n');
    let re = regex::Regex::new(r"\n\n+").unwrap();
    let mut last_end = 0;
    for mat in re.find_iter(trimmed) {
        let record = &trimmed[last_end..mat.start()];
        if !record.is_empty() {
            context.set_var("RT", AwkValue::String(mat.as_str().to_string()));
            context.update_record(record);
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
        context.set_var("RT", AwkValue::String(String::new()));
        context.update_record(last);
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
    let mut all = String::new();
    reader.read_to_string(&mut all)?;
    let re = match regex::Regex::new(rs) {
        Ok(r) => r,
        Err(_) => return Ok(FlowControl::None),
    };
    let mut last_end = 0;
    for mat in re.find_iter(&all) {
        let record = &all[last_end..mat.start()];
        context.set_var("RT", AwkValue::String(mat.as_str().to_string()));
        context.update_record(record);
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
        context.set_var("RT", AwkValue::String(String::new()));
        context.update_record(last);
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
    } else if rs_val.chars().count() == 1 {
        process_single_byte(reader, rs_val.as_bytes()[0], context, rules)
    } else {
        process_regex_rs(reader, &rs_val, context, rules)
    };
    context.nextfile_pending = false;
    res
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
            let s: String = parts
                .iter()
                .map(|e| eval_expr(e, context).as_string_convfmt(&convfmt))
                .collect();
            AwkValue::String(s)
        }
        Expr::RegexLiteral(re) => {
            let record = context.get_field(0).as_string();
            let regex = context.compile_or_get_regex(re);
            AwkValue::Number(if regex.is_match(&record) { 1.0 } else { 0.0 })
        }
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
        Expr::Getline(var_opt, source) => {
            let mut line = String::new();
            let mut read_success = false;

            match source {
                GetlineSource::Main => {
                    if let Ok(n) = std::io::stdin().read_line(&mut line)
                        && n > 0
                    {
                        read_success = true;
                    }
                }
                GetlineSource::File(file_expr) => {
                    let filename = eval_expr(file_expr, context).as_string();
                    if !context.in_files.contains_key(&filename)
                        && let Ok(file) = std::fs::File::open(&filename)
                    {
                        context.in_files.insert(
                            filename.clone(),
                            InputStream::File(Box::new(std::io::BufReader::new(file))),
                        );
                    }
                    if let Some(stream) = context.in_files.get_mut(&filename)
                        && let Ok(n) = stream.reader().read_line(&mut line)
                        && n > 0
                    {
                        read_success = true;
                    }
                }
                GetlineSource::Pipe(cmd_expr) => {
                    let cmd = eval_expr(cmd_expr, context).as_string();
                    if !context.in_files.contains_key(&cmd) {
                        use std::process::{Command, Stdio};
                        let child_res = Command::new("sh")
                            .arg("-c")
                            .arg(&cmd)
                            .stdout(Stdio::piped())
                            .spawn();
                        if let Ok(mut child) = child_res {
                            let stdout = child.stdout.take().unwrap();
                            let reader = std::io::BufReader::new(stdout);
                            context.in_files.insert(
                                cmd.clone(),
                                InputStream::Pipe {
                                    stdout: Box::new(reader),
                                    child,
                                },
                            );
                        } else {
                            return AwkValue::Number(-1.0);
                        }
                    }
                    if let Some(stream) = context.in_files.get_mut(&cmd) {
                        match stream.reader().read_line(&mut line) {
                            Ok(0) => read_success = false,
                            Ok(_) => read_success = true,
                            Err(_) => return AwkValue::Number(-1.0),
                        }
                    }
                }
            }

            if read_success {
                let line_str = line.trim_end_matches(&['\r', '\n'][..]).to_string();
                if let Some(var) = var_opt {
                    context.set_var(var, AwkValue::from_str_num(line_str));
                } else {
                    context.update_record(&line_str);
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
                let mut keys_str = Vec::new();
                for k in ks {
                    keys_str.push(eval_expr(k, context).as_string());
                }
                let key = keys_str.join(&context.get_var("SUBSEP").as_string());
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
                let mut keys_str = Vec::new();
                for k in ks {
                    keys_str.push(eval_expr(k, context).as_string());
                }
                let key = keys_str.join(&context.get_var("SUBSEP").as_string());
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
                let mut keys_str = Vec::new();
                for k in ks {
                    keys_str.push(eval_expr(k, context).as_string());
                }
                let key = keys_str.join(&context.get_var("SUBSEP").as_string());
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
                let mut keys_str = Vec::new();
                for k in ks {
                    keys_str.push(eval_expr(k, context).as_string());
                }
                let key = keys_str.join(&context.get_var("SUBSEP").as_string());
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
                    let re_str = if let Expr::RegexLiteral(re) = &**rhs {
                        re.clone()
                    } else {
                        r_val.as_string()
                    };
                    let re = context.compile_or_get_regex(&re_str);
                    AwkValue::Number(if re.is_match(&l_val.as_string()) {
                        1.0
                    } else {
                        0.0
                    })
                }
                BinaryOperator::NotMatch => {
                    let re_str = if let Expr::RegexLiteral(re) = &**rhs {
                        re.clone()
                    } else {
                        r_val.as_string()
                    };
                    let re = context.compile_or_get_regex(&re_str);
                    AwkValue::Number(if re.is_match(&l_val.as_string()) {
                        0.0
                    } else {
                        1.0
                    })
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
                            .map(|a| a.contains_key(&key))
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
                let keys: Vec<String> = context
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
                    let fmt = eval_expr(&exprs[0], context).as_string();
                    let args: Vec<AwkValue> =
                        exprs[1..].iter().map(|e| eval_expr(e, context)).collect();
                    let formatted = awk_sprintf(&fmt, &args);
                    if let Err(e) = handle_output(&formatted, redirect, context) {
                        eprintln!("rawk: {e:#}");
                        return FlowControl::Exit(2);
                    }
                }
            }
            Statement::Print(exprs, redirect) => {
                let mut out = Vec::new();
                for e in exprs {
                    out.push(eval_expr(e, context).as_string_convfmt(&context.ofmt));
                }
                let ofs = context.get_var("OFS").as_string();
                let ors = context.get_var("ORS").as_string();
                let output = out.join(&ofs) + &ors;
                if let Err(e) = handle_output(&output, redirect, context) {
                    eprintln!("rawk: {e:#}");
                    return FlowControl::Exit(2);
                }
            }
            Statement::Assign(var_name, expr) => {
                let val = eval_expr(expr, context);
                context.set_var(var_name, val);
            }
            Statement::AssignArray(arr_name, key_exprs, val_expr) => {
                let mut keys = Vec::new();
                for e in key_exprs {
                    keys.push(eval_expr(e, context).as_string());
                }
                let key = keys.join(&context.get_var("SUBSEP").as_string());
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
                    let mut keys_str = Vec::new();
                    for k in keys {
                        keys_str.push(eval_expr(k, context).as_string());
                    }
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

fn handle_output(
    output: &str,
    redirect: &Option<(String, Expr)>,
    context: &mut EvalContext,
) -> anyhow::Result<()> {
    if let Some((op, file_expr)) = redirect {
        let filename = eval_expr(file_expr, context).as_string();
        use std::collections::hash_map::Entry;
        use std::fs::OpenOptions;
        let stream = match context.out_files.entry(filename.clone()) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(v) => {
                let new_stream = if op == ">>" {
                    let f = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&filename)
                        .with_context(|| format!("apertura file output '{filename}' in append"))?;
                    OutputStream::File(Box::new(f))
                } else if op == "|" {
                    use std::process::{Command, Stdio};
                    let mut child = Command::new("sh")
                        .arg("-c")
                        .arg(&filename)
                        .stdin(Stdio::piped())
                        .spawn()
                        .with_context(|| format!("spawn pipe '{filename}'"))?;
                    let stdin = child
                        .stdin
                        .take()
                        .expect("Stdio::piped garantisce stdin disponibile");
                    OutputStream::Pipe {
                        stdin: Box::new(stdin),
                        child,
                    }
                } else {
                    let f = OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(true)
                        .open(&filename)
                        .with_context(|| {
                            format!("apertura file output '{filename}' in scrittura")
                        })?;
                    OutputStream::File(Box::new(f))
                };
                v.insert(new_stream)
            }
        };
        write!(stream.writer(), "{}", output)
            .with_context(|| format!("scrittura su '{filename}'"))?;
    } else {
        print!("{}", output);
    }
    Ok(())
}

fn awk_sprintf(fmt: &str, args: &[AwkValue]) -> String {
    let mut out = String::new();
    let mut chars = fmt.chars().peekable();
    let mut arg_idx = 0;
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        // Caso speciale %% senza arg
        if chars.peek() == Some(&'%') {
            chars.next();
            out.push('%');
            continue;
        }
        // Accumula spec: flags + width + .precision + conversion
        let mut spec = String::from('%');
        loop {
            match chars.next() {
                None => {
                    out.push_str(&spec);
                    return out;
                }
                Some(ch) => {
                    spec.push(ch);
                    if "diouxXeEfgGcs".contains(ch) {
                        let arg = args
                            .get(arg_idx)
                            .cloned()
                            .unwrap_or(AwkValue::Uninitialized);
                        arg_idx += 1;
                        out.push_str(&format_one(&spec, &arg));
                        break;
                    }
                }
            }
        }
    }
    out
}

fn format_one(spec: &str, arg: &AwkValue) -> String {
    let conv = spec.chars().last().unwrap();
    match conv {
        'd' | 'i' => sprintf::sprintf!(spec, arg.as_number() as i64).unwrap_or_default(),
        'o' | 'x' | 'X' | 'u' => {
            sprintf::sprintf!(spec, arg.as_number() as u64).unwrap_or_default()
        }
        'c' => {
            let ch: char = match arg {
                AwkValue::String(s) | AwkValue::StrNum(s, _) if !s.is_empty() => {
                    s.chars().next().unwrap()
                }
                _ => char::from_u32(arg.as_number() as u32).unwrap_or('\0'),
            };
            let one_char = ch.to_string();
            let spec_s = spec.replacen('c', "s", 1);
            sprintf::sprintf!(&spec_s, one_char).unwrap_or_default()
        }
        'e' | 'E' | 'f' | 'g' | 'G' => sprintf::sprintf!(spec, arg.as_number()).unwrap_or_default(),
        's' => sprintf::sprintf!(spec, arg.as_string()).unwrap_or_default(),
        _ => spec.to_string(),
    }
}

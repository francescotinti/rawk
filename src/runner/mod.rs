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

mod builtins;
mod fmt;
mod io;

pub enum CompiledPattern {
    Expr(Expr),
    Range(Expr, Expr),
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
    Error(String),
}

impl From<anyhow::Error> for FlowControl {
    fn from(error: anyhow::Error) -> Self {
        Self::Error(format!("{error:#}"))
    }
}

pub fn run(config: Config) -> anyhow::Result<i32> {
    crate::text::init();
    let fs = if config.csv {
        ","
    } else if let Some(ref fs) = config.field_separator {
        if fs == "t" { "\t" } else { fs.as_str() }
    } else {
        " "
    };

    let mut context = EvalContext::new(fs.as_bytes());
    context.csv = config.csv;

    context.set_var(
        "ARGC",
        AwkValue::Number(config.input_files.len() as f64 + 1.0),
    );
    context.set_array_var(
        "ARGV",
        b"0",
        AwkValue::from_str_num(
            std::env::args()
                .next()
                .unwrap_or_else(|| "rawk".into())
                .into_bytes(),
        ),
    );
    for (i, file) in config.input_files.iter().enumerate() {
        let key_str = format!("{}", i + 1);
        context.set_array_var(
            "ARGV",
            key_str.as_bytes(),
            AwkValue::from_str_num(file.clone().into_bytes()),
        );
    }
    for (key, val) in std::env::vars() {
        if config.safe {
            break;
        }
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
    let mut sources = Vec::new();
    if !config.program_files.is_empty() {
        for pf in &config.program_files {
            let content = if pf == "-" {
                use std::io::Read;
                let mut content = Vec::new();
                std::io::stdin()
                    .read_to_end(&mut content)
                    .context("reading program from stdin")?;
                parser::source_text(&content)
            } else {
                let content =
                    std::fs::read(pf).with_context(|| format!("lettura programfile '{pf}'"))?;
                parser::source_text(&content)
            };
            if !program_text.is_empty() {
                program_text.push('\n');
            }
            sources.push((program_text.len(), pf.as_str()));
            program_text.push_str(&content);
        }
    } else if let Some(ref p) = config.program {
        {
            use std::os::unix::ffi::OsStrExt;
            program_text.push_str(&parser::source_text(p.as_bytes()));
        }
    }

    let program = if sources.is_empty() {
        parser::parse(&program_text)?
    } else {
        parser::parse_with_sources(&program_text, &sources)?
    };
    crate::validation::validate(&program, config.safe)?;

    let mut compiled_rules = Vec::new();
    for rule in &program.rules {
        let pattern = match &rule.pattern {
            Some(Pattern::Expr(e)) => Some(CompiledPattern::Expr(e.clone())),
            Some(Pattern::Range(a, b)) => Some(CompiledPattern::Range(a.clone(), b.clone())),
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

    context.array_params = crate::validation::array_parameters(&program);
    for f in program.functions {
        context.functions.insert(f.name, (f.params, f.body));
    }

    for v in &config.variables {
        if !assignment(&mut context, v) {
            anyhow::bail!("invalid -v assignment '{v}': expected name=value");
        }
    }

    context.rules = std::rc::Rc::new(compiled_rules);
    let rules = context.rules.clone();
    let mut flow = execute_special_blocks(&rules, &mut context, SpecialBlock::Begin);
    if flow == FlowControl::None
        && rules
            .iter()
            .any(|r| !matches!(r.pattern, Some(CompiledPattern::Begin)))
    {
        loop {
            match read_main(&mut context) {
                Ok(Some(record)) => {
                    if let Err(error) = context.update_record(&record) {
                        flow = error;
                        break;
                    }
                }
                Ok(None) => break,
                Err(signal) => {
                    flow = signal;
                    break;
                }
            }
            flow = run_rules_on_record(&rules, &mut context);
            match flow {
                FlowControl::None | FlowControl::Next => {}
                FlowControl::NextFile => {
                    flow = end_file(&mut context);
                    if flow != FlowControl::None {
                        break;
                    }
                }
                _ => break,
            }
        }
    }
    if matches!(
        flow,
        FlowControl::None | FlowControl::Next | FlowControl::NextFile | FlowControl::Exit(_)
    ) {
        if let FlowControl::Exit(code) = flow {
            context.exit_code = code;
        }
        flow = execute_special_blocks(&rules, &mut context, SpecialBlock::End);
    }
    io::flush_and_close_all(&mut context);
    match flow {
        FlowControl::None => Ok(context.exit_code),
        FlowControl::Exit(code) => Ok(code),
        FlowControl::Error(error) => Err(anyhow::anyhow!(error)),
        other => Err(anyhow::anyhow!("invalid control flow: {other:?}")),
    }
}

fn assignment(context: &mut EvalContext, text: &str) -> bool {
    let Some((name, value)) = text.split_once('=') else {
        return false;
    };
    let mut chars = name.chars();
    if !chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return false;
    }
    let value = if value.contains('\n') {
        eprintln!("rawk: newline in assignment string");
        context.exit_code = 2;
        value.replace('\n', "")
    } else {
        value.to_owned()
    };
    context.set_var(
        name,
        AwkValue::from_str_num(parser::decode_string_escapes(&value)),
    );
    true
}

fn end_file(context: &mut EvalContext) -> FlowControl {
    context.input.reader = None;
    let rules = context.rules.clone();
    execute_special_blocks(&rules, context, SpecialBlock::EndFile)
}

fn read_main(context: &mut EvalContext) -> Result<Option<Vec<u8>>, FlowControl> {
    loop {
        if context.input.reader.is_none() {
            let mut filename = None;
            while context.input.next_arg < context.get_var("ARGC").as_number() as usize {
                let arg = context
                    .get_array_var("ARGV", context.input.next_arg.to_string().as_bytes())
                    .as_string();
                context.input.next_arg += 1;
                let arg = String::from_utf8_lossy(&arg);
                if arg.is_empty() || assignment(context, &arg) {
                    continue;
                }
                filename = Some(arg.into_owned());
                break;
            }
            let filename = match filename {
                Some(name) => name,
                None if !context.input.opened => "-".to_owned(),
                None => return Ok(None),
            };
            context.input.reader = Some(if filename == "-" {
                context.stdin.clone()
            } else {
                crate::input::RecordReader::new(
                    File::open(&filename)
                        .map_err(|e| FlowControl::Error(format!("input {filename}: {e}")))?,
                )
                .shared()
            });
            context.input.opened = true;
            context.fnr = 0;
            context.set_var("FILENAME", AwkValue::String(filename.into_bytes()));
            let rules = context.rules.clone();
            match execute_special_blocks(&rules, context, SpecialBlock::BeginFile) {
                FlowControl::None => {}
                FlowControl::NextFile => {
                    let flow = end_file(context);
                    if flow != FlowControl::None {
                        return Err(flow);
                    }
                    continue;
                }
                flow => return Err(flow),
            }
        }
        let rs = context.get_var("RS").as_string();
        let Some(reader) = context.input.reader.as_mut() else {
            continue;
        };
        let record = (if context.csv {
            reader.borrow_mut().next_csv()
        } else {
            reader.borrow_mut().next(&rs)
        })
        .map_err(|e| FlowControl::Error(e.to_string()))?;
        if let Some((record, rt)) = record {
            context.nr += 1;
            context.fnr += 1;
            context.set_var("RT", AwkValue::String(rt));
            return Ok(Some(record));
        }
        let flow = end_file(context);
        if flow != FlowControl::None {
            return Err(flow);
        }
    }
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
            if fc == FlowControl::NextFile
                && matches!(block_type, SpecialBlock::Begin | SpecialBlock::End)
            {
                return FlowControl::Error("nextfile is not allowed in BEGIN/END".into());
            }
            if fc != FlowControl::None {
                return fc;
            }
        }
    }
    FlowControl::None
}

fn run_rules_on_record(rules: &[CompiledRule], context: &mut EvalContext) -> FlowControl {
    for (index, rule) in rules.iter().enumerate() {
        let matches = (|| -> Result<bool, FlowControl> {
            Ok(match &rule.pattern {
                None => true,
                Some(CompiledPattern::Expr(expr)) => eval_expr(expr, context)?.is_truthy(),
                Some(CompiledPattern::Range(start, end)) => {
                    let active =
                        context.ranges.contains(&index) || eval_expr(start, context)?.is_truthy();
                    if active {
                        if eval_expr(end, context)?.is_truthy() {
                            context.ranges.remove(&index);
                        } else {
                            context.ranges.insert(index);
                        }
                    }
                    active
                }
                _ => false,
            })
        })();
        match matches {
            Err(flow) => return flow,
            Ok(false) => {}
            Ok(true) => {
                let flow = execute_action(&rule.action, context);
                if flow != FlowControl::None {
                    return flow;
                }
            }
        }
    }
    FlowControl::None
}

#[derive(Clone)]
enum Target {
    Variable(String),
    Array(String, Vec<u8>),
    Field(usize),
}

fn target(expr: &Expr, context: &mut EvalContext) -> Result<Target, FlowControl> {
    match expr {
        Expr::Variable(name) => Ok(Target::Variable(name.clone())),
        Expr::ArrayAccess(name, keys) => {
            context.ensure_array(name)?;
            Ok(Target::Array(name.clone(), eval_array_key(keys, context)?))
        }
        Expr::Field(index) => {
            let index = eval_expr(index, context)?.as_number();
            if !index.is_finite() || index < 0.0 {
                return Err(FlowControl::Error("invalid field index".into()));
            }
            if index > i32::MAX as f64 {
                return Err(FlowControl::Error("out of range field".into()));
            }
            Ok(Target::Field(index as usize))
        }
        _ => Err(FlowControl::Error("expression is not assignable".into())),
    }
}
impl Target {
    fn get(&self, context: &mut EvalContext) -> AwkValue {
        match self {
            Self::Variable(name) => context.get_var(name),
            Self::Array(name, key) => context.read_array(name, key),
            Self::Field(index) => context.get_field(*index),
        }
    }
    fn set(&self, context: &mut EvalContext, value: AwkValue) -> Result<(), FlowControl> {
        match self {
            Self::Variable(name) => {
                if context.is_function_name(name) {
                    return Err(FlowControl::Error(format!(
                        "cannot assign to function {name}"
                    )));
                }
                if name == "NF" && (!value.as_number().is_finite() || value.as_number() < 0.0) {
                    return Err(FlowControl::Error(
                        "NF must be nonnegative and finite".into(),
                    ));
                }
                if context.array(name).is_some() {
                    return Err(FlowControl::Error(format!("{name} is an array")));
                }
                context.set_var(name, value);
            }
            Self::Array(name, key) => context.set_array_var(name, key, value),
            Self::Field(index) => context.set_field(*index, value)?,
        }
        Ok(())
    }
}

/// Compone la chiave di un array AWK: valuta i sotto-indici e li unisce con SUBSEP.
/// Phase 7.3: chiave byte-pulita, nessuna conversione lossy.
fn eval_array_key(key_exprs: &[Expr], context: &mut EvalContext) -> Result<Vec<u8>, FlowControl> {
    let mut key = Vec::new();
    for (index, expr) in key_exprs.iter().enumerate() {
        if index != 0 {
            key.extend(context.get_var("SUBSEP").as_string());
        }
        key.extend(eval_expr(expr, context)?.as_string_convfmt(&context.convfmt));
    }
    Ok(key)
}

fn eval_expr(expr: &Expr, context: &mut EvalContext) -> Result<AwkValue, FlowControl> {
    Ok(match expr {
        Expr::Field(_) => target(expr, context)?.get(context),
        Expr::NumberLiteral(n) => AwkValue::Number(*n),
        Expr::StringLiteral(s) => AwkValue::String(s.clone()),
        Expr::Tuple(parts) => AwkValue::String(eval_array_key(parts, context)?),
        Expr::Concat(parts) => {
            let convfmt = context.convfmt.clone();
            let mut s: Vec<u8> = Vec::new();
            for e in parts {
                s.extend(eval_expr(e, context)?.as_string_convfmt(&convfmt));
            }
            AwkValue::String(s)
        }
        Expr::RegexLiteral(re) => {
            let record = context.get_field(0).as_string();
            let regex = context.compile_or_get_regex(re)?;
            AwkValue::Number(if regex.is_match(&record) { 1.0 } else { 0.0 })
        }
        Expr::Variable(v) => {
            if context.is_function_name(v) {
                return Err(FlowControl::Error(format!(
                    "cannot read function {v} as a value"
                )));
            }
            if context.array(v).is_some() {
                return Err(FlowControl::Error(format!("{v} is an array")));
            }
            context.get_var(v)
        }
        Expr::ArrayAccess(arr_name, key_exprs) => {
            context.ensure_array(arr_name)?;
            let key = eval_array_key(key_exprs, context)?;
            context.read_array(arr_name, &key)
        }
        Expr::Getline(var_opt, source) => {
            let record = match source {
                GetlineSource::Main => read_main(context)?,
                GetlineSource::File(expr) | GetlineSource::Pipe(expr) => {
                    let target = String::from_utf8_lossy(&eval_expr(expr, context)?.as_string())
                        .into_owned();
                    match source {
                        GetlineSource::File(_) => io::ensure_input_file(&target, context),
                        GetlineSource::Pipe(_) => {
                            io::ensure_input_pipe(&target, context);
                        }
                        _ => unreachable!(),
                    }
                    let rs = context.get_var("RS").as_string();
                    let Some(stream) = context.in_files.get_mut(&target) else {
                        return Ok(AwkValue::Number(-1.0));
                    };
                    match if context.csv {
                        stream.reader().next_csv()
                    } else {
                        stream.reader().next(&rs)
                    } {
                        Ok(Some((record, rt))) => {
                            context.set_var("RT", AwkValue::String(rt));
                            Some(record)
                        }
                        Ok(None) => None,
                        Err(_) => return Ok(AwkValue::Number(-1.0)),
                    }
                }
            };
            if let Some(record) = record {
                if let Some(var) = var_opt {
                    context.set_var(var, AwkValue::from_str_num(record));
                } else {
                    context.update_record(&record)?;
                }
                AwkValue::Number(1.0)
            } else {
                AwkValue::Number(0.0)
            }
        }
        Expr::FunctionCall(name, args) => {
            if let Some(val) = builtins::dispatch_builtin(name, args, context)? {
                return Ok(val);
            }
            // Not a builtin: user-defined function fallback (semantica invariata).
            if let Some((params, body)) = context.functions.get(name).cloned() {
                if args.len() + params.len() > 50 {
                    return Err(FlowControl::Error(format!(
                        "function {name} has too many arguments (limit 50)"
                    )));
                }
                if args.len() > params.len() {
                    eprintln!(
                        "rawk: function {name} called with {} args, uses only {}",
                        args.len(),
                        params.len()
                    );
                }
                let mut local_scope = std::collections::HashMap::new();
                let mut aliases = std::collections::HashMap::new();
                let mut owned = Vec::new();
                for (i, param) in params.iter().enumerate() {
                    let is_array = context
                        .array_params
                        .get(name)
                        .is_some_and(|set| set.contains(param))
                        || matches!(args.get(i),Some(Expr::Variable(v)) if context.array(v).is_some());
                    if is_array {
                        let actual = if let Some(arg) = args.get(i) {
                            let Expr::Variable(v) = arg else {
                                return Err(FlowControl::Error(format!(
                                    "{name}: array argument required"
                                )));
                            };
                            if context.get_var(v) != AwkValue::Uninitialized {
                                return Err(FlowControl::Error(format!("{v} is a scalar")));
                            }
                            context.array_name(v)
                        } else {
                            let name = format!("@{}:{param}", context.local_scopes.len());
                            owned.push(name.clone());
                            name
                        };
                        context.arrays.entry(actual.clone()).or_default();
                        aliases.insert(param.clone(), actual);
                    } else {
                        let value = if let Some(arg) = args.get(i) {
                            eval_expr(arg, context)?
                        } else {
                            AwkValue::Uninitialized
                        };
                        local_scope.insert(param.clone(), value);
                    }
                }
                for arg in args.iter().skip(params.len()) {
                    eval_expr(arg, context)?;
                }
                context.local_scopes.push(local_scope);
                context.array_scopes.push(aliases);
                let fc = execute_action(&body, context);
                context.local_scopes.pop();
                context.array_scopes.pop();
                for name in owned {
                    context.arrays.remove(&name);
                }
                if let FlowControl::Return(val) = fc {
                    return Ok(val);
                }
                if fc != FlowControl::None {
                    return Err(fc);
                }
                AwkValue::Uninitialized
            } else {
                return Err(FlowControl::Error(format!("unknown function '{name}'")));
            }
        }
        Expr::Ternary(cond, true_expr, false_expr) => {
            if eval_expr(cond, context)?.is_truthy() {
                eval_expr(true_expr, context)?
            } else {
                eval_expr(false_expr, context)?
            }
        }
        Expr::Assign(lhs, op, rhs) => {
            let target = target(lhs, context)?;
            let old = target.get(context);
            let value = eval_expr(rhs, context)?;
            let value = match op {
                None => value,
                Some(BinaryOperator::Add) => old.add(&value),
                Some(BinaryOperator::Sub) => old.sub(&value),
                Some(BinaryOperator::Mul) => old.mul(&value),
                Some(BinaryOperator::Div) if value.as_number() == 0.0 => {
                    return Err(FlowControl::Error("division by zero".into()));
                }
                Some(BinaryOperator::Div) => old.div(&value),
                Some(BinaryOperator::Mod) if value.as_number() == 0.0 => {
                    return Err(FlowControl::Error("division by zero in mod".into()));
                }
                Some(BinaryOperator::Mod) => old.rem(&value),
                Some(BinaryOperator::Pow) => old.pow(&value),
                _ => unreachable!(),
            };
            target.set(context, value.clone())?;
            value
        }
        Expr::PreInc(e) | Expr::PostInc(e) | Expr::PreDec(e) | Expr::PostDec(e) => {
            let target = target(e, context)?;
            let old = target.get(context);
            let increment = if matches!(expr, Expr::PreDec(_) | Expr::PostDec(_)) {
                -1.0
            } else {
                1.0
            };
            let value = old.add(&AwkValue::Number(increment));
            target.set(context, value.clone())?;
            if matches!(expr, Expr::PostInc(_) | Expr::PostDec(_)) {
                AwkValue::Number(old.as_number())
            } else {
                value
            }
        }
        Expr::Not(e) => {
            let val = eval_expr(e, context)?;
            AwkValue::Number(if val.is_truthy() { 0.0 } else { 1.0 })
        }
        Expr::UnaryMinus(e) => {
            let val = eval_expr(e, context)?.as_number();
            AwkValue::Number(-val)
        }
        Expr::UnaryPlus(e) => {
            let val = eval_expr(e, context)?.as_number();
            AwkValue::Number(val)
        }
        Expr::BinaryOp(lhs, op, rhs) => {
            let l_val = eval_expr(lhs, context)?;
            if *op == BinaryOperator::And && !l_val.is_truthy() {
                return Ok(AwkValue::Number(0.0));
            }
            if *op == BinaryOperator::Or && l_val.is_truthy() {
                return Ok(AwkValue::Number(1.0));
            }
            // A literal on the right of ~ or !~ is a pattern, not an
            // implicit search of $0. The operator below uses its source.
            let r_val = if *op == BinaryOperator::In
                || (matches!(op, BinaryOperator::Match | BinaryOperator::NotMatch)
                    && matches!(&**rhs, Expr::RegexLiteral(_)))
            {
                AwkValue::Uninitialized
            } else {
                eval_expr(rhs, context)?
            };
            match op {
                BinaryOperator::Add => l_val.add(&r_val),
                BinaryOperator::Sub => l_val.sub(&r_val),
                BinaryOperator::Mul => l_val.mul(&r_val),
                BinaryOperator::Div if r_val.as_number() == 0.0 => {
                    return Err(FlowControl::Error("division by zero".into()));
                }
                BinaryOperator::Div => l_val.div(&r_val),
                BinaryOperator::Mod if r_val.as_number() == 0.0 => {
                    return Err(FlowControl::Error("division by zero in mod".into()));
                }
                BinaryOperator::Mod => l_val.rem(&r_val),
                BinaryOperator::Pow => l_val.pow(&r_val),
                BinaryOperator::Eq => l_val.is_eq(&r_val, &context.convfmt),
                BinaryOperator::Neq => AwkValue::Number(
                    if l_val.is_eq(&r_val, &context.convfmt).as_number() == 1.0 {
                        0.0
                    } else {
                        1.0
                    },
                ),
                BinaryOperator::Lt => l_val.is_lt(&r_val, &context.convfmt),
                BinaryOperator::Gt => l_val.is_gt(&r_val, &context.convfmt),
                BinaryOperator::Lte => AwkValue::Number(
                    if l_val.is_gt(&r_val, &context.convfmt).as_number() == 1.0 {
                        0.0
                    } else {
                        1.0
                    },
                ),
                BinaryOperator::Gte => AwkValue::Number(
                    if l_val.is_lt(&r_val, &context.convfmt).as_number() == 1.0 {
                        0.0
                    } else {
                        1.0
                    },
                ),
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
                    let re_bytes: std::borrow::Cow<[u8]> = if let Expr::RegexLiteral(re) = &**rhs {
                        std::borrow::Cow::Borrowed(re.as_slice())
                    } else {
                        std::borrow::Cow::Owned(r_val.as_string())
                    };
                    let re = context.compile_or_get_regex(&re_bytes)?;
                    let subject = l_val.as_string();
                    AwkValue::Number(if re.is_match(&subject) { 1.0 } else { 0.0 })
                }
                BinaryOperator::NotMatch => {
                    let re_bytes: std::borrow::Cow<[u8]> = if let Expr::RegexLiteral(re) = &**rhs {
                        std::borrow::Cow::Borrowed(re.as_slice())
                    } else {
                        std::borrow::Cow::Owned(r_val.as_string())
                    };
                    let re = context.compile_or_get_regex(&re_bytes)?;
                    let subject = l_val.as_string();
                    AwkValue::Number(if re.is_match(&subject) { 0.0 } else { 1.0 })
                }
                BinaryOperator::In => {
                    let key = l_val.as_string_convfmt(&context.convfmt);
                    let arr_name = if let Expr::Variable(v) = &**rhs {
                        v.clone()
                    } else {
                        "".to_string()
                    };
                    context.ensure_array(&arr_name)?;
                    AwkValue::Number(
                        if context
                            .array(&arr_name)
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
    })
}

fn execute_action(action: &[Statement], context: &mut EvalContext) -> FlowControl {
    execute_action_inner(action, context).unwrap_or_else(|flow| flow)
}

fn execute_action_inner(
    action: &[Statement],
    context: &mut EvalContext,
) -> Result<FlowControl, FlowControl> {
    for stmt in action {
        match stmt {
            Statement::Break => return Ok(FlowControl::Break),
            Statement::Continue => return Ok(FlowControl::Continue),
            Statement::Next => return Ok(FlowControl::Next),
            Statement::NextFile => return Ok(FlowControl::NextFile),
            Statement::Return(expr_opt) => {
                let val = if let Some(expr) = expr_opt {
                    eval_expr(expr, context)?
                } else {
                    AwkValue::Uninitialized
                };
                return Ok(FlowControl::Return(val));
            }
            Statement::Exit(expr_opt) => {
                let code = if let Some(expr) = expr_opt {
                    eval_expr(expr, context)?.as_number() as i32
                } else {
                    context.exit_code
                };
                return Ok(FlowControl::Exit(code));
            }
            Statement::While(cond, block) => {
                while eval_expr(cond, context)?.is_truthy() {
                    let fc = execute_action(block, context);
                    if fc == FlowControl::Break {
                        break;
                    }
                    if fc == FlowControl::Continue {
                        continue;
                    }
                    if fc == FlowControl::Next || fc == FlowControl::NextFile {
                        return Ok(fc);
                    }
                    if matches!(
                        fc,
                        FlowControl::Exit(_) | FlowControl::Return(_) | FlowControl::Error(_)
                    ) {
                        return Ok(fc);
                    }
                }
            }
            Statement::DoWhile(block, cond) => loop {
                let fc = execute_action(block, context);
                if fc == FlowControl::Break {
                    break;
                }
                if fc == FlowControl::Next || fc == FlowControl::NextFile {
                    return Ok(fc);
                }
                if matches!(
                    fc,
                    FlowControl::Exit(_) | FlowControl::Return(_) | FlowControl::Error(_)
                ) {
                    return Ok(fc);
                }
                if !eval_expr(cond, context)?.is_truthy() {
                    break;
                }
            },
            Statement::ForIn(key_name, arr_name, block) => {
                context.ensure_array(arr_name)?;
                let keys: Vec<Vec<u8>> = context
                    .array(arr_name)
                    .map(|arr| arr.keys().cloned().collect())
                    .unwrap_or_default();

                for key in keys {
                    Target::Variable(key_name.clone()).set(context, AwkValue::String(key))?;
                    let fc = execute_action(block, context);
                    if fc == FlowControl::Break {
                        break;
                    }
                    if fc == FlowControl::Continue {
                        continue;
                    }
                    if fc == FlowControl::Next || fc == FlowControl::NextFile {
                        return Ok(fc);
                    }
                    if matches!(
                        fc,
                        FlowControl::Exit(_) | FlowControl::Return(_) | FlowControl::Error(_)
                    ) {
                        return Ok(fc);
                    }
                }
            }
            Statement::For(init, cond, step, block) => {
                if let Some(i) = init {
                    let flow = execute_action(std::slice::from_ref(i), context);
                    if flow != FlowControl::None {
                        return Ok(flow);
                    }
                }
                loop {
                    if let Some(c) = cond
                        && !eval_expr(c, context)?.is_truthy()
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
                        return Ok(fc);
                    }
                    if matches!(
                        fc,
                        FlowControl::Exit(_) | FlowControl::Return(_) | FlowControl::Error(_)
                    ) {
                        return Ok(fc);
                    }
                    // FlowControl::Continue just continues
                    if let Some(s) = step {
                        let flow = execute_action(std::slice::from_ref(s), context);
                        if flow != FlowControl::None {
                            return Ok(flow);
                        }
                    }
                }
            }
            Statement::IfElse(cond, true_branch, false_branch) => {
                let cond_val = eval_expr(cond, context)?;
                if cond_val.is_truthy() {
                    let fc = execute_action(true_branch, context);
                    if fc != FlowControl::None {
                        return Ok(fc);
                    }
                } else if let Some(fb) = false_branch {
                    let fc = execute_action(fb, context);
                    if fc != FlowControl::None {
                        return Ok(fc);
                    }
                }
            }
            Statement::Printf(exprs, redirect) => {
                if !exprs.is_empty() {
                    let format_str = eval_expr(&exprs[0], context)?.as_string();
                    let args: Vec<AwkValue> = exprs[1..]
                        .iter()
                        .map(|e| eval_expr(e, context))
                        .collect::<Result<_, _>>()?;
                    let formatted: Vec<u8> =
                        fmt::awk_sprintf(&format_str, &args, &context.convfmt)?;
                    io::handle_output(&formatted, redirect, context)?;
                }
            }
            Statement::Print(exprs, redirect) => {
                let mut out: Vec<Vec<u8>> = Vec::new();
                let ofmt = context.ofmt.clone();
                for e in exprs {
                    out.push(eval_expr(e, context)?.as_string_convfmt(&ofmt));
                }
                let ofs = context.get_var("OFS").as_string();
                let ors = context.get_var("ORS").as_string();
                let mut output = out.join(ofs.as_slice());
                output.extend_from_slice(&ors);
                io::handle_output(&output, redirect, context)?;
            }
            Statement::Delete(arr_name, keys_opt) => {
                context.ensure_array(arr_name)?;
                if let Some(keys) = keys_opt {
                    let key = eval_array_key(keys, context)?;
                    if let Some(arr) = context.arrays.get_mut(&context.array_name(arr_name)) {
                        arr.remove(&key);
                    }
                } else {
                    context
                        .arrays
                        .entry(context.array_name(arr_name))
                        .or_default()
                        .clear();
                }
            }
            Statement::Expr(e) => {
                eval_expr(e, context)?;
            }
        }
    }
    Ok(FlowControl::None)
}

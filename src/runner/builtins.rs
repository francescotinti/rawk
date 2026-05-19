/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: Dispatch delle funzioni built-in AWK (length, substr, sprintf,
 *              match, split, sub/gsub, close, fflush, system, sin/cos/exp/log/sqrt,
 *              atan2, int, rand/srand, systime/strftime, and/or/xor/lshift/rshift).
 *              Esce da runner/mod.rs nello Step 19 Phase 4b: semantica invariata,
 *              il chiamante prova user-defined function se ritorna None.
 */

use crate::ast::Expr;
use crate::types::{AwkValue, EvalContext, InputStream, OutputStream};

use super::eval_expr;
use super::fmt::awk_sprintf;

pub(super) fn dispatch_builtin(
    name: &str,
    args: &[Expr],
    context: &mut EvalContext,
) -> Option<AwkValue> {
    let value = match name {
        "length" => {
            let s = if args.is_empty() {
                context.record.clone()
            } else {
                eval_expr(&args[0], context).as_string()
            };
            AwkValue::Number(s.len() as f64)
        }
        "tolower" => {
            let s = if args.is_empty() {
                String::new()
            } else {
                eval_expr(&args[0], context).as_string()
            };
            AwkValue::String(s.to_lowercase())
        }
        "toupper" => {
            let s = if args.is_empty() {
                String::new()
            } else {
                eval_expr(&args[0], context).as_string()
            };
            AwkValue::String(s.to_uppercase())
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
            AwkValue::String(sub)
        }
        "index" => {
            let s = eval_expr(&args[0], context).as_string();
            let t = eval_expr(&args[1], context).as_string();
            let idx = s.find(&t).map(|i| i + 1).unwrap_or(0);
            AwkValue::Number(idx as f64)
        }
        "sin" => AwkValue::Number(eval_expr(&args[0], context).as_number().sin()),
        "cos" => AwkValue::Number(eval_expr(&args[0], context).as_number().cos()),
        "exp" => AwkValue::Number(eval_expr(&args[0], context).as_number().exp()),
        "log" => AwkValue::Number(eval_expr(&args[0], context).as_number().ln()),
        "sqrt" => AwkValue::Number(eval_expr(&args[0], context).as_number().sqrt()),
        "int" => AwkValue::Number(eval_expr(&args[0], context).as_number().trunc()),
        "atan2" => {
            let y = eval_expr(&args[0], context).as_number();
            let x = eval_expr(&args[1], context).as_number();
            AwkValue::Number(y.atan2(x))
        }
        "rand" => {
            use rand::RngExt;
            let r: f64 = context.rng.random();
            AwkValue::Number(r)
        }
        "srand" => {
            use rand::SeedableRng;
            let prev_seed = context.get_var("RAND_SEED").as_number() as u64;
            let new_seed = if args.is_empty() {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            } else {
                eval_expr(&args[0], context).as_number() as u64
            };
            context.rng = rand::rngs::StdRng::seed_from_u64(new_seed);
            context.set_var("RAND_SEED", AwkValue::Number(new_seed as f64));
            AwkValue::Number(prev_seed as f64)
        }
        "systime" => {
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            AwkValue::Number(t as f64)
        }
        "strftime" => {
            let format = if args.is_empty() {
                "%Y-%m-%d %H:%M:%S".to_string()
            } else {
                eval_expr(&args[0], context).as_string()
            };
            let timestamp = if args.len() > 1 {
                eval_expr(&args[1], context).as_number() as i64
            } else {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
            };
            if let Some(dt) = chrono::DateTime::from_timestamp(timestamp, 0) {
                AwkValue::String(dt.format(&format).to_string())
            } else {
                AwkValue::String("".to_string())
            }
        }
        "and" => {
            let v1 = eval_expr(&args[0], context).as_number() as i64;
            let v2 = eval_expr(&args[1], context).as_number() as i64;
            AwkValue::Number((v1 & v2) as f64)
        }
        "or" => {
            let v1 = eval_expr(&args[0], context).as_number() as i64;
            let v2 = eval_expr(&args[1], context).as_number() as i64;
            AwkValue::Number((v1 | v2) as f64)
        }
        "xor" => {
            let v1 = eval_expr(&args[0], context).as_number() as i64;
            let v2 = eval_expr(&args[1], context).as_number() as i64;
            AwkValue::Number((v1 ^ v2) as f64)
        }
        "lshift" => {
            let v1 = eval_expr(&args[0], context).as_number() as i64;
            let v2 = eval_expr(&args[1], context).as_number() as i64;
            AwkValue::Number((v1 << v2) as f64)
        }
        "rshift" => {
            let v1 = eval_expr(&args[0], context).as_number() as i64;
            let v2 = eval_expr(&args[1], context).as_number() as i64;
            AwkValue::Number((v1 >> v2) as f64)
        }
        "system" => {
            if args.is_empty() {
                return Some(AwkValue::Number(0.0));
            }
            let cmd = eval_expr(&args[0], context).as_string();
            use std::io::Write;
            let _ = std::io::stdout().flush();
            let status = std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .status();
            let code = match status {
                Ok(s) => s.code().unwrap_or(-1),
                Err(_) => -1,
            };
            AwkValue::Number(code as f64)
        }
        "close" => {
            if args.is_empty() {
                return Some(AwkValue::Number(-1.0));
            }
            let target = eval_expr(&args[0], context).as_string();
            let mut status: i32 = 0;
            let mut found = false;

            if let Some(stream) = context.out_files.remove(&target) {
                found = true;
                match stream {
                    OutputStream::File(_) => {}
                    OutputStream::Pipe { stdin, mut child } => {
                        drop(stdin);
                        if let Ok(s) = child.wait() {
                            status = s.code().unwrap_or(-1);
                        } else {
                            status = -1;
                        }
                    }
                }
            }

            if let Some(stream) = context.in_files.remove(&target) {
                found = true;
                if let InputStream::Pipe { stdout, mut child } = stream {
                    drop(stdout);
                    if let Ok(s) = child.wait() {
                        status = s.code().unwrap_or(-1);
                    } else {
                        status = -1;
                    }
                }
            }

            if found {
                AwkValue::Number(status as f64)
            } else {
                AwkValue::Number(-1.0)
            }
        }
        "fflush" => {
            use std::io::Write;
            let target = if args.is_empty() {
                String::new()
            } else {
                eval_expr(&args[0], context).as_string()
            };

            if target.is_empty() {
                let mut ok = std::io::stdout().flush().is_ok();
                for stream in context.out_files.values_mut() {
                    if stream.writer().flush().is_err() {
                        ok = false;
                    }
                }
                AwkValue::Number(if ok { 0.0 } else { -1.0 })
            } else if target == "stdout" || target == "/dev/stdout" {
                let r = std::io::stdout().flush();
                AwkValue::Number(if r.is_ok() { 0.0 } else { -1.0 })
            } else if let Some(stream) = context.out_files.get_mut(&target) {
                let r = stream.writer().flush();
                AwkValue::Number(if r.is_ok() { 0.0 } else { -1.0 })
            } else {
                AwkValue::Number(-1.0)
            }
        }
        "sprintf" => {
            if args.is_empty() {
                return Some(AwkValue::String(String::new()));
            }
            let fmt = eval_expr(&args[0], context).as_string();
            let vals: Vec<AwkValue> = args[1..].iter().map(|e| eval_expr(e, context)).collect();
            AwkValue::String(awk_sprintf(&fmt, &vals))
        }
        "match" => {
            let s = eval_expr(&args[0], context).as_string();
            let re_str = if let Expr::RegexLiteral(re) = &args[1] {
                re.clone()
            } else {
                eval_expr(&args[1], context).as_string()
            };
            let re = context.compile_or_get_regex(&re_str);
            if let Some(m) = re.find(&s) {
                context.set_var("RSTART", AwkValue::Number(m.start() as f64 + 1.0));
                context.set_var("RLENGTH", AwkValue::Number(m.len() as f64));
                AwkValue::Number(m.start() as f64 + 1.0)
            } else {
                context.set_var("RSTART", AwkValue::Number(0.0));
                context.set_var("RLENGTH", AwkValue::Number(-1.0));
                AwkValue::Number(0.0)
            }
        }
        "split" => {
            let s = eval_expr(&args[0], context).as_string();
            let arr_name = if let Expr::Variable(v) = &args[1] {
                v.clone()
            } else {
                "err".to_string()
            };
            let fs = if args.len() > 2 {
                if let Expr::RegexLiteral(re) = &args[2] {
                    re.clone()
                } else {
                    eval_expr(&args[2], context).as_string()
                }
            } else {
                context.fs.clone()
            };
            let re = context.compile_or_get_regex(&fs);
            let parts: Vec<&str> = re.split(&s).filter(|x| !x.is_empty()).collect();
            let count = parts.len();
            for (i, p) in parts.iter().enumerate() {
                let key = format!("{}", i + 1);
                context.set_array_var(&arr_name, &key, AwkValue::from_str_num(p.to_string()));
            }
            AwkValue::Number(count as f64)
        }
        "sub" | "gsub" => {
            let r = if let Expr::RegexLiteral(re) = &args[0] {
                re.clone()
            } else {
                eval_expr(&args[0], context).as_string()
            };
            let s = eval_expr(&args[1], context).as_string();
            let is_gsub = name == "gsub";

            let target_val = if args.len() > 2 {
                eval_expr(&args[2], context).as_string()
            } else {
                context.record.clone()
            };
            let re = context.compile_or_get_regex(&r);

            let mut changed = false;
            let new_val = if is_gsub {
                let res = re.replace_all(&target_val, s.as_str());
                if res != target_val {
                    changed = true;
                }
                res.to_string()
            } else {
                let res = re.replace(&target_val, s.as_str());
                if res != target_val {
                    changed = true;
                }
                res.to_string()
            };

            if args.len() > 2 {
                match &args[2] {
                    Expr::Variable(v) => context.set_var(v, AwkValue::String(new_val)),
                    Expr::Field(e) => {
                        let f_idx = eval_expr(e, context).as_number() as usize;
                        context.set_field(f_idx, AwkValue::String(new_val));
                    }
                    Expr::ArrayAccess(arr, ks) => {
                        let mut keys = Vec::new();
                        for k in ks {
                            keys.push(eval_expr(k, context).as_string());
                        }
                        let key = keys.join(&context.get_var("SUBSEP").as_string());
                        context.set_array_var(arr, &key, AwkValue::String(new_val));
                    }
                    _ => {}
                }
            } else {
                context.update_record(&new_val);
            }

            AwkValue::Number(if changed { 1.0 } else { 0.0 })
        }
        _ => return None,
    };
    Some(value)
}

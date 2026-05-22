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
            // length() POSIX byte-count: opera direttamente sui byte.
            let n = if args.is_empty() {
                context.record.len()
            } else {
                eval_expr(&args[0], context).as_string().len()
            };
            AwkValue::Number(n as f64)
        }
        "tolower" => {
            let s = if args.is_empty() {
                Vec::new()
            } else {
                eval_expr(&args[0], context).as_string()
            };
            AwkValue::String(s.to_ascii_lowercase())
        }
        "toupper" => {
            let s = if args.is_empty() {
                Vec::new()
            } else {
                eval_expr(&args[0], context).as_string()
            };
            AwkValue::String(s.to_ascii_uppercase())
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
            // substr byte-based: skip/take sui byte (design Phase 7).
            let sub: Vec<u8> = s.iter().skip(start_idx).take(len).copied().collect();
            AwkValue::String(sub)
        }
        "index" => {
            let s = eval_expr(&args[0], context).as_string();
            let t = eval_expr(&args[1], context).as_string();
            // index byte-based: ricerca sub-slice sui byte (design Phase 7).
            let idx = if t.is_empty() {
                1
            } else {
                s.windows(t.len())
                    .position(|w| w == t.as_slice())
                    .map(|i| i + 1)
                    .unwrap_or(0)
            };
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
                String::from_utf8_lossy(&eval_expr(&args[0], context).as_string()).into_owned()
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
                AwkValue::String(dt.format(&format).to_string().into_bytes())
            } else {
                AwkValue::String(Vec::new())
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
            // Comando shell: convertito a String per Command::arg.
            let cmd =
                String::from_utf8_lossy(&eval_expr(&args[0], context).as_string()).into_owned();
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
            // Chiave file/pipe: resta String (path domain, design R3).
            let target =
                String::from_utf8_lossy(&eval_expr(&args[0], context).as_string()).into_owned();
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
                String::from_utf8_lossy(&eval_expr(&args[0], context).as_string()).into_owned()
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
                return Some(AwkValue::String(Vec::new()));
            }
            let fmt = eval_expr(&args[0], context).as_string();
            let vals: Vec<AwkValue> = args[1..].iter().map(|e| eval_expr(e, context)).collect();
            // PHASE7.2→7.5 BRIDGE: awk_sprintf opera ancora su &str/String.
            AwkValue::String(awk_sprintf(&String::from_utf8_lossy(&fmt), &vals).into_bytes())
        }
        "match" => {
            // PHASE7.2→7.4 BRIDGE: subject/regex su &str finché 7.4 non porta regex::bytes.
            let s = String::from_utf8_lossy(&eval_expr(&args[0], context).as_string()).into_owned();
            let re_str = if let Expr::RegexLiteral(re) = &args[1] {
                re.clone()
            } else {
                String::from_utf8_lossy(&eval_expr(&args[1], context).as_string()).into_owned()
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
            // PHASE7.2→7.4 BRIDGE: subject/FS su &str finché 7.4 non porta regex::bytes.
            let s = String::from_utf8_lossy(&eval_expr(&args[0], context).as_string()).into_owned();
            let arr_name = if let Expr::Variable(v) = &args[1] {
                v.clone()
            } else {
                "err".to_string()
            };
            let fs = if args.len() > 2 {
                if let Expr::RegexLiteral(re) = &args[2] {
                    re.clone()
                } else {
                    String::from_utf8_lossy(&eval_expr(&args[2], context).as_string()).into_owned()
                }
            } else {
                context.fs.clone()
            };
            let re = context.compile_or_get_regex(&fs);
            let parts: Vec<&str> = re.split(&s).filter(|x| !x.is_empty()).collect();
            let count = parts.len();
            for (i, p) in parts.iter().enumerate() {
                let key = format!("{}", i + 1);
                context.set_array_var(
                    &arr_name,
                    &key,
                    AwkValue::from_str_num(p.as_bytes().to_vec()),
                );
            }
            AwkValue::Number(count as f64)
        }
        "sub" | "gsub" => {
            // PHASE7.2→7.4 BRIDGE: regex/replace su &str finché 7.4 non porta regex::bytes.
            let r = if let Expr::RegexLiteral(re) = &args[0] {
                re.clone()
            } else {
                String::from_utf8_lossy(&eval_expr(&args[0], context).as_string()).into_owned()
            };
            let s = String::from_utf8_lossy(&eval_expr(&args[1], context).as_string()).into_owned();
            let is_gsub = name == "gsub";

            let target_val = if args.len() > 2 {
                String::from_utf8_lossy(&eval_expr(&args[2], context).as_string()).into_owned()
            } else {
                String::from_utf8_lossy(&context.record).into_owned()
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
                    Expr::Variable(v) => context.set_var(v, AwkValue::String(new_val.into_bytes())),
                    Expr::Field(e) => {
                        let f_idx = eval_expr(e, context).as_number() as usize;
                        context.set_field(f_idx, AwkValue::String(new_val.into_bytes()));
                    }
                    Expr::ArrayAccess(arr, ks) => {
                        let mut keys: Vec<Vec<u8>> = Vec::new();
                        for k in ks {
                            keys.push(eval_expr(k, context).as_string());
                        }
                        let subsep = context.get_var("SUBSEP").as_string();
                        // PHASE7.2→7.3 BRIDGE: array key resta String fino a 7.3.
                        let key =
                            String::from_utf8_lossy(&keys.join(subsep.as_slice())).into_owned();
                        context.set_array_var(arr, &key, AwkValue::String(new_val.into_bytes()));
                    }
                    _ => {}
                }
            } else {
                context.update_record(new_val.as_bytes());
            }

            AwkValue::Number(if changed { 1.0 } else { 0.0 })
        }
        _ => return None,
    };
    Some(value)
}

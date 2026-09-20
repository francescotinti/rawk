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

use super::fmt::awk_sprintf;
use super::{FlowControl, eval_expr};

fn expand_awk_replacement(repl: &[u8], whole: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(repl.len() + whole.len());
    let mut i = 0;
    while i < repl.len() {
        match repl[i] {
            b'\\' if repl.get(i + 1) == Some(&b'\\') => {
                if repl.get(i + 2..i + 4) == Some(b"\\&") {
                    out.extend_from_slice(b"\\&");
                    i += 4;
                } else if repl.get(i + 2) == Some(&b'&') {
                    out.push(b'\\');
                    i += 2;
                } else {
                    out.push(b'\\');
                    if std::env::var_os("POSIXLY_CORRECT").is_none() {
                        out.push(b'\\');
                    }
                    i += 2;
                }
            }
            b'\\' if repl.get(i + 1) == Some(&b'&') => {
                out.push(b'&');
                i += 2;
            }
            b'&' => {
                out.extend_from_slice(whole);
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

pub(super) fn dispatch_builtin(
    name: &str,
    args: &[Expr],
    context: &mut EvalContext,
) -> Result<Option<AwkValue>, FlowControl> {
    let value = match name {
        "length" => {
            // Count character units selected by LC_CTYPE; array length stays cardinality.
            let n = if args.is_empty() {
                crate::text::len(&context.record)
            } else if let Expr::Variable(name) = &args[0]
                && let Some(array) = context.array(name)
            {
                array.len()
            } else {
                crate::text::len(&eval_expr(&args[0], context)?.as_string())
            };
            if args.len() > 1 {
                eprintln!("rawk: warning: function has too many arguments");
                for arg in &args[1..] {
                    eval_expr(arg, context)?;
                }
            }
            AwkValue::Number(n as f64)
        }
        "tolower" => {
            let s = if args.is_empty() {
                Vec::new()
            } else {
                eval_expr(&args[0], context)?.as_string()
            };
            AwkValue::String(crate::text::convert(&s, false).map_err(FlowControl::Error)?)
        }
        "toupper" => {
            let s = if args.is_empty() {
                Vec::new()
            } else {
                eval_expr(&args[0], context)?.as_string()
            };
            AwkValue::String(crate::text::convert(&s, true).map_err(FlowControl::Error)?)
        }
        "substr" => {
            let s = eval_expr(&args[0], context)?.as_string();
            let start = eval_expr(&args[1], context)?.as_number() as usize;
            let len = if args.len() > 2 {
                eval_expr(&args[2], context)?.as_number() as usize
            } else {
                s.len()
            };
            let start_idx = if start > 0 { start - 1 } else { 0 };
            // Translate character positions to byte offsets without lossy conversion.
            let start = crate::text::byte_offset(&s, start_idx);
            let end = start + crate::text::byte_offset(&s[start..], len);
            let sub = s[start..end].to_vec();
            AwkValue::String(sub)
        }
        "index" => {
            let s = eval_expr(&args[0], context)?.as_string();
            let t = eval_expr(&args[1], context)?.as_string();
            // BWK searches byte substrings, then reports the containing character.
            let idx = if t.is_empty() {
                usize::from(!s.is_empty())
            } else {
                s.windows(t.len())
                    .position(|w| w == t.as_slice())
                    .map(|i| crate::text::character_index(&s, i))
                    .unwrap_or(0)
            };
            AwkValue::Number(idx as f64)
        }
        "sin" => AwkValue::Number(eval_expr(&args[0], context)?.as_number().sin()),
        "cos" => AwkValue::Number(eval_expr(&args[0], context)?.as_number().cos()),
        "exp" => AwkValue::Number(eval_expr(&args[0], context)?.as_number().exp()),
        "log" => AwkValue::Number(eval_expr(&args[0], context)?.as_number().ln()),
        "sqrt" => AwkValue::Number(eval_expr(&args[0], context)?.as_number().sqrt()),
        "int" => AwkValue::Number(eval_expr(&args[0], context)?.as_number().trunc()),
        "atan2" => {
            if args.len() == 1 {
                eval_expr(&args[0], context)?;
                eprintln!("rawk: atan2 requires two arguments; returning 1.0");
                return Ok(Some(AwkValue::Number(1.0)));
            }
            let y = eval_expr(&args[0], context)?.as_number();
            let x = eval_expr(&args[1], context)?.as_number();
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
                eval_expr(&args[0], context)?.as_number() as u64
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
                String::from_utf8_lossy(&eval_expr(&args[0], context)?.as_string()).into_owned()
            };
            let timestamp = if args.len() > 1 {
                eval_expr(&args[1], context)?.as_number() as i64
            } else {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
            };
            if let Some(dt) = chrono::DateTime::from_timestamp(timestamp, 0) {
                {
                    use std::fmt::Write;
                    let mut result = String::new();
                    write!(&mut result, "{}", dt.format(&format))
                        .map_err(|_| FlowControl::Error("invalid strftime format".into()))?;
                    AwkValue::String(result.into_bytes())
                }
            } else {
                AwkValue::String(Vec::new())
            }
        }
        "and" => {
            let v1 = eval_expr(&args[0], context)?.as_number() as i64;
            let v2 = eval_expr(&args[1], context)?.as_number() as i64;
            AwkValue::Number((v1 & v2) as f64)
        }
        "or" => {
            let v1 = eval_expr(&args[0], context)?.as_number() as i64;
            let v2 = eval_expr(&args[1], context)?.as_number() as i64;
            AwkValue::Number((v1 | v2) as f64)
        }
        "xor" => {
            let v1 = eval_expr(&args[0], context)?.as_number() as i64;
            let v2 = eval_expr(&args[1], context)?.as_number() as i64;
            AwkValue::Number((v1 ^ v2) as f64)
        }
        "lshift" => {
            let v1 = eval_expr(&args[0], context)?.as_number() as i64;
            let v2 = eval_expr(&args[1], context)?.as_number() as i64;
            AwkValue::Number(
                v1.checked_shl(u32::try_from(v2).unwrap_or(u32::MAX))
                    .ok_or_else(|| FlowControl::Error("invalid shift count".into()))?
                    as f64,
            )
        }
        "rshift" => {
            let v1 = eval_expr(&args[0], context)?.as_number() as i64;
            let v2 = eval_expr(&args[1], context)?.as_number() as i64;
            AwkValue::Number(
                v1.checked_shr(u32::try_from(v2).unwrap_or(u32::MAX))
                    .ok_or_else(|| FlowControl::Error("invalid shift count".into()))?
                    as f64,
            )
        }
        "system" => {
            if args.is_empty() {
                return Ok(Some(AwkValue::Number(0.0)));
            }
            // Comando shell: convertito a String per Command::arg.
            let cmd =
                String::from_utf8_lossy(&eval_expr(&args[0], context)?.as_string()).into_owned();
            use std::io::Write;
            let _ = std::io::stdout().flush();
            let status = std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .status();
            let code = match status {
                Ok(s) => process_status(s),
                Err(_) => -1,
            };
            AwkValue::Number(code as f64)
        }
        "close" => {
            if args.is_empty() {
                return Ok(Some(AwkValue::Number(-1.0)));
            }
            // Chiave file/pipe: resta String (path domain, design R3).
            let target =
                String::from_utf8_lossy(&eval_expr(&args[0], context)?.as_string()).into_owned();
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
                String::from_utf8_lossy(&eval_expr(&args[0], context)?.as_string()).into_owned()
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
                return Ok(Some(AwkValue::String(Vec::new())));
            }
            let fmt = eval_expr(&args[0], context)?.as_string();
            let vals: Vec<AwkValue> = args[1..]
                .iter()
                .map(|e| eval_expr(e, context))
                .collect::<Result<_, _>>()?;
            AwkValue::String(awk_sprintf(&fmt, &vals, &context.convfmt)?)
        }
        "match" => {
            let s = eval_expr(&args[0], context)?.as_string();
            let re_bytes: std::borrow::Cow<[u8]> = if let Expr::RegexLiteral(re) = &args[1] {
                std::borrow::Cow::Borrowed(re.as_slice())
            } else {
                std::borrow::Cow::Owned(eval_expr(&args[1], context)?.as_string())
            };
            let re = context.compile_or_get_regex(&re_bytes)?;
            if let Some(m) = re.find(&s) {
                context.set_var(
                    "RSTART",
                    AwkValue::Number(crate::text::len(&s[..m.start()]) as f64 + 1.0),
                );
                context.set_var(
                    "RLENGTH",
                    AwkValue::Number(crate::text::len(m.as_bytes()) as f64),
                );
                AwkValue::Number(crate::text::len(&s[..m.start()]) as f64 + 1.0)
            } else {
                context.set_var("RSTART", AwkValue::Number(0.0));
                context.set_var("RLENGTH", AwkValue::Number(-1.0));
                AwkValue::Number(0.0)
            }
        }
        "split" => {
            let s = eval_expr(&args[0], context)?.as_string();
            let arr_name = if let Expr::Variable(v) = &args[1] {
                v.clone()
            } else {
                return Err(FlowControl::Error("split requires an array name".into()));
            };
            context.ensure_array(&arr_name)?;
            let fs_bytes: Vec<u8> = if args.len() > 2 {
                if let Expr::RegexLiteral(re) = &args[2] {
                    re.clone()
                } else {
                    eval_expr(&args[2], context)?.as_string()
                }
            } else {
                context.fs.clone()
            };
            let parts = if context.csv && args.len() == 2 {
                crate::input::csv_fields(&s)
            } else if args
                .get(2)
                .is_some_and(|arg| matches!(arg, Expr::RegexLiteral(re) if !re.is_empty()))
            {
                let re = context.compile_or_get_regex(&fs_bytes)?;
                crate::ere::split_using(&s, &re)
            } else {
                crate::ere::split(&s, &fs_bytes).map_err(FlowControl::Error)?
            };
            context
                .arrays
                .entry(context.array_name(&arr_name))
                .or_default()
                .clear();
            let count = parts.len();
            for (i, p) in parts.iter().enumerate() {
                let key = format!("{}", i + 1);
                context.set_array_var(
                    &arr_name,
                    key.as_bytes(),
                    AwkValue::from_str_num(p.to_vec()),
                );
            }
            AwkValue::Number(count as f64)
        }
        "sub" | "gsub" => {
            let r_bytes: Vec<u8> = if let Expr::RegexLiteral(re) = &args[0] {
                re.clone()
            } else {
                eval_expr(&args[0], context)?.as_string()
            };
            let s_bytes = eval_expr(&args[1], context)?.as_string();
            let is_gsub = name == "gsub";
            let destination = if let Some(expr) = args.get(2) {
                super::target(expr, context)?
            } else {
                super::Target::Field(0)
            };
            let target = destination.get(context).as_string();
            let re = context.compile_or_get_regex(&r_bytes)?;
            let subject = re.subject(&target);
            let mut new_val = Vec::new();
            let mut last = 0;
            let mut search = 0;
            let mut count = 0;
            let mut previous_nonempty_end = None;
            while search <= target.len() {
                let Some(m) = subject.find_at(search) else {
                    break;
                };
                // An empty match immediately following a nonempty one is not a second replacement.
                if m.is_empty() && previous_nonempty_end == Some(m.start()) {
                    search = crate::text::advance(&target, m.end());
                    continue;
                }
                new_val.extend_from_slice(&target[last..m.start()]);
                new_val.extend(expand_awk_replacement(&s_bytes, m.as_bytes()));
                count += 1;
                last = m.end();
                previous_nonempty_end = if m.is_empty() { None } else { Some(m.end()) };
                search = if m.is_empty() {
                    crate::text::advance(&target, m.end())
                } else {
                    m.end()
                };
                if !is_gsub {
                    break;
                }
            }
            new_val.extend_from_slice(&target[last..]);
            if count > 0 {
                destination.set(context, AwkValue::String(new_val))?;
            }
            AwkValue::Number(count as f64)
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn process_status(status: std::process::ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 256 + signal + if status.core_dumped() { 256 } else { 0 };
        }
    }
    -1
}

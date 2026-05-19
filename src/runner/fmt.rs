/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: Engine printf/sprintf di rawk. Esce da runner/mod.rs nello
 *              Step 19 Phase 4d. Una sola superficie pub(super) — awk_sprintf —
 *              che processa lo string format AWK e delega a sprintf::sprintf!
 *              per ciascun specifier convertito (diouxXeEfgGcs).
 */

use crate::types::AwkValue;

/// Sostituisce le sequenze `%…` in `fmt` con i valori convertiti di `args`.
/// Comportamento aderente al subset POSIX AWK già coperto dai testcase XML
/// (printf in tutte le sue varianti, %%, troncamento %.Ns, padding, hex,
/// scientifico). Specifier sconosciuto resta letterale.
pub(super) fn awk_sprintf(fmt: &str, args: &[AwkValue]) -> String {
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

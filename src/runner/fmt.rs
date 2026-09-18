/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: Engine printf/sprintf di rawk. Phase 7.5 — byte-aware.
 *              `awk_sprintf(fmt: &[u8], args: &[AwkValue]) -> Vec<u8>`
 *              processa lo string format AWK byte-by-byte. Conversion
 *              numeriche (d/i/o/u/x/X/e/E/f/g/G) delegano a sprintf::sprintf!
 *              su ASCII puro e poi a Vec<u8>. %c emette il primo byte raw
 *              dell'argomento stringa. %s emette i byte raw senza UTF-8
 *              round-trip; width/precision applicati byte-aware.
 */

use crate::types::AwkValue;

/// Sostituisce le sequenze `%…` in `fmt` con i valori convertiti di `args`.
/// Byte-aware: input e output sono `&[u8]` / `Vec<u8>`. Spec format string
/// non valido viene segnalato come errore del programma AWK.
pub(super) fn awk_sprintf(
    fmt: &[u8],
    args: &[AwkValue],
    convfmt: &[u8],
) -> Result<Vec<u8>, super::FlowControl> {
    let mut out: Vec<u8> = Vec::with_capacity(fmt.len());
    let mut i = 0;
    let mut arg_idx = 0;
    while i < fmt.len() {
        let b = fmt[i];
        if b != b'%' {
            out.push(b);
            i += 1;
            continue;
        }
        // Caso speciale %% senza arg
        if i + 1 < fmt.len() && fmt[i + 1] == b'%' {
            out.push(b'%');
            i += 2;
            continue;
        }
        // Accumula spec: flags + width + .precision + conversion
        let spec_start = i;
        i += 1;
        let mut conv: Option<u8> = None;
        while i < fmt.len() {
            let c = fmt[i];
            i += 1;
            if b"diouxXeEfgGaAcs".contains(&c) {
                conv = Some(c);
                break;
            }
            if !b"-+ #0.123456789".contains(&c) {
                return Err(super::FlowControl::Error(
                    "unsupported printf format".into(),
                ));
            }
        }
        let spec_bytes = &fmt[spec_start..i];
        match conv {
            None => {
                // EOF dentro lo spec — emetti letterale e termina.
                return Err(super::FlowControl::Error("incomplete printf format".into()));
            }
            Some(c) => {
                let arg = args
                    .get(arg_idx)
                    .cloned()
                    .ok_or_else(|| super::FlowControl::Error("missing printf argument".into()))?;
                arg_idx += 1;
                format_one(spec_bytes, c, &arg, convfmt, &mut out);
            }
        }
    }
    Ok(out)
}

fn format_one(spec_bytes: &[u8], conv: u8, arg: &AwkValue, convfmt: &[u8], out: &mut Vec<u8>) {
    // Lo spec è ASCII puro per costruzione (% + flags `-+ #0` + digit + `.` +
    // conversion byte). `from_utf8` è O(spec.len) ma piccolo (raramente >10B).
    let spec = std::str::from_utf8(spec_bytes).expect("awk_sprintf: format spec must be ASCII");
    match conv {
        b'd' | b'i' => {
            let s = sprintf::sprintf!(spec, arg.as_number() as i64).unwrap_or_default();
            out.extend_from_slice(s.as_bytes());
        }
        b'o' | b'x' | b'X' | b'u' => {
            let s = sprintf::sprintf!(spec, arg.as_number() as u64).unwrap_or_default();
            out.extend_from_slice(s.as_bytes());
        }
        b'a' | b'A' => out.extend(hex_float(spec_bytes, arg.as_number())),
        b'e' | b'E' | b'f' | b'g' | b'G' => {
            let s = sprintf::sprintf!(spec, arg.as_number()).unwrap_or_default();
            out.extend_from_slice(s.as_bytes());
        }
        b'c' => {
            // Phase 7.5: %c emette il PRIMO BYTE raw dell'argomento stringa.
            // Se non-stringa (Number/Uninitialized), si converte il numero a u32
            // e si emette il byte basso (parità col comportamento legacy).
            let byte: u8 = match arg {
                AwkValue::String(s) | AwkValue::StrNum(s, _) if !s.is_empty() => s[0],
                _ => arg.as_number() as u32 as u8,
            };
            if spec_bytes.len() == 2 {
                // Fast path: spec esattamente `%c` → emetti il byte raw.
                out.push(byte);
            } else if byte < 0x80 {
                // Spec con width/flags su byte ASCII → delega a sprintf!.
                let spec_s: String = spec
                    .chars()
                    .map(|ch| if ch == 'c' { 's' } else { ch })
                    .collect();
                let one = (byte as char).to_string();
                let s = sprintf::sprintf!(&spec_s, one).unwrap_or_default();
                out.extend_from_slice(s.as_bytes());
            } else {
                // Spec con width/flags su byte alto → emetti raw (caso edge,
                // perdita width formatting ma byte preservato).
                out.push(byte);
            }
        }
        b's' => {
            // Phase 7.5: %s emette `arg.as_string_convfmt(convfmt)` integro come bytes raw.
            // Width/precision applicati byte-aware.
            let s_bytes = arg.as_string_convfmt(convfmt);
            if spec_bytes.len() == 2 {
                // Fast path: spec esattamente `%s` → emetti tutto raw.
                out.extend_from_slice(&s_bytes);
            } else {
                let (width, precision, left_align, zero_pad) = parse_s_flags(spec_bytes);
                let mut truncated: &[u8] = &s_bytes;
                if let Some(p) = precision
                    && truncated.len() > p
                {
                    truncated = &truncated[..p];
                }
                let pad_count = width.saturating_sub(truncated.len());
                let pad_byte = if zero_pad { b'0' } else { b' ' };
                if left_align {
                    out.extend_from_slice(truncated);
                    for _ in 0..pad_count {
                        out.push(pad_byte);
                    }
                } else {
                    for _ in 0..pad_count {
                        out.push(pad_byte);
                    }
                    out.extend_from_slice(truncated);
                }
            }
        }
        _ => {
            // Defensivo: conversion sconosciuto (già filtrato dal while sopra).
            out.extend_from_slice(spec_bytes);
        }
    }
}

/// Parser dei flag dello spec `%s`: ritorna (width, precision, left_align, zero_pad).
/// `spec_bytes` include `%` iniziale e il conversion `s` finale.
fn parse_s_flags(spec_bytes: &[u8]) -> (usize, Option<usize>, bool, bool) {
    let mut i = 1; // skip `%`
    let mut left_align = false;
    let mut zero_pad = false;
    while i < spec_bytes.len() {
        match spec_bytes[i] {
            b'-' => {
                left_align = true;
                i += 1;
            }
            b'0' => {
                zero_pad = true;
                i += 1;
            }
            b'+' | b' ' | b'#' => i += 1,
            _ => break,
        }
    }
    let mut width: usize = 0;
    while i < spec_bytes.len() && spec_bytes[i].is_ascii_digit() {
        width = width * 10 + (spec_bytes[i] - b'0') as usize;
        i += 1;
    }
    let mut precision: Option<usize> = None;
    if i < spec_bytes.len() && spec_bytes[i] == b'.' {
        i += 1;
        let mut p: usize = 0;
        while i < spec_bytes.len() && spec_bytes[i].is_ascii_digit() {
            p = p * 10 + (spec_bytes[i] - b'0') as usize;
            i += 1;
        }
        precision = Some(p);
    }
    (width, precision, left_align, zero_pad)
}

// C99 hexadecimal floating point conversion from the exact IEEE-754 bits.
// Precision follows the reference libc: Darwin resolves exact ties toward
// zero; other targets use ties to even.
fn hex_float(spec: &[u8], value: f64) -> Vec<u8> {
    let value = value + 0.0; // BWK setfval normalizes negative zero.
    let (width, precision, left, zero) = parse_s_flags(spec);
    let upper = spec.last() == Some(&b'A');
    let sign = if value.is_sign_negative() {
        "-"
    } else if spec.contains(&b'+') {
        "+"
    } else if spec.contains(&b' ') {
        " "
    } else {
        ""
    };
    let mut body = if !value.is_finite() {
        if value.is_nan() {
            "nan".to_string()
        } else {
            "inf".to_string()
        }
    } else {
        let bits = value.abs().to_bits();
        let raw_exp = ((bits >> 52) & 0x7ff) as i32;
        let mut mantissa = bits & ((1u64 << 52) - 1);
        let exponent = if raw_exp == 0 {
            if mantissa == 0 {
                0
            } else if cfg!(target_os = "macos") {
                let shift = mantissa.leading_zeros() - 11;
                mantissa <<= shift;
                -1022 - shift as i32
            } else {
                -1022
            }
        } else {
            mantissa |= 1 << 52;
            raw_exp - 1023
        };
        let digits = precision.unwrap_or(13);
        if digits < 13 {
            let shift = 52 - 4 * digits;
            let unit = 1u64 << shift;
            let remainder = mantissa & (unit - 1);
            let half = unit / 2;
            // Darwin's %a rounds from the first omitted hexadecimal digit;
            // an omitted 8 is not rounded up, even with further nonzero digits.
            let round = if cfg!(target_os = "macos") {
                (remainder >> (shift - 4)) > 8
            } else {
                remainder > half || (remainder == half && (mantissa / unit) & 1 != 0)
            };
            mantissa = (mantissa / unit + u64::from(round)) * unit;
        }
        let whole = mantissa >> 52;
        let mut fraction = format!("{:013x}", mantissa & ((1u64 << 52) - 1));
        if let Some(n) = precision {
            fraction.truncate(n.min(13));
            fraction.extend(std::iter::repeat_n('0', n.saturating_sub(13)));
        } else {
            while fraction.ends_with('0') {
                fraction.pop();
            }
        }
        let point = if !fraction.is_empty() || spec.contains(&b'#') {
            "."
        } else {
            ""
        };
        format!("0x{whole:x}{point}{fraction}p{exponent:+}")
    };
    if upper {
        body.make_ascii_uppercase();
    }
    let padding = width.saturating_sub(sign.len() + body.len());
    let result = if left {
        format!("{sign}{body}{}", " ".repeat(padding))
    } else if zero && value.is_finite() {
        format!("{sign}{}{}{}", &body[..2], "0".repeat(padding), &body[2..])
    } else {
        format!("{}{sign}{body}", " ".repeat(padding))
    };
    result.into_bytes()
}

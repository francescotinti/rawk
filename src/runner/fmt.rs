/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: Engine printf/sprintf di rawk. Phase 7.5 — byte-aware.
 *              `awk_sprintf(fmt: &[u8], args: &[AwkValue]) -> Vec<u8>`
 *              processa lo string format AWK byte-by-byte. Conversion
 *              intere sono gestite direttamente; quelle decimali usano
 *              number_format e quelle esadecimali hex_float. %c distingue
 *              valori numerici (anche StrNum) da stringhe esplicite. %s emette i byte raw senza UTF-8
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
        let mut spec_bytes = vec![b'%'];
        i += 1;
        let mut conv: Option<u8> = None;
        while i < fmt.len() {
            let c = fmt[i];
            i += 1;
            if b"hjLlqtz".contains(&c) {
                continue;
            }
            if c == b'*' {
                let value = args
                    .get(arg_idx)
                    .ok_or_else(|| {
                        super::FlowControl::Error("missing printf width/precision argument".into())
                    })?
                    .as_number();
                arg_idx += 1;
                if !value.is_finite() || value.abs() > 1_000_000.0 {
                    return Err(super::FlowControl::Error(
                        "printf width/precision exceeds supported range".into(),
                    ));
                }
                let value = value as i32;
                if value < 0
                    && spec_bytes.last() == Some(&b'.')
                    && !cfg!(all(target_os = "linux", target_env = "gnu"))
                {
                    // BWK substitutes the star textually. Darwin interprets
                    // %width.-Ns as left-aligned width N with zero precision.
                    spec_bytes = negative_precision_spec(&spec_bytes, value.unsigned_abs());
                } else {
                    spec_bytes.extend(value.to_string().bytes());
                }
                continue;
            }
            spec_bytes.push(c);
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
        match conv {
            None => {
                let arg = args
                    .get(arg_idx)
                    .ok_or_else(|| super::FlowControl::Error("incomplete printf format".into()))?;
                arg_idx += 1;
                eprintln!(
                    "rawk: weird printf conversion {}",
                    String::from_utf8_lossy(&spec_bytes)
                );
                out.extend_from_slice(&spec_bytes);
                out.extend(arg.as_string_convfmt(convfmt));
            }
            Some(c) => {
                let arg = args
                    .get(arg_idx)
                    .cloned()
                    .ok_or_else(|| super::FlowControl::Error("missing printf argument".into()))?;
                arg_idx += 1;
                format_one(&spec_bytes, c, &arg, convfmt, &mut out);
            }
        }
    }
    Ok(out)
}

fn format_one(spec_bytes: &[u8], conv: u8, arg: &AwkValue, convfmt: &[u8], out: &mut Vec<u8>) {
    if cfg!(all(target_os = "linux", target_env = "gnu"))
        && let Some(literal) = glibc_negative_precision(spec_bytes, conv)
    {
        // BWK bypasses snprintf for multibyte strings/characters and numeric
        // NUL. Keep rawk's existing byte-preserving formatting on those paths.
        let uses_libc = match conv {
            b's' => {
                let bytes = arg.as_string_convfmt(convfmt);
                !(0..bytes.len()).any(|i| crate::text::next_len(&bytes[i..]) > 1)
            }
            b'c' => match arg {
                AwkValue::String(s) => !s.is_empty() && s[0] != 0 && crate::text::next_len(s) <= 1,
                _ => {
                    arg.as_number() as i64 != 0
                        && !(crate::text::multibyte() && arg.as_number() >= 128.0)
                }
            },
            _ => true,
        };
        if uses_libc {
            out.extend(literal);
        } else {
            let dot = spec_bytes.windows(2).position(|p| p == b".-").unwrap();
            let precision = std::str::from_utf8(&spec_bytes[dot + 2..spec_bytes.len() - 1])
                .ok()
                .and_then(|p| p.parse::<u32>().ok())
                .unwrap_or(0);
            let mut normalized = negative_precision_spec(spec_bytes, precision);
            normalized.push(conv);
            format_one(&normalized, conv, arg, convfmt, out);
        }
        return;
    }
    // Lo spec è ASCII puro per costruzione (% + flags `-+ #0` + digit + `.` +
    // conversion byte). `from_utf8` è O(spec.len) ma piccolo (raramente >10B).
    let spec = std::str::from_utf8(spec_bytes).expect("awk_sprintf: format spec must be ASCII");
    // BWK normalizes numeric temporaries to +0, while a numeric conversion
    // of a string such as "-0" retains its sign.
    let number = match arg {
        AwkValue::Number(n) => *n + 0.0,
        _ => arg.as_number(),
    };
    match conv {
        b'd' | b'i' | b'o' | b'x' | b'X' | b'u' => {
            out.extend(integer_format(spec_bytes, conv, arg.as_number()));
        }
        b'a' | b'A' => out.extend(hex_float(spec_bytes, number)),
        b'e' | b'E' | b'f' | b'g' | b'G' => {
            let s = crate::number_format::format(spec, number).unwrap_or_default();
            out.extend_from_slice(s.as_bytes());
        }
        b'c' => {
            // Numeric input fields are StrNum: %c converts their numeric value,
            // whereas an explicit string contributes its first byte.
            let bytes = match arg {
                AwkValue::String(s) if !s.is_empty() => s[..crate::text::next_len(s)].to_vec(),
                AwkValue::String(_) => vec![0],
                _ if crate::text::multibyte() && arg.as_number() >= 128.0 => {
                    char::from_u32(arg.as_number() as u32)
                        .unwrap_or('\u{fffd}')
                        .to_string()
                        .into_bytes()
                }
                _ => vec![arg.as_number() as i64 as u8],
            };
            let (width, precision, left, zero) = parse_s_flags(spec_bytes);
            let multibyte = crate::text::multibyte() && bytes.len() > 1;
            let padding = width.saturating_sub(if multibyte {
                precision.unwrap_or(1).min(1)
            } else {
                1
            });
            if !left {
                let pad = if zero && cfg!(target_os = "macos") && !multibyte {
                    b'0'
                } else {
                    b' '
                };
                out.extend(std::iter::repeat_n(pad, padding));
            }
            out.extend(bytes);
            if left {
                out.extend(std::iter::repeat_n(b' ', padding));
            }
        }
        b's' => {
            // Phase 7.5: %s emette `arg.as_string_convfmt(convfmt)` integro come bytes raw.
            // Width/precision count the character units selected by LC_CTYPE.
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
                    truncated = &truncated[..crate::text::byte_offset(truncated, p)];
                }
                let pad_count = width.saturating_sub(crate::text::len(truncated));
                let space_pad = left_align
                    || cfg!(all(target_os = "linux", target_env = "gnu"))
                    || (crate::text::multibyte() && s_bytes.iter().any(|b| *b >= 128));
                let pad_byte = if zero_pad && !space_pad { b'0' } else { b' ' };
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

fn negative_precision_spec(spec: &[u8], precision: u32) -> Vec<u8> {
    let mut normalized = vec![b'%'];
    normalized.extend(
        spec.iter()
            .skip(1)
            .take_while(|b| b"-+ #0".contains(b))
            .copied(),
    );
    normalized.extend(format!("-{precision}.0").bytes());
    normalized
}

// BWK textually substitutes a negative precision, then adds `j` for integers.
// In glibc the minus after the dot is an unknown conversion: printf renders
// the parsed flags/width and zero precision, followed by the unparsed suffix.
// Keep this byte-based so formats containing NUL outside this spec stay intact.
fn glibc_negative_precision(spec: &[u8], conv: u8) -> Option<Vec<u8>> {
    let dot = spec.windows(2).position(|pair| pair == b".-")?;
    let prefix = &spec[..dot];
    let (width, _, left, zero) = parse_s_flags(prefix);
    let mut literal = vec![b'%'];
    if prefix.contains(&b'#') {
        literal.push(b'#');
    }
    if prefix.contains(&b'+') {
        literal.push(b'+');
    } else if prefix.contains(&b' ') {
        literal.push(b' ');
    }
    if left {
        literal.push(b'-');
    } else if zero {
        literal.push(b'0');
    }
    if width != 0 {
        literal.extend(width.to_string().bytes());
    }
    literal.extend_from_slice(b".0");
    literal.extend_from_slice(&spec[dot + 1..spec.len() - 1]);
    if b"diouxX".contains(&conv) {
        literal.push(b'j');
    }
    literal.push(conv);
    Some(literal)
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

fn integer_format(spec: &[u8], conv: u8, value: f64) -> Vec<u8> {
    let (width, precision, left, zero) = parse_s_flags(spec);
    let signed = matches!(conv, b'd' | b'i');
    let integer = value as i64;
    let magnitude = if signed {
        integer.unsigned_abs()
    } else if cfg!(all(
        target_os = "linux",
        target_env = "gnu",
        target_arch = "x86_64"
    )) && (-9_223_372_036_854_775_808.0..0.0).contains(&value)
    {
        // The pinned C oracle on Linux x86-64 converts negative doubles
        // through a signed 64-bit result. ARM64 saturates to zero instead.
        // This is native compatibility, not a portable C unsigned rule.
        integer as u64
    } else {
        value as u64
    };
    let mut digits = match conv {
        b'o' => format!("{magnitude:o}"),
        b'x' => format!("{magnitude:x}"),
        b'X' => format!("{magnitude:X}"),
        _ => magnitude.to_string(),
    };
    if precision == Some(0) && magnitude == 0 {
        digits.clear();
    }
    if let Some(p) = precision {
        digits = format!("{}{digits}", "0".repeat(p.saturating_sub(digits.len())));
    }
    let sign = if signed && integer < 0 {
        "-"
    } else if signed && spec.contains(&b'+') {
        "+"
    } else if signed && spec.contains(&b' ') {
        " "
    } else {
        ""
    };
    let prefix = if spec.contains(&b'#') {
        match conv {
            b'o' if !digits.starts_with('0') => "0",
            b'x' if magnitude != 0 => "0x",
            b'X' if magnitude != 0 => "0X",
            _ => "",
        }
    } else {
        ""
    };
    let pad = width.saturating_sub(sign.len() + prefix.len() + digits.len());
    if left {
        format!("{sign}{prefix}{digits}{}", " ".repeat(pad))
    } else if zero && precision.is_none() {
        format!("{sign}{prefix}{}{digits}", "0".repeat(pad))
    } else {
        format!("{}{sign}{prefix}{digits}", " ".repeat(pad))
    }
    .into_bytes()
}

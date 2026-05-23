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
/// non valido (conversion byte non in `diouxXeEfgGcs`) viene emesso letterale.
pub(super) fn awk_sprintf(fmt: &[u8], args: &[AwkValue]) -> Vec<u8> {
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
            if b"diouxXeEfgGcs".contains(&c) {
                conv = Some(c);
                break;
            }
        }
        let spec_bytes = &fmt[spec_start..i];
        match conv {
            None => {
                // EOF dentro lo spec — emetti letterale e termina.
                out.extend_from_slice(spec_bytes);
                return out;
            }
            Some(c) => {
                let arg = args
                    .get(arg_idx)
                    .cloned()
                    .unwrap_or(AwkValue::Uninitialized);
                arg_idx += 1;
                format_one(spec_bytes, c, &arg, &mut out);
            }
        }
    }
    out
}

fn format_one(spec_bytes: &[u8], conv: u8, arg: &AwkValue, out: &mut Vec<u8>) {
    // Lo spec è ASCII puro per costruzione (% + flags `-+ #0` + digit + `.` +
    // conversion byte). `from_utf8` è O(spec.len) ma piccolo (raramente >10B).
    let spec = std::str::from_utf8(spec_bytes)
        .expect("awk_sprintf: format spec must be ASCII");
    match conv {
        b'd' | b'i' => {
            let s = sprintf::sprintf!(spec, arg.as_number() as i64).unwrap_or_default();
            out.extend_from_slice(s.as_bytes());
        }
        b'o' | b'x' | b'X' | b'u' => {
            let s = sprintf::sprintf!(spec, arg.as_number() as u64).unwrap_or_default();
            out.extend_from_slice(s.as_bytes());
        }
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
            // Phase 7.5: %s emette `arg.as_string()` integro come bytes raw.
            // Width/precision applicati byte-aware.
            let s_bytes = arg.as_string();
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

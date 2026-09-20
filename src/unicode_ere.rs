//! Compile BWK rune atoms to fixed-width keys for the existing byte DFA.
//! This preserves leftmost-longest matching and byte offsets even for mixed
//! UTF-8/invalid bytes, without replacement characters or sentinel collisions.
use crate::text;
const MAX_RUNE: u32 = 0x1fffff;

pub(crate) fn encode(bytes: &[u8]) -> (Vec<u8>, Vec<usize>) {
    // Leave room for a short subject and its end offset without growing
    // the vector after the first few runes. Long subjects still grow normally.
    let mut offsets = Vec::with_capacity(8);
    let encoded = encode_with_offsets(bytes, |i| offsets.push(i));
    (encoded, offsets)
}

/// Boolean searches need rune keys, but never translate positions back to bytes.
pub(crate) fn encode_keys(bytes: &[u8]) -> Vec<u8> {
    encode_with_offsets(bytes, |_| {})
}

fn encode_with_offsets(bytes: &[u8], mut visit: impl FnMut(usize)) -> Vec<u8> {
    // Keys use four bytes per rune. A small initial allocation avoids
    // repeated growth on short subjects; empty boolean searches allocate none.
    let capacity = if bytes.is_empty() {
        0
    } else {
        bytes.len().max(32)
    };
    let mut encoded = Vec::with_capacity(capacity);
    let mut i = 0;
    while i < bytes.len() {
        visit(i);
        let (rune, n) = text::rune(&bytes[i..]);
        encoded.extend_from_slice(&rune.to_be_bytes());
        i += n;
    }
    visit(i);
    encoded
}
fn hex(b: u8) -> String {
    format!("\\x{b:02x}")
}
fn atom(c: u32) -> String {
    format!(
        "(?:{})",
        c.to_be_bytes().into_iter().map(hex).collect::<String>()
    )
}

/// Split an integer interval into lexicographic byte ranges. At most two
/// boundary branches per byte, rather than enumerating a Unicode-sized class.
fn interval(lo: u32, hi: u32) -> String {
    fn part(a: &[u8], b: &[u8]) -> String {
        if a.is_empty() {
            return String::new();
        }
        if a[0] == b[0] {
            return hex(a[0]) + &part(&a[1..], &b[1..]);
        }
        let n = a.len() - 1;
        let mut choices = vec![hex(a[0]) + &part(&a[1..], &vec![255; n])];
        if u16::from(b[0]) > u16::from(a[0]) + 1 {
            choices.push(format!(
                "[{}-{}]{}",
                hex(a[0] + 1),
                hex(b[0] - 1),
                "[\\x00-\\xff]".repeat(n)
            ));
        }
        choices.push(hex(b[0]) + &part(&vec![0; n], &b[1..]));
        format!("(?:{})", choices.join("|"))
    }
    part(&lo.to_be_bytes(), &hi.to_be_bytes())
}
fn take(bytes: &[u8], i: &mut usize) -> Result<u32, String> {
    if *i >= bytes.len() {
        return Err("trailing backslash in regular expression".into());
    }
    let (c, n) = text::rune(&bytes[*i..]);
    *i += n;
    Ok(c)
}
fn quoted(bytes: &[u8], i: &mut usize) -> Result<u32, String> {
    let c = take(bytes, i)?;
    let simple = match c {
        97 => Some(7),
        98 => Some(8),
        116 => Some(9),
        110 => Some(10),
        118 => Some(11),
        102 => Some(12),
        114 => Some(13),
        _ => None,
    };
    if let Some(c) = simple {
        return Ok(c);
    }
    if c == 120 || c == 117 {
        let mut value = 0u32;
        for _ in 0..if c == 120 { 2 } else { 8 } {
            let Some(d) = bytes.get(*i).and_then(|b| (*b as char).to_digit(16)) else {
                break;
            };
            value = (value << 4) | d;
            *i += 1;
        }
        if value > MAX_RUNE {
            return Err("regex escape exceeds supported rune range".into());
        }
        return Ok(value);
    }
    if (48..=55).contains(&c) {
        let mut value = c - 48;
        for _ in 0..2 {
            let Some(&b @ b'0'..=b'7') = bytes.get(*i) else {
                break;
            };
            value = value * 8 + u32::from(b - b'0');
            *i += 1;
        }
        return Ok(value);
    }
    Ok(c)
}
fn class(bytes: &[u8], i: &mut usize) -> Result<String, String> {
    let negate = bytes.get(*i) == Some(&b'^');
    if negate {
        *i += 1;
    }
    let mut tokens = Vec::new();
    while *i < bytes.len() && (bytes[*i] != b']' || tokens.is_empty()) {
        if bytes[*i] == b'\\' {
            *i += 1;
            tokens.push((quoted(bytes, i)?, false));
        } else if bytes.get(*i..*i + 2) == Some(b"[:") {
            let start = *i + 2;
            let end = bytes[start..]
                .windows(2)
                .position(|p| p == b":]")
                .map(|n| start + n)
                .ok_or("unterminated named character class")?;
            let name = std::str::from_utf8(&bytes[start..end])
                .map_err(|_| "invalid character class name")?;
            for b in 1..=255 {
                if text::class_member(name, b)? {
                    tokens.push((u32::from(b), false));
                }
            }
            *i = end + 2;
        } else {
            let c = take(bytes, i)?;
            tokens.push((c, c == 45));
        }
    }
    if bytes.get(*i) != Some(&b']') {
        return Err("unterminated character class".into());
    }
    *i += 1;
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    let mut t = 0;
    while t < tokens.len() {
        let (c, dash) = tokens[t];
        if dash && !ranges.is_empty() && t + 1 < tokens.len() {
            let start = ranges.last().unwrap().1;
            let end = tokens[t + 1].0;
            if start > end {
                let last = ranges.last_mut().unwrap();
                if last.0 == last.1 {
                    ranges.pop();
                } else {
                    last.1 -= 1;
                }
            } else {
                ranges.last_mut().unwrap().1 = end;
            }
            t += 2;
        } else {
            ranges.push((c, c));
            t += 1;
        }
    }
    ranges.sort_unstable();
    let mut merged: Vec<(u32, u32)> = Vec::new();
    for (a, b) in ranges {
        if let Some(last) = merged.last_mut()
            && a <= last.1.saturating_add(1)
        {
            last.1 = last.1.max(b);
        } else {
            merged.push((a, b));
        }
    }
    if negate {
        let mut complement = Vec::new();
        let mut start = 0;
        for (a, b) in merged {
            if start < a {
                complement.push((start, a - 1));
            }
            start = b + 1;
        }
        if start <= MAX_RUNE {
            complement.push((start, MAX_RUNE));
        }
        merged = complement;
    }
    if merged.is_empty() {
        return Ok("(?:)".into());
    }
    Ok(format!(
        "(?:{})",
        merged
            .into_iter()
            .map(|(a, b)| interval(a, b))
            .collect::<Vec<_>>()
            .join("|")
    ))
}
pub(crate) fn compile(bytes: &[u8]) -> Result<String, String> {
    compile_mode(bytes, false)
}

// Outside BWK's structural rune range, so EOF never collides with input,
// including the Rust extension for embedded NUL.
pub(crate) const STREAM_EOF: u32 = MAX_RUNE + 1;
pub(crate) fn compile_stream(bytes: &[u8]) -> Result<String, String> {
    compile_mode(bytes, true)
}

fn compile_mode(bytes: &[u8], stream: bool) -> Result<String, String> {
    let (mut i, mut depth) = (0, 0usize);
    let mut out = String::new();
    while i < bytes.len() {
        let b = bytes[i];
        i += 1;
        match b {
            b'\\' => out.push_str(&atom(quoted(bytes, &mut i)?)),
            b'[' => out.push_str(&class(bytes, &mut i)?),
            b'.' if stream => out.push_str(&format!("(?:{})", interval(0, MAX_RUNE))),
            b'.' => out.push_str("(?:[\\x00-\\xff]{4})"),
            b'(' => {
                depth += 1;
                out.push('(');
            }
            b')' if depth > 0 => {
                depth -= 1;
                out.push(')');
            }
            b'{' if bytes.get(i).is_some_and(u8::is_ascii_digit) => {
                out.push('{');
                while i < bytes.len() && (bytes[i].is_ascii_digit() || b",}".contains(&bytes[i])) {
                    let c = bytes[i];
                    out.push(c as char);
                    i += 1;
                    if c == b'}' {
                        break;
                    }
                }
            }
            b'$' if bytes.get(i) == Some(&b'^') => {
                return Err("invalid adjacent regex anchors $^".into());
            }
            b'$' if stream => out.push_str(&atom(STREAM_EOF)),
            b'^' | b'$' | b'*' | b'+' | b'?' | b'|' => out.push(b as char),
            _ => {
                i -= 1;
                out.push_str(&atom(take(bytes, &mut i)?));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compiled_intervals_cover_only_the_requested_runes() {
        for (lo, hi) in [
            (0, MAX_RUNE),
            (0, 0),
            (0xff, 0x101),
            (0xffff, 0x10001),
            (0x390, 0x3ff),
            (0x100, 0x10ffff),
        ] {
            let re = regex::bytes::RegexBuilder::new(&format!("^(?:{})$", interval(lo, hi)))
                .unicode(false)
                .build()
                .unwrap();
            let mut values = vec![lo, hi, lo.saturating_sub(1), hi + 1];
            let mut seed = 17u32;
            for _ in 0..1000 {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                values.push(seed & MAX_RUNE);
            }
            for n in values {
                assert_eq!(
                    re.is_match(&n.to_be_bytes()),
                    (lo..=hi).contains(&n),
                    "{lo:x}..{hi:x}: {n:x}"
                );
            }
        }
    }
    #[test]
    fn encoding_preserves_offsets_and_invalid_bytes_without_sentinels() {
        let (bytes, offsets) = encode(b"A\xc3\xa9\xff\xf0\x9f\x98\x80\0");
        let runes: Vec<_> = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_be_bytes(*b))
            .collect();
        assert_eq!(runes, [65, 233, 255, 0x1f600, 0]);
        assert_eq!(offsets, [0, 1, 3, 4, 8, 9]);
    }
}

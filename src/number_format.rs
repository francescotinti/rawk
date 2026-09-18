//! Decimal floating-point formatting using Rust's correctly rounded binary64
//! conversion, shared by printf and numeric-to-string coercion.

pub(crate) fn format(spec: &str, value: f64) -> Option<String> {
    let bytes = spec.as_bytes();
    if bytes.first() != Some(&b'%') {
        return None;
    }
    let mut i = 1;
    let (mut left, mut plus, mut space, mut alternate, mut zero) =
        (false, false, false, false, false);
    while let Some(b) = bytes.get(i) {
        match b {
            b'-' => left = true,
            b'+' => plus = true,
            b' ' => space = true,
            b'#' => alternate = true,
            b'0' => zero = true,
            _ => break,
        }
        i += 1;
    }
    let start = i;
    while bytes.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    let width = if i == start {
        0
    } else {
        spec[start..i].parse::<usize>().ok()?
    };
    let mut precision = 6;
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        precision = if i == start {
            0
        } else {
            spec[start..i].parse::<usize>().ok()?
        };
    }
    let conv = *bytes.get(i)?;
    if i + 1 != bytes.len() || !b"eEfgG".contains(&conv) {
        return None;
    }
    let magnitude = value.abs();
    let mut body = if value.is_nan() {
        "nan".into()
    } else if value.is_infinite() {
        "inf".into()
    } else if conv == b'f' {
        let mut s = format!("{magnitude:.precision$}");
        if alternate && precision == 0 {
            s.push('.');
        }
        s
    } else if matches!(conv, b'e' | b'E') {
        scientific(magnitude, precision, alternate, false)
    } else {
        let significant = precision.max(1);
        // Decide notation from the rounded exponent, including carry across
        // a power of ten, rather than an approximate logarithm of the input.
        let rounded = format!("{:.*e}", significant - 1, magnitude);
        let exponent = rounded.split_once('e')?.1.parse::<i32>().ok()?;
        if exponent < -4 || exponent >= significant as i32 {
            scientific(magnitude, significant - 1, alternate, !alternate)
        } else {
            let decimals = (significant as i64 - 1 - i64::from(exponent)) as usize;
            let mut s = format!("{magnitude:.decimals$}");
            if alternate {
                if decimals == 0 {
                    s.push('.');
                }
            } else {
                trim_fraction(&mut s);
            }
            s
        }
    };
    if matches!(conv, b'E' | b'G') {
        body.make_ascii_uppercase();
    }
    let sign = if value.is_sign_negative() {
        "-"
    } else if plus {
        "+"
    } else if space {
        " "
    } else {
        ""
    };
    let padding = width.saturating_sub(sign.len() + body.len());
    Some(if left {
        format!("{sign}{body}{}", " ".repeat(padding))
    } else if zero && value.is_finite() {
        format!("{sign}{}{body}", "0".repeat(padding))
    } else {
        format!("{}{sign}{body}", " ".repeat(padding))
    })
}

fn trim_fraction(text: &mut String) {
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
}

fn scientific(value: f64, precision: usize, alternate: bool, trim: bool) -> String {
    let text = format!("{value:.precision$e}");
    let (fraction, exponent) = text
        .split_once('e')
        .expect("Rust scientific format has exponent");
    let exponent: i32 = exponent
        .parse()
        .expect("Rust scientific exponent is integer");
    let mut fraction = fraction.to_owned();
    if trim {
        trim_fraction(&mut fraction);
    }
    if alternate && !fraction.contains('.') {
        fraction.push('.');
    }
    format!(
        "{fraction}e{}{:02}",
        if exponent < 0 { "-" } else { "+" },
        exponent.unsigned_abs()
    )
}

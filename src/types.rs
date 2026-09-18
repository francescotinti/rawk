/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: A high-fidelity port of the historic AWK language from C to Rust.
 */

use crate::ast::Statement;
use std::collections::HashMap;

use rand::rngs::StdRng;

/// Convert a raw byte sequence into a `&str` regex pattern, promoting each
/// non-ASCII byte to its `\xNN` escape form. ASCII bytes (including regex
/// metacharacters and backslash) pass through unchanged so AWK regex syntax
/// is preserved. The result is always valid UTF-8 and represents the same
/// byte-literal semantics when fed to `regex::bytes::Regex` with Unicode
/// mode disabled.
pub(crate) fn regex_pattern_from_bytes(re: &[u8]) -> String {
    // Fast path: if the input is already valid UTF-8 with no high bytes,
    // it's safe to use as-is. Non-ASCII bytes (>=0x80) must be escaped
    // regardless of UTF-8 validity, since their codepoints (when valid)
    // would not match raw bytes 1:1 in the haystack.
    if re.iter().all(|&b| b < 0x80) {
        // SAFETY: all bytes are ASCII, so the slice is valid UTF-8.
        return std::str::from_utf8(re).unwrap().to_string();
    }
    let mut out = String::with_capacity(re.len() + 8);
    for &b in re {
        if b < 0x80 {
            out.push(b as char);
        } else {
            out.push_str(&format!("\\x{:02X}", b));
        }
    }
    out
}

/// Valore AWK polimorfo. Le coercioni Number↔String seguono le regole POSIX
/// (numeric context usa `as_number`, string context usa `as_string` con `CONVFMT`/`OFMT`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AwkValue {
    /// Variabile mai assegnata. Coerce a `0` in contesto numerico, `""` in contesto stringa.
    Uninitialized,
    /// Valore numerico (double-precision IEEE 754).
    Number(f64),
    /// Stringa esplicita: letterale del programma o risultato di concatenazione.
    String(Vec<u8>),
    /// Dual-typed: input proveniente da `getline`, `$N` o argv che parsea numericamente.
    /// La cache `(testo, valore)` evita di ri-parsare a ogni confronto numerico.
    StrNum(Vec<u8>, f64),
}

impl AwkValue {
    pub(crate) fn from_str_num(s: Vec<u8>) -> Self {
        let parsed = std::str::from_utf8(&s)
            .ok()
            .and_then(|t| t.trim().parse::<f64>().ok());
        match parsed {
            Some(n) => AwkValue::StrNum(s, n),
            None => AwkValue::String(s),
        }
    }

    pub(crate) fn as_number(&self) -> f64 {
        match self {
            AwkValue::Uninitialized => 0.0,
            AwkValue::Number(n) => *n,
            AwkValue::String(s) => numeric_prefix(s),
            AwkValue::StrNum(_, n) => *n,
        }
    }

    pub(crate) fn as_string(&self) -> Vec<u8> {
        self.as_string_convfmt(b"%.6g")
    }

    /// Converte il valore in bytes secondo CONVFMT/OFMT. `fmt` è atteso ASCII
    /// (CONVFMT/OFMT default `%.6g`); il path numerico delega a
    /// `format_number_awk` che richiede `&str` — convertiamo con `from_utf8`
    /// e fallback `%.6g` se l'utente setta CONVFMT con byte non-UTF-8
    /// (comportamento undefined POSIX).
    pub(crate) fn as_string_convfmt(&self, fmt: &[u8]) -> Vec<u8> {
        match self {
            AwkValue::Uninitialized => Vec::new(),
            AwkValue::String(s) => s.clone(),
            AwkValue::StrNum(s, _) => s.clone(),
            AwkValue::Number(n) => {
                let fmt_str = std::str::from_utf8(fmt).unwrap_or("%.6g");
                format_number_awk(*n, fmt_str).into_bytes()
            }
        }
    }

    pub(crate) fn is_truthy(&self) -> bool {
        match self {
            AwkValue::Uninitialized => false,
            AwkValue::Number(n) => *n != 0.0,
            AwkValue::String(s) => !s.is_empty(), // Standard awk: empty string is false, non-empty is true (even "0")
            AwkValue::StrNum(_, n) => *n != 0.0,
        }
    }

    // AWK comparison rules MVP: try numeric comparison first, fallback to string
    fn numeric_values(&self, other: &Self) -> Option<(f64, f64)> {
        let l = match self {
            AwkValue::Number(n) => Some(*n),
            AwkValue::String(_) => None,
            AwkValue::StrNum(_, n) => Some(*n),
            AwkValue::Uninitialized => Some(0.0),
        };
        let r = match other {
            AwkValue::Number(n) => Some(*n),
            AwkValue::String(_) => None,
            AwkValue::StrNum(_, n) => Some(*n),
            AwkValue::Uninitialized => Some(0.0),
        };
        if let (Some(ln), Some(rn)) = (l, r) {
            Some((ln, rn))
        } else {
            None
        }
    }

    pub(crate) fn is_eq(&self, other: &Self, fmt: &[u8]) -> AwkValue {
        if let Some((l, r)) = self.numeric_values(other) {
            AwkValue::Number(if l == r { 1.0 } else { 0.0 })
        } else {
            AwkValue::Number(
                if self.as_string_convfmt(fmt) == other.as_string_convfmt(fmt) {
                    1.0
                } else {
                    0.0
                },
            )
        }
    }

    pub(crate) fn is_lt(&self, other: &Self, fmt: &[u8]) -> AwkValue {
        if let Some((l, r)) = self.numeric_values(other) {
            AwkValue::Number(if l < r { 1.0 } else { 0.0 })
        } else {
            AwkValue::Number(
                if self.as_string_convfmt(fmt) < other.as_string_convfmt(fmt) {
                    1.0
                } else {
                    0.0
                },
            )
        }
    }

    pub(crate) fn is_gt(&self, other: &Self, fmt: &[u8]) -> AwkValue {
        if let Some((l, r)) = self.numeric_values(other) {
            AwkValue::Number(if l > r { 1.0 } else { 0.0 })
        } else {
            AwkValue::Number(
                if self.as_string_convfmt(fmt) > other.as_string_convfmt(fmt) {
                    1.0
                } else {
                    0.0
                },
            )
        }
    }

    pub(crate) fn add(&self, other: &Self) -> Self {
        AwkValue::Number(self.as_number() + other.as_number())
    }

    pub(crate) fn sub(&self, other: &Self) -> Self {
        AwkValue::Number(self.as_number() - other.as_number())
    }

    pub(crate) fn mul(&self, other: &Self) -> Self {
        AwkValue::Number(self.as_number() * other.as_number())
    }

    pub(crate) fn div(&self, other: &Self) -> Self {
        let divisor = other.as_number();
        if divisor == 0.0 {
            eprintln!("rawk: warning: division by zero");
            return AwkValue::Number(0.0);
        }
        AwkValue::Number(self.as_number() / divisor)
    }

    pub(crate) fn rem(&self, other: &Self) -> Self {
        let divisor = other.as_number();
        if divisor == 0.0 {
            eprintln!("rawk: warning: division by zero in mod");
            return AwkValue::Number(0.0);
        }
        AwkValue::Number(self.as_number() % divisor)
    }

    pub(crate) fn pow(&self, other: &Self) -> Self {
        AwkValue::Number(self.as_number().powf(other.as_number()))
    }
}

fn numeric_prefix(bytes: &[u8]) -> f64 {
    let b = bytes.trim_ascii_start();
    let mut i = usize::from(b.first().is_some_and(|c| *c == b'+' || *c == b'-'));
    let mut digits = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
        digits += 1;
    }
    if b.get(i) == Some(&b'.') {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return 0.0;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        let exp = i;
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            i = exp;
        }
    }
    std::str::from_utf8(&b[..i])
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}

fn format_number_awk(n: f64, fmt: &str) -> String {
    let mut text = format_number_unbounded(n, fmt);
    let mut limit = text.len().min(255);
    while !text.is_char_boundary(limit) {
        limit -= 1;
    }
    text.truncate(limit);
    text
}

fn format_number_unbounded(n: f64, fmt: &str) -> String {
    if !n.is_finite() {
        let sign = if n.is_sign_negative() { "-" } else { "+" };
        return format!("{sign}{}", if n.is_nan() { "nan" } else { "inf" });
    }
    // Path A: i64 fast-path per integer-like piccoli (esistente)
    if n.is_finite() && n == n.trunc() && n.abs() < 1e16 {
        return format!("{}", n as i64);
    }
    // Path B: %.0f per integer-like grandi entro f64 precision (NUOVO)
    if n.is_finite() && n == n.trunc() && n.abs() < 1e21 {
        return sprintf::sprintf!("%.0f", n).unwrap_or_else(|_| n.to_string());
    }
    // Rust's formatter avoids sprintf's incorrect rounding with very large fixed precision.
    if let Some(precision) = fmt
        .strip_prefix("%.")
        .and_then(|s| s.strip_suffix('f'))
        .and_then(|s| s.parse::<usize>().ok())
    {
        // The original conversion buffer is 256 bytes including its NUL.
        let mut result = format!("{n:.precision$}", precision = precision.min(1024));
        result.truncate(result.len().min(255));
        return result;
    }
    // Path C: usa fmt richiesto, strip dot orfani (esistente)
    let s = sprintf::sprintf!(fmt, n).unwrap_or_else(|_| n.to_string());
    // Fix 1 (Step 12-bis): trailing dot in fixed notation, "X." -> "X"
    let s = if s.ends_with('.') {
        s[..s.len() - 1].to_string()
    } else {
        s
    };
    // Fix 2 (Step 13): orphan dot before exponent, "X.e+Y" -> "Xe+Y"
    s.replace(".e+", "e+")
        .replace(".e-", "e-")
        .replace(".E+", "E+")
        .replace(".E-", "E-")
}

/// Stream di output AWK aperto: file regolare oppure pipe a un comando.
/// Le `Pipe` mantengono lo `Child` per poter attendere `wait()` su `close()`.
pub(crate) enum OutputStream {
    File(Box<dyn std::io::Write>),
    Pipe {
        stdin: Box<dyn std::io::Write>,
        child: std::process::Child,
    },
}

impl OutputStream {
    pub(crate) fn writer(&mut self) -> &mut dyn std::io::Write {
        match self {
            OutputStream::File(w) => w.as_mut(),
            OutputStream::Pipe { stdin, .. } => stdin.as_mut(),
        }
    }
}

/// Stream di input AWK aperto: file regolare oppure pipe da un comando (`"cmd" | getline`).
/// Come `OutputStream::Pipe`, conserva lo `Child` per `wait()` su `close()`.
pub(crate) enum InputStream {
    File(Box<crate::input::RecordReader>),
    Pipe {
        stdout: Box<crate::input::RecordReader>,
        child: std::process::Child,
    },
}

impl InputStream {
    pub(crate) fn reader(&mut self) -> &mut crate::input::RecordReader {
        match self {
            InputStream::File(r) => r.as_mut(),
            InputStream::Pipe { stdout, .. } => stdout.as_mut(),
        }
    }
}

/// Stato runtime di un programma AWK in esecuzione: record correnti, campi,
/// variabili scalari/array (con scope locali per funzioni utente), stream
/// aperti, cache regex, parametri di format (`CONVFMT`/`OFMT`) e segnali di
/// explicit main-input state and function scopes.
pub(crate) struct EvalContext {
    pub(crate) csv: bool,
    pub(crate) nr: usize,  // Number of Records read so far
    pub(crate) fnr: usize, // Number of Records in current file
    pub(crate) nf: usize,  // Number of Fields in current record
    pub(crate) fs: Vec<u8>,
    pub(crate) fields: Vec<AwkValue>,
    pub(crate) record: Vec<u8>,
    pub(crate) vars: HashMap<String, AwkValue>,
    pub(crate) arrays: HashMap<String, HashMap<Vec<u8>, AwkValue>>,
    pub(crate) out_files: HashMap<String, OutputStream>,
    pub(crate) in_files: HashMap<String, InputStream>,
    pub(crate) rng: StdRng,
    pub(crate) array_scopes: Vec<HashMap<String, String>>,
    pub(crate) array_params: HashMap<String, std::collections::HashSet<String>>,
    pub(crate) local_scopes: Vec<HashMap<String, AwkValue>>,
    pub(crate) functions: HashMap<String, (Vec<String>, Vec<Statement>)>,
    pub(crate) regex_cache: HashMap<Vec<u8>, std::rc::Rc<crate::ere::Ere>>,
    pub(crate) convfmt: Vec<u8>,
    pub(crate) ofmt: Vec<u8>,
    pub(crate) input: crate::input::MainInput,
    pub(crate) rules: std::rc::Rc<Vec<crate::runner::CompiledRule>>,

    pub(crate) exit_code: i32,
    pub(crate) ranges: std::collections::HashSet<usize>,
}

impl EvalContext {
    pub(crate) fn new(fs: &[u8]) -> Self {
        let mut vars = HashMap::new();
        vars.insert("SUBSEP".to_string(), AwkValue::String(b"\x1C".to_vec()));
        Self {
            csv: false,
            nr: 0,
            fnr: 0,
            nf: 0,
            fs: fs.to_vec(),
            fields: Vec::new(),
            record: Vec::new(),
            vars,
            arrays: HashMap::new(),
            out_files: HashMap::new(),
            in_files: HashMap::new(),
            rng: rand::SeedableRng::seed_from_u64(0),
            array_scopes: Vec::new(),
            array_params: HashMap::new(),
            local_scopes: Vec::new(),
            functions: HashMap::new(),
            regex_cache: HashMap::new(),
            convfmt: b"%.6g".to_vec(),
            ofmt: b"%.6g".to_vec(),
            input: Default::default(),
            rules: Default::default(),

            exit_code: 0,
            ranges: Default::default(),
        }
    }

    pub(crate) fn compile_or_get_regex(
        &mut self,
        pattern: &[u8],
    ) -> Result<std::rc::Rc<crate::ere::Ere>, crate::runner::FlowControl> {
        if let Some(re) = self.regex_cache.get(pattern) {
            return Ok(re.clone());
        }
        let compiled = std::rc::Rc::new(
            crate::ere::Ere::new(pattern).map_err(crate::runner::FlowControl::Error)?,
        );
        // Bounded cache; clearing changes performance only, never semantics.
        if self.regex_cache.len() >= 64 {
            self.regex_cache.clear();
        }
        self.regex_cache.insert(pattern.to_vec(), compiled.clone());
        Ok(compiled)
    }

    /// Resplit a record without changing the counters for input consumption.
    pub(crate) fn update_record(&mut self, line: &[u8]) -> Result<(), crate::runner::FlowControl> {
        self.record = line.to_vec();
        let fields = if self.csv {
            crate::input::csv_fields(line)
        } else {
            let separator = if self.get_var("RS").as_string().is_empty()
                && self.fs != b" "
                && !self.fs.is_empty()
            {
                if self.fs.len() == 1 {
                    format!("(?:\\x{:02X}|\\n)", self.fs[0]).into_bytes()
                } else {
                    [b"(?:".as_slice(), self.fs.as_slice(), b"|\n)"].concat()
                }
            } else {
                self.fs.clone()
            };
            if separator.len() > 1 {
                let re = self.compile_or_get_regex(&separator)?;
                crate::ere::split_using(line, &re)
            } else {
                crate::ere::split(line, &separator).map_err(crate::runner::FlowControl::Error)?
            }
        };
        self.fields = fields.into_iter().map(AwkValue::from_str_num).collect();
        self.nf = self.fields.len();
        Ok(())
    }

    /// Get $N. If n == 0, returns $0 (the whole record). If n > NF, returns Uninitialized.
    pub(crate) fn get_field(&self, n: usize) -> AwkValue {
        if n == 0 {
            AwkValue::from_str_num(self.record.clone())
        } else if n <= self.nf {
            self.fields[n - 1].clone()
        } else {
            AwkValue::Uninitialized
        }
    }

    pub(crate) fn set_field(
        &mut self,
        n: usize,
        value: AwkValue,
    ) -> Result<(), crate::runner::FlowControl> {
        if n == 0 {
            self.update_record(&value.as_string_convfmt(&self.convfmt))?;
        } else {
            while self.fields.len() < n {
                self.fields.push(AwkValue::String(Vec::new()));
            }
            self.fields[n - 1] = value;
            self.nf = self.fields.len();

            // Rebuild $0 using OFS
            let ofs = self.get_var("OFS").as_string();
            let mut parts: Vec<Vec<u8>> = Vec::new();
            for f in &self.fields {
                parts.push(f.as_string_convfmt(&self.convfmt));
            }
            self.record = parts.join(ofs.as_slice());
        }
        Ok(())
    }

    pub(crate) fn get_var(&self, name: &str) -> AwkValue {
        if self
            .array_scopes
            .last()
            .is_some_and(|scope| scope.contains_key(name))
        {
            return AwkValue::Uninitialized;
        }
        if let Some(scope) = self.local_scopes.last()
            && let Some(val) = scope.get(name)
        {
            return val.clone();
        }
        match name {
            "NF" => return AwkValue::Number(self.nf as f64),
            "NR" => return AwkValue::Number(self.nr as f64),
            "FNR" => return AwkValue::Number(self.fnr as f64),
            "FS" => return AwkValue::String(self.fs.clone()),
            "CONVFMT" => return AwkValue::String(self.convfmt.clone()),
            "OFMT" => return AwkValue::String(self.ofmt.clone()),
            _ => {}
        }
        self.vars
            .get(name)
            .cloned()
            .unwrap_or(AwkValue::Uninitialized)
    }

    pub(crate) fn set_var(&mut self, name: &str, value: AwkValue) {
        if let Some(scope) = self.local_scopes.last_mut()
            && scope.contains_key(name)
        {
            scope.insert(name.to_string(), value);
            return;
        }
        match name {
            "NF" => {
                let new_nf = value.as_number() as usize;
                if new_nf < self.fields.len() {
                    self.fields.truncate(new_nf);
                } else {
                    while self.fields.len() < new_nf {
                        self.fields.push(AwkValue::String(Vec::new()));
                    }
                }
                self.nf = new_nf;
                let ofs = self
                    .vars
                    .get("OFS")
                    .map(|v| v.as_string())
                    .unwrap_or_else(|| b" ".to_vec());
                let parts: Vec<Vec<u8>> = self
                    .fields
                    .iter()
                    .map(|f| f.as_string_convfmt(&self.convfmt))
                    .collect();
                self.record = parts.join(ofs.as_slice());
            }
            "NR" => self.nr = value.as_number() as usize,
            "FNR" => self.fnr = value.as_number() as usize,
            "FS" => self.fs = value.as_string_convfmt(&self.convfmt),
            "CONVFMT" => self.convfmt = value.as_string(),
            "OFMT" => self.ofmt = value.as_string(),
            _ => {
                self.vars.insert(name.to_string(), value);
            }
        }
    }

    pub(crate) fn array_name(&self, name: &str) -> String {
        self.array_scopes
            .last()
            .and_then(|scope| scope.get(name))
            .cloned()
            .unwrap_or_else(|| name.to_owned())
    }
    pub(crate) fn array(&self, name: &str) -> Option<&HashMap<Vec<u8>, AwkValue>> {
        if self
            .local_scopes
            .last()
            .is_some_and(|scope| scope.contains_key(name))
        {
            return None;
        }
        self.arrays.get(&self.array_name(name))
    }
    pub(crate) fn ensure_array(&mut self, name: &str) -> Result<(), crate::runner::FlowControl> {
        if self.get_var(name) != AwkValue::Uninitialized {
            return Err(crate::runner::FlowControl::Error(format!(
                "{name} is a scalar"
            )));
        }
        self.arrays.entry(self.array_name(name)).or_default();
        Ok(())
    }
    pub(crate) fn read_array(&mut self, name: &str, key: &[u8]) -> AwkValue {
        self.arrays
            .entry(self.array_name(name))
            .or_default()
            .entry(key.to_vec())
            .or_insert(AwkValue::Uninitialized)
            .clone()
    }
    pub(crate) fn get_array_var(&self, array_name: &str, key: &[u8]) -> AwkValue {
        self.arrays
            .get(&self.array_name(array_name))
            .and_then(|arr| arr.get(key))
            .cloned()
            .unwrap_or(AwkValue::Uninitialized)
    }

    pub(crate) fn set_array_var(&mut self, array_name: &str, key: &[u8], value: AwkValue) {
        let arr = self.arrays.entry(self.array_name(array_name)).or_default();
        arr.insert(key.to_vec(), value);
    }
}

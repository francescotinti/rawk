/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: A high-fidelity port of the historic AWK language from C to Rust.
 */

use crate::ast::Statement;
use std::collections::HashMap;
use std::fmt;

use rand::rngs::StdRng;

/// Valore AWK polimorfo. Le coercioni Number↔String seguono le regole POSIX
/// (numeric context usa `as_number`, string context usa `as_string` con `CONVFMT`/`OFMT`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AwkValue {
    /// Variabile mai assegnata. Coerce a `0` in contesto numerico, `""` in contesto stringa.
    Uninitialized,
    /// Valore numerico (double-precision IEEE 754).
    Number(f64),
    /// Stringa esplicita: letterale del programma o risultato di concatenazione.
    String(String),
    /// Dual-typed: input proveniente da `getline`, `$N` o argv che parsea numericamente.
    /// La cache `(testo, valore)` evita di ri-parsare a ogni confronto numerico.
    StrNum(String, f64),
}

impl AwkValue {
    pub(crate) fn from_str_num(s: String) -> Self {
        if let Ok(n) = s.trim().parse::<f64>() {
            AwkValue::StrNum(s, n)
        } else {
            AwkValue::String(s)
        }
    }

    pub(crate) fn as_number(&self) -> f64 {
        match self {
            AwkValue::Uninitialized => 0.0,
            AwkValue::Number(n) => *n,
            AwkValue::String(s) => s.trim().parse::<f64>().unwrap_or(0.0),
            AwkValue::StrNum(_, n) => *n,
        }
    }

    pub(crate) fn as_string(&self) -> String {
        self.as_string_convfmt("%.6g")
    }

    pub(crate) fn as_string_convfmt(&self, fmt: &str) -> String {
        match self {
            AwkValue::Uninitialized => String::new(),
            AwkValue::String(s) => s.clone(),
            AwkValue::StrNum(s, _) => s.clone(),
            AwkValue::Number(n) => format_number_awk(*n, fmt),
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
            AwkValue::String(s) => s.trim().parse::<f64>().ok(),
            AwkValue::StrNum(_, n) => Some(*n),
            AwkValue::Uninitialized => Some(0.0),
        };
        let r = match other {
            AwkValue::Number(n) => Some(*n),
            AwkValue::String(s) => s.trim().parse::<f64>().ok(),
            AwkValue::StrNum(_, n) => Some(*n),
            AwkValue::Uninitialized => Some(0.0),
        };
        if let (Some(ln), Some(rn)) = (l, r) {
            Some((ln, rn))
        } else {
            None
        }
    }

    pub(crate) fn is_eq(&self, other: &Self) -> AwkValue {
        if let Some((l, r)) = self.numeric_values(other) {
            AwkValue::Number(if l == r { 1.0 } else { 0.0 })
        } else {
            AwkValue::Number(if self.as_string() == other.as_string() {
                1.0
            } else {
                0.0
            })
        }
    }

    pub(crate) fn is_lt(&self, other: &Self) -> AwkValue {
        if let Some((l, r)) = self.numeric_values(other) {
            AwkValue::Number(if l < r { 1.0 } else { 0.0 })
        } else {
            AwkValue::Number(if self.as_string() < other.as_string() {
                1.0
            } else {
                0.0
            })
        }
    }

    pub(crate) fn is_gt(&self, other: &Self) -> AwkValue {
        if let Some((l, r)) = self.numeric_values(other) {
            AwkValue::Number(if l > r { 1.0 } else { 0.0 })
        } else {
            AwkValue::Number(if self.as_string() > other.as_string() {
                1.0
            } else {
                0.0
            })
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

impl fmt::Display for AwkValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

fn format_number_awk(n: f64, fmt: &str) -> String {
    // Path A: i64 fast-path per integer-like piccoli (esistente)
    if n.is_finite() && n == n.trunc() && n.abs() < 1e16 {
        return format!("{}", n as i64);
    }
    // Path B: %.0f per integer-like grandi entro f64 precision (NUOVO)
    if n.is_finite() && n == n.trunc() && n.abs() < 1e21 {
        return sprintf::sprintf!("%.0f", n).unwrap_or_else(|_| n.to_string());
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
    File(Box<dyn std::io::BufRead>),
    Pipe {
        stdout: Box<dyn std::io::BufRead>,
        child: std::process::Child,
    },
}

impl InputStream {
    pub(crate) fn reader(&mut self) -> &mut dyn std::io::BufRead {
        match self {
            InputStream::File(r) => r.as_mut(),
            InputStream::Pipe { stdout, .. } => stdout.as_mut(),
        }
    }
}

/// Stato runtime di un programma AWK in esecuzione: record correnti, campi,
/// variabili scalari/array (con scope locali per funzioni utente), stream
/// aperti, cache regex, parametri di format (`CONVFMT`/`OFMT`) e segnali di
/// flow-control out-of-band (`nextfile_pending`, `exit_pending`).
pub(crate) struct EvalContext {
    pub(crate) nr: usize,  // Number of Records read so far
    pub(crate) fnr: usize, // Number of Records in current file
    pub(crate) nf: usize,  // Number of Fields in current record
    pub(crate) fs: String,
    pub(crate) fields: Vec<AwkValue>,
    pub(crate) record: Vec<u8>,
    pub(crate) vars: HashMap<String, AwkValue>,
    pub(crate) arrays: HashMap<String, HashMap<String, AwkValue>>,
    pub(crate) out_files: HashMap<String, OutputStream>,
    pub(crate) in_files: HashMap<String, InputStream>,
    pub(crate) rng: StdRng,
    pub(crate) local_scopes: Vec<HashMap<String, AwkValue>>,
    pub(crate) functions: HashMap<String, (Vec<String>, Vec<Statement>)>,
    pub(crate) regex_cache: HashMap<String, regex::Regex>,
    pub(crate) convfmt: String,
    pub(crate) ofmt: String,
    pub(crate) nextfile_pending: bool,
    pub(crate) exit_pending: Option<i32>,
}

impl EvalContext {
    pub(crate) fn new(fs: &str) -> Self {
        let mut vars = HashMap::new();
        vars.insert("SUBSEP".to_string(), AwkValue::String("\x1C".to_string()));
        Self {
            nr: 0,
            fnr: 0,
            nf: 0,
            fs: fs.to_string(),
            fields: Vec::new(),
            record: Vec::new(),
            vars,
            arrays: HashMap::new(),
            out_files: HashMap::new(),
            in_files: HashMap::new(),
            rng: rand::SeedableRng::seed_from_u64(0),
            local_scopes: Vec::new(),
            functions: HashMap::new(),
            regex_cache: HashMap::new(),
            convfmt: "%.6g".to_string(),
            ofmt: "%.6g".to_string(),
            nextfile_pending: false,
            exit_pending: None,
        }
    }

    pub(crate) fn compile_or_get_regex(&mut self, re: &str) -> regex::Regex {
        if let Some(r) = self.regex_cache.get(re) {
            return r.clone();
        }
        let compiled = regex::Regex::new(re).unwrap_or_else(|_| regex::Regex::new("").unwrap());
        self.regex_cache.insert(re.to_string(), compiled.clone());
        compiled
    }

    /// Update the context with a new record (line), splitting it into fields.
    pub(crate) fn update_record(&mut self, line: &[u8]) {
        self.record = line.to_vec();
        self.nr += 1;
        self.fnr += 1;

        // PHASE7.1→7.2 BRIDGE: field splitting still operates on str.
        let line_str = String::from_utf8_lossy(line);
        let line = line_str.as_ref();

        // Awk default field splitting: split by whitespace, and ignore leading/trailing whitespace
        if self.fs == " " {
            self.fields = line
                .split_whitespace()
                .map(|s| AwkValue::from_str_num(s.to_string()))
                .collect();
        } else {
            // Split by custom Field Separator
            self.fields = line
                .split(&self.fs)
                .map(|s| AwkValue::from_str_num(s.to_string()))
                .collect();
        }
        self.nf = self.fields.len();
    }

    /// Get $N. If n == 0, returns $0 (the whole record). If n > NF, returns Uninitialized.
    pub(crate) fn get_field(&self, n: usize) -> AwkValue {
        if n == 0 {
            // PHASE7.1→7.2 BRIDGE: AwkValue::String still expects String.
            AwkValue::String(String::from_utf8_lossy(&self.record).into_owned())
        } else if n <= self.nf {
            self.fields[n - 1].clone()
        } else {
            AwkValue::Uninitialized
        }
    }

    pub(crate) fn set_field(&mut self, n: usize, value: AwkValue) {
        if n == 0 {
            // PHASE7.1→7.2 BRIDGE: value.as_string() still returns String.
            self.update_record(value.as_string().as_bytes());
        } else {
            while self.fields.len() < n {
                self.fields.push(AwkValue::String(String::new()));
            }
            self.fields[n - 1] = value;
            self.nf = self.fields.len();

            // Rebuild $0 using OFS
            let ofs = self.get_var("OFS").as_string();
            let mut parts = Vec::new();
            for f in &self.fields {
                parts.push(f.as_string());
            }
            // PHASE7.1→7.2 BRIDGE: join over String, then convert to bytes.
            self.record = parts.join(&ofs).into_bytes();
        }
    }

    pub(crate) fn get_var(&self, name: &str) -> AwkValue {
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
                        self.fields.push(AwkValue::String(String::new()));
                    }
                }
                self.nf = new_nf;
                let ofs = self
                    .vars
                    .get("OFS")
                    .map(|v| v.as_string())
                    .unwrap_or_else(|| " ".to_string());
                let parts: Vec<String> = self.fields.iter().map(|f| f.as_string()).collect();
                // PHASE7.1→7.2 BRIDGE: join over String, then convert to bytes.
                self.record = parts.join(&ofs).into_bytes();
            }
            "NR" => self.nr = value.as_number() as usize,
            "FNR" => self.fnr = value.as_number() as usize,
            "FS" => self.fs = value.as_string(),
            "CONVFMT" => self.convfmt = value.as_string(),
            "OFMT" => self.ofmt = value.as_string(),
            _ => {
                self.vars.insert(name.to_string(), value);
            }
        }
    }

    pub(crate) fn get_array_var(&self, array_name: &str, key: &str) -> AwkValue {
        self.arrays
            .get(array_name)
            .and_then(|arr| arr.get(key))
            .cloned()
            .unwrap_or(AwkValue::Uninitialized)
    }

    pub(crate) fn set_array_var(&mut self, array_name: &str, key: &str, value: AwkValue) {
        let arr = self.arrays.entry(array_name.to_string()).or_default();
        arr.insert(key.to_string(), value);
    }
}

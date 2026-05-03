/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: A high-fidelity port of the historic AWK language from C to Rust.
 */

use crate::ast::Statement;
use std::collections::HashMap;
use std::fmt;
use std::io::BufRead;
use rand::rngs::StdRng;

/// Represents an AWK dynamic value (number, string, or uninitialized).
#[derive(Debug, Clone, PartialEq)]
pub enum AwkValue {
    Uninitialized,
    Number(f64),
    String(String),
    StrNum(String, f64),
}

impl AwkValue {
    pub fn from_str_num(s: String) -> Self {
        if let Ok(n) = s.trim().parse::<f64>() {
            AwkValue::StrNum(s, n)
        } else {
            AwkValue::String(s)
        }
    }

    pub fn as_number(&self) -> f64 {
        match self {
            AwkValue::Uninitialized => 0.0,
            AwkValue::Number(n) => *n,
            AwkValue::String(s) => s.trim().parse::<f64>().unwrap_or(0.0),
            AwkValue::StrNum(_, n) => *n,
        }
    }

    pub fn as_string(&self) -> String {
        self.as_string_convfmt("%.6g")
    }

    pub fn as_string_convfmt(&self, fmt: &str) -> String {
        match self {
            AwkValue::Uninitialized => String::new(),
            AwkValue::String(s) => s.clone(),
            AwkValue::StrNum(s, _) => s.clone(),
            AwkValue::Number(n) => format_number_awk(*n, fmt),
        }
    }
    
    pub fn is_truthy(&self) -> bool {
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

    pub fn is_eq(&self, other: &Self) -> AwkValue {
        if let Some((l, r)) = self.numeric_values(other) {
            AwkValue::Number(if l == r { 1.0 } else { 0.0 })
        } else {
            AwkValue::Number(if self.as_string() == other.as_string() { 1.0 } else { 0.0 })
        }
    }

    pub fn is_lt(&self, other: &Self) -> AwkValue {
        if let Some((l, r)) = self.numeric_values(other) {
            AwkValue::Number(if l < r { 1.0 } else { 0.0 })
        } else {
            AwkValue::Number(if self.as_string() < other.as_string() { 1.0 } else { 0.0 })
        }
    }
    
    pub fn is_gt(&self, other: &Self) -> AwkValue {
        if let Some((l, r)) = self.numeric_values(other) {
            AwkValue::Number(if l > r { 1.0 } else { 0.0 })
        } else {
            AwkValue::Number(if self.as_string() > other.as_string() { 1.0 } else { 0.0 })
        }
    }

    pub fn add(&self, other: &Self) -> Self {
        AwkValue::Number(self.as_number() + other.as_number())
    }

    pub fn sub(&self, other: &Self) -> Self {
        AwkValue::Number(self.as_number() - other.as_number())
    }

    pub fn mul(&self, other: &Self) -> Self {
        AwkValue::Number(self.as_number() * other.as_number())
    }

    pub fn div(&self, other: &Self) -> Self {
        let divisor = other.as_number();
        if divisor == 0.0 {
            eprintln!("awk: division by zero");
            std::process::exit(1);
        }
        AwkValue::Number(self.as_number() / divisor)
    }

    pub fn rem(&self, other: &Self) -> Self {
        let divisor = other.as_number();
        if divisor == 0.0 {
            eprintln!("awk: division by zero in mod");
            std::process::exit(1);
        }
        AwkValue::Number(self.as_number() % divisor)
    }

    pub fn pow(&self, other: &Self) -> Self {
        AwkValue::Number(self.as_number().powf(other.as_number()))
    }
}

impl fmt::Display for AwkValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

fn format_number_awk(n: f64, fmt: &str) -> String {
    if n.is_finite() && n == n.trunc() && n.abs() < 1e16 {
        format!("{}", n as i64)
    } else {
        sprintf::sprintf!(fmt, n).unwrap_or_else(|_| n.to_string())
    }
}

pub enum OutputStream {
    File(Box<dyn std::io::Write>),
    Pipe { stdin: Box<dyn std::io::Write>, child: std::process::Child },
}

impl OutputStream {
    pub fn writer(&mut self) -> &mut dyn std::io::Write {
        match self {
            OutputStream::File(w) => w.as_mut(),
            OutputStream::Pipe { stdin, .. } => stdin.as_mut(),
        }
    }
}

/// Represents the Evaluation Context (the state) for the current line.
pub struct EvalContext {
    pub nr: usize,              // Number of Records read so far
    pub fnr: usize,             // Number of Records in current file
    pub nf: usize,              // Number of Fields in current record
    pub fs: String,
    pub fields: Vec<AwkValue>,
    pub record: String,
    pub vars: HashMap<String, AwkValue>,
    pub arrays: HashMap<String, HashMap<String, AwkValue>>,
    pub out_files: HashMap<String, OutputStream>,
    pub in_files: HashMap<String, Box<dyn BufRead>>,
    pub rng: StdRng,
    pub local_scopes: Vec<HashMap<String, AwkValue>>,
    pub functions: HashMap<String, (Vec<String>, Vec<Statement>)>,
    pub regex_cache: HashMap<String, regex::Regex>,
    pub convfmt: String,
    pub ofmt: String,
    pub nextfile_pending: bool,
    pub exit_pending: Option<i32>,
}

impl EvalContext {
    pub fn new(fs: &str) -> Self {
        let mut vars = HashMap::new();
        vars.insert("SUBSEP".to_string(), AwkValue::String("\x1C".to_string()));
        Self {
            nr: 0,
            fnr: 0,
            nf: 0,
            fs: fs.to_string(),
            fields: Vec::new(),
            record: String::new(),
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

    pub fn compile_or_get_regex(&mut self, re: &str) -> regex::Regex {
        if let Some(r) = self.regex_cache.get(re) {
            return r.clone();
        }
        let compiled = regex::Regex::new(re).unwrap_or_else(|_| regex::Regex::new("").unwrap());
        self.regex_cache.insert(re.to_string(), compiled.clone());
        compiled
    }

    /// Update the context with a new record (line), splitting it into fields.
    pub fn update_record(&mut self, line: &str) {
        self.record = line.to_string();
        self.nr += 1;
        self.fnr += 1;

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
    pub fn get_field(&self, n: usize) -> AwkValue {
        if n == 0 {
            AwkValue::String(self.record.clone())
        } else if n <= self.nf {
            self.fields[n - 1].clone()
        } else {
            AwkValue::Uninitialized
        }
    }
    
    pub fn set_field(&mut self, n: usize, value: AwkValue) {
        if n == 0 {
            self.update_record(&value.as_string());
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
            self.record = parts.join(&ofs);
        }
    }
    
    pub fn get_var(&self, name: &str) -> AwkValue {
        if let Some(scope) = self.local_scopes.last() {
            if let Some(val) = scope.get(name) {
                return val.clone();
            }
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
        self.vars.get(name).cloned().unwrap_or(AwkValue::Uninitialized)
    }
    
    pub fn set_var(&mut self, name: &str, value: AwkValue) {
        if let Some(scope) = self.local_scopes.last_mut() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return;
            }
        }
        match name {
            "NF" => {
                self.nf = value.as_number() as usize;
                // Posix AWK requires $0 to be rebuilt when NF is set. We can skip that logic for now,
                // but at least we don't store it twice.
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
    
    pub fn get_array_var(&self, array_name: &str, key: &str) -> AwkValue {
        self.arrays
            .get(array_name)
            .and_then(|arr| arr.get(key))
            .cloned()
            .unwrap_or(AwkValue::Uninitialized)
    }
    
    pub fn set_array_var(&mut self, array_name: &str, key: &str, value: AwkValue) {
        let arr = self.arrays.entry(array_name.to_string()).or_insert_with(HashMap::new);
        arr.insert(key.to_string(), value);
    }
}

/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: A high-fidelity port of the historic AWK language from C to Rust.
 */

use crate::ast::Statement;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{Write, BufRead};
use rand::rngs::StdRng;
use rand::SeedableRng;

/// Represents an AWK dynamic value (number, string, or uninitialized).
#[derive(Debug, Clone, PartialEq)]
pub enum AwkValue {
    Uninitialized,
    Number(f64),
    String(String),
}

impl AwkValue {
    pub fn from_string(s: String) -> Self {
        AwkValue::String(s)
    }

    pub fn as_number(&self) -> f64 {
        match self {
            AwkValue::Uninitialized => 0.0,
            AwkValue::Number(n) => *n,
            AwkValue::String(s) => s.trim().parse::<f64>().unwrap_or(0.0),
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            AwkValue::Uninitialized => String::new(),
            AwkValue::Number(n) => n.to_string(), // In standard awk, CONVFMT is used, but for MVP we use default to_string()
            AwkValue::String(s) => s.clone(),
        }
    }
    
    pub fn is_truthy(&self) -> bool {
        match self {
            AwkValue::Uninitialized => false,
            AwkValue::Number(n) => *n != 0.0,
            AwkValue::String(s) => !s.is_empty(), // Standard awk: empty string is false, non-empty is true (even "0")
        }
    }

    // AWK comparison rules MVP: try numeric comparison first, fallback to string
    fn numeric_values(&self, other: &Self) -> Option<(f64, f64)> {
        let l = match self {
            AwkValue::Number(n) => Some(*n),
            AwkValue::String(s) => s.trim().parse::<f64>().ok(),
            AwkValue::Uninitialized => Some(0.0),
        };
        let r = match other {
            AwkValue::Number(n) => Some(*n),
            AwkValue::String(s) => s.trim().parse::<f64>().ok(),
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
}

impl fmt::Display for AwkValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_string())
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
    pub out_files: HashMap<String, Box<dyn Write>>,
    pub in_files: HashMap<String, Box<dyn BufRead>>,
    pub rng: StdRng,
    pub local_scopes: Vec<HashMap<String, AwkValue>>,
    pub functions: HashMap<String, (Vec<String>, Vec<Statement>)>,
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
        }
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
                .map(|s| AwkValue::from_string(s.to_string()))
                .collect();
        } else {
            // Split by custom Field Separator
            self.fields = line
                .split(&self.fs)
                .map(|s| AwkValue::from_string(s.to_string()))
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
    
    pub fn get_var(&self, name: &str) -> AwkValue {
        if let Some(scope) = self.local_scopes.last() {
            if let Some(val) = scope.get(name) {
                return val.clone();
            }
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
        self.vars.insert(name.to_string(), value);
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

# rawk 🦅
A blazing-fast, high-fidelity port of the historic AWK data extraction and reporting tool from C to Rust.

Built cooperatively by **Francesco Tinti** and **Antigravity (Google Deepmind)**.

## 🚀 Features
`rawk` is a fully functional interpreter that mimics POSIX AWK and parts of GNU Awk (`gawk`) while bringing modern memory safety, performance, and deterministic parsing thanks to Rust.

- **Formal Grammar Parsing**: Replaced historical Yacc/Lex combinations with modern PEG (Parsing Expression Grammars) using the `pest` crate, including a fully compliant `PrattParser` for operator precedence.
- **Dynamic Typing**: `rawk` intelligently manages numeric and string types, fully replicating AWK's famous implicit coercion capabilities.
- **Flow Control & User Functions**: Complete support for `if/else`, `while`, `do/while`, `for (in)`, `break`, `continue`, `next`, `return`, `exit`, and user-defined functions with local scoping support.
- **Extended Built-ins**:
  - Math: `sin`, `cos`, `exp`, `log`, `sqrt`, `int`, `rand`, `srand`, `atan2`
  - Bitwise (gawk extension): `and`, `or`, `xor`, `lshift`, `rshift`
  - Time (gawk extension): `systime`, `strftime`
  - Strings: `length`, `tolower`, `toupper`, `substr`, `index`, `split`, `sub`, `gsub`, `match` (updates `RSTART`/`RLENGTH`), `sprintf`
- **Advanced I/O & Pipes**: Native support for output redirects (`> file`, `>> file`), pipeline execution to bash children (`print "hello" | "cat -n"`), and extended `getline` with streaming file cache.
- **Global Magic Variables**: Built-in support for `FS`, `OFS`, `RS`, `ORS`, `NR`, `FNR`, `NF`, `SUBSEP`, `ARGC`, `ARGV`, and dynamic environment capturing in `ENVIRON`.
- **Associative Arrays**: True hash map arrays supporting multi-dimensional key simulation via `SUBSEP` and item removal (`delete`).

## 🛠 Project Architecture
- `cli.rs`: CLI argument parsing via `clap`.
- `awk.pest`: The definitive PEG grammar for the language.
- `parser.rs`: Transforms token pairs into an Abstract Syntax Tree.
- `ast.rs`: The typed AST enumerations modeling the language structures.
- `types.rs`: Holds the evaluation context, dynamic types, I/O caches, and the random number generator.
- `runner.rs`: The virtual machine executing the AST natively in Rust.

## 📦 Usage
Just like traditional AWK:
```bash
# Direct scripts
echo "foo,bar" | cargo run -- -F "," '{ print $2 }'

# Script files
cargo run -- -f my_script.awk input.txt

# Pipe outputs to system commands!
echo "1\n2\n3" | cargo run -- '{ print $0 | "cat -n" }'
```

## 📜 Authors
This code was written as part of an iterative AI pair-programming project aiming to explore limits in translating untyped, legacy C CLI utilities to deterministic Rust ecosystems.

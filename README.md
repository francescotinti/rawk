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
- `runner/`: The interpreter executing the AST natively in Rust, with separate runtime, built-in, formatting, and I/O modules.

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

## Build & Test

```bash
make -C ../c_awk                                  # build the original C reference first
cargo build --locked --release --bins
cargo test --locked                               # unit, integration and differential tests
cargo run -- -f program.awk file.txt
cargo run --bin diffrun -- tests/testsuite.xml    # comparison against ../c_awk/a.out
```

**Quality gates:**
- `cargo clippy --all-targets -- -D warnings`
- `cargo fmt --check`
- `bash scripts/checks.sh` (tutti i verification gate del piano di adeguamento idiomatico)

## 📜 Authors
This code was written as part of an iterative AI pair-programming project aiming to explore limits in translating untyped, legacy C CLI utilities to deterministic Rust ecosystems.


## 🆕 Latest Updates — September 2026

The compatibility work has consolidated the interpreter while preserving the
project's original direction and Rust implementation.

- **Runtime and compatibility:** corrected control flow, coercions, parsing,
  streaming input, regex matching, formatting and CLI behavior. CSV and safe
  mode have dedicated regressions; safe mode is not a general-purpose sandbox.
- **Unicode and locales:** UTF-8 string operations, regexes, separators and
  formatting are verified on Darwin ARM64. ISO-8859-1 and ISO-8859-9 case
  conversion and POSIX character classes now follow the selected locale.
  Binary strings retain the explicitly documented NUL-preservation extension.
- **Verification:** the latest local gate passes **132 tests in both debug and
  release**, including drivers covering **300 original UTF-8 cases**. The XML
  comparison reports 97 matches, 12 documented expected differences, and no
  unexpected differences or skipped cases.
- **Performance:** boolean UTF-8 regex searches take **12–41% less time** in the
  three measured workloads. Byte-profile workloads varied by +1–3%; these
  measurements are not a general speed guarantee relative to the C interpreter.
- **Portability:** CI is configured for macOS and Linux glibc x86-64/ARM64.
  Local target checks passed; native Linux compatibility is not yet established.
- **Next steps:** Shift-JIS and other legacy encodings, native Linux validation,
  and further optimizations supported by profiling.

The verified profiles and deliberate differences define the current compatibility
scope; the tests do not establish complete POSIX or GNU Awk conformance. The CLI
currently requires UTF-8 file names. Tests require Python 3, a C compiler, the
adjacent original `c_awk` source tree, and the locales exercised by the suites.

See the [compatibility details](docs/COMPATIBILITY.md),
[phase 7 report](diary/2026-09-19-phase7-closure.md),
[Unicode report](diary/2026-09-19-unicode-locale.md),
[legacy locale report](diary/2026-09-20-legacy-locales.md),
[performance measurements](diary/2026-09-20-regex-performance.md),
and [work plan](audit-2026-09-19/PIANO_DI_LAVORO.md).

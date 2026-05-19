# rawk — Piano di adeguamento idiomatico Rust (TDD strict)

> **Per l'agent che esegue:** SUB-SKILL RICHIESTA: `superpowers:subagent-driven-development` (consigliata) o `superpowers:executing-plans`. Step in checkbox (`- [ ]`). **Disciplina TDD strict**: ogni unità di lavoro segue il ciclo Red → Green → Refactor → Commit. Anche i cleanup "meccanici" usano *verification test* eseguibili (shell one-liner che asserisce lo stato desiderato) come gate Red prima e Green dopo.

---

## Stato di avanzamento (aggiornare a fine di ogni sessione)

| Fase | Stato | Commit di chiusura | Note |
|------|-------|--------------------|------|
| Phase 0 — Workspace hygiene | ✅ FATTO | 3e3035d / 83fd5a8 / 55ff6c4 | gates: macOS forks, scratch root, fmt — tutti verdi |
| Phase 1 — Clippy zero-warning | ✅ FATTO | 65432a8 / f9650fb / 90e2f8f / e319009 / 9672924 | `[lints.clippy] all=deny`, 16 lint chiusi (10 collapsible_if, 4 expect_fun_call, 1 redundant_pattern_matching, 1 or_insert_with) |
| Phase 2 — Runtime exit-free | ✅ FATTO | ab597e5 / 4eaff8f | 10 `process::exit()` → main.rs solo; `run() -> Result<i32>` |
| Phase 3 — Error handling | ⏳ TODO | — | `with_context()` + expect documentati |
| Phase 4a — `runner.rs` → dir module | ⏳ TODO | — | preparatorio |
| Phase 4b — Estrai `runner/builtins.rs` | ⏳ TODO | — | ~400 LOC |
| Phase 4c — Estrai `runner/io.rs` | ⏳ TODO | — | ~200 LOC |
| Phase 4d — Estrai `runner/fmt.rs` | ⏳ TODO | — | printf engine |
| Phase 5 — Fix proptest_diff | ⏳ TODO | — | `env!("CARGO_BIN_EXE_rawk")` |
| Phase 6 — Docs & visibilità | ⏳ TODO | — | `pub(crate)`, `///`, README, Step 19 in diary |

Legenda stato: ⏳ TODO · 🚧 IN CORSO · ✅ FATTO · ⚠️ PARZIALE · ❌ BLOCCATO

**Modalità di lavoro concordata:** una fase per sessione (granularità grossa). L'utente apre una sessione, l'agent fa la fase indicata (es. *"fai Phase 0"* o *"continua dalla prossima TODO"*), aggiorna la tabella sopra con commit-SHA + stato, fine sessione.

**Resume rapido (cosa fare a inizio sessione su `rawk`):**

```bash
cd "/Volumes/Extreme Pro/Claude/testag-awk/rawk"
git log --oneline -5            # vedi gli ultimi commit (Step 19 progress)
head -40 STEP19_PLAN.md         # leggi la tabella di stato
# poi vai alla prima riga ⏳ TODO o 🚧 IN CORSO ed esegui la fase
```

**Gate invariante globale** (da rispettare a OGNI step di OGNI fase):
```bash
cargo test --test xml_runner_test 2>&1 | grep -E 'test result' | head -1
# DEVE riportare: test result: ok. 109 passed; 0 failed
```

---

**Goal:** Portare `rawk` (porting AWK→Rust, ~2500 LOC, edition 2024, chiuso a Step 18) a piena conformità idiomatica Rust: zero warning clippy, runtime exit-free, error-handling con contesto, runner.rs scomposto in moduli, dev-loop pulito.

**Architettura:** Sei fasi sequenziali. Ogni fase ha un *gate invariante* — i 109 testcase XML restano verdi tra ogni Red→Green cycle. Nessuna modifica semantica di AWK.

**Tech Stack:** Rust 2024 · anyhow · clap · pest · regex · sprintf · quick-xml · serde · proptest

---

## Context

Audit indipendente su tre direttrici (struttura, errori, build/lint) ha trovato **5 problemi HIGH**, **6 MEDIUM**, **2 LOW**. Il progetto compila pulito ma `cargo clippy --all-targets -- -D warnings` fallisce su 13 lint; `cargo test` ha 6 fallimenti di infrastruttura (proptest_diff non trova il binario); l'invariante dichiarata "Step 15: runtime exit-free" è **disattesa** — 10 `process::exit()` ancora presenti in `src/runner.rs`. Il file `src/runner.rs` (1097 LOC) concentra orchestrazione, eval, 30+ builtin e formattazione printf. `eval_expr()` è 510 LOC con il ramo `FunctionCall` da 300 LOC. Repository inquinato da ~100 fork macOS (`._*`) e artefatti di scratch (450 KB) committati.

Obiettivo: l'esperimento rimane formalmente chiuso, ma riallinearlo agli standard idiomatici del linguaggio rende la base codice un esempio di porting credibile e mantenibile, in coerenza col diario metodologico già pubblicato.

---

## TDD-strict — pattern applicato

Ogni step segue uno di questi due cicli:

**Cycle A (comportamento testabile)**: scrivi/abilita un test che fallisce → fai la modifica minima → test passa → commit.

**Cycle B (invariante strutturale "meccanico")**: scrivi un *verification test* — uno script shell o un check `cargo` che asserisce lo stato finale desiderato. Eseguilo per vedere il fallimento (Red). Fai la modifica. Eseguilo di nuovo (Green). Commit. Lo script resta documentato nel commit message o, dove ha valore continuativo, va in `scripts/checks.sh` (creato in Step 0.0).

**Gate invariante globale (fra OGNI cycle)**:
```bash
cargo test --test xml_runner_test 2>&1 | grep -E 'test result' | head -1
# DEVE riportare: test result: ok. 109 passed; 0 failed
```
Se qualunque step rompe i 109, ferma e rollback.

---

## File map

| Path | Ruolo | Modifica prevista |
|------|-------|-------------------|
| `scripts/checks.sh` (nuovo) | Script che esegue tutti i verification test | CREATE |
| `src/runner.rs` (1097) | God-file: orchestrazione + eval + builtin + I/O + printf | Splittato in `src/runner/{mod,builtins,io,fmt}.rs` |
| `src/runner/builtins.rs` (nuovo) | Tutte le funzioni built-in AWK | CREATE |
| `src/runner/io.rs` (nuovo) | Apertura/chiusura file, pipe, getline, close(), fflush() | CREATE |
| `src/runner/fmt.rs` (nuovo) | Formattazione printf/sprintf | CREATE |
| `src/main.rs` (26) | Entry point | Unico sito di `process::exit()`; debug `println!` → `eprintln!` |
| `src/parser.rs` (600) | Parser pest | `.unwrap()` → `.expect("pest: …")` mirati |
| `src/types.rs` (369) | AwkValue, contesti, stream | Visibilità → `pub(crate)`; doc `///` sui tipi pubblici |
| `src/ast.rs` (93) | Enum AST | Doc `///` sugli enum |
| `src/bin/diffrun.rs` (202) | Test harness differenziale | Fix clippy (collapsible_if, expect_fun_call) |
| `tests/proptest_diff.rs` (97) | Property test | Fix path binario (`env!("CARGO_BIN_EXE_rawk")`) |
| `Cargo.toml` | Manifest | `[lints.clippy]` con `all = deny` |
| `.gitignore` | | Esclude `._*`, `scratch*`, `*.rs` a root non in `src/` |
| `README.md` | | Sezione `Build & Test` |
| `NEXT_STEPS.md` | Diary delle fasi | Append fase "Step 19: Idiomatic Cleanup" con esiti |

---

## Phase 0 — Igiene workspace

### Task 0.0: Setup script verification gate

**Files:**
- Create: `scripts/checks.sh`

- [ ] **Step 0.0.1 (Red)**: verifica che lo script ancora non esista

```bash
cd "/Volumes/Extreme Pro/Claude/testag-awk/rawk"
test ! -f scripts/checks.sh && echo "RED-OK: script assente"
```

- [ ] **Step 0.0.2 (Green)**: crea `scripts/checks.sh`

```bash
mkdir -p scripts
```

Contenuto di `scripts/checks.sh`:
```bash
#!/usr/bin/env bash
# Verification gates per il piano di adeguamento Rust idiomatico.
# Ogni funzione ritorna 0 se invariante OK, 1 altrimenti.
set -eu
cd "$(dirname "$0")/.."

check_no_macos_forks() {
  ! find . -name '._*' -not -path './target/*' -not -path './.git/*' | grep -q . \
    || { echo "FAIL: macOS forks presenti"; return 1; }
}
check_no_scratch_root() {
  for f in scratch scratch.rs parse_test.rs pest_test.rs debug.rs f1.txt f2.txt out.txt; do
    [ ! -e "$f" ] || { echo "FAIL: artefatto root '$f'"; return 1; }
  done
}
check_fmt() { cargo fmt --check >/dev/null; }
check_clippy() { cargo clippy --all-targets -- -D warnings >/dev/null 2>&1; }
check_tests() { cargo test --test xml_runner_test 2>&1 | grep -q '109 passed'; }
check_no_exit_outside_main() {
  found=$(grep -rn 'std::process::exit\|process::exit' src/ \
    | grep -v 'src/main.rs' | wc -l | tr -d ' ')
  [ "$found" = "0" ] || { echo "FAIL: $found process::exit() fuori main.rs"; return 1; }
}
check_runner_split() {
  [ -f src/runner/mod.rs ] && [ -f src/runner/builtins.rs ] \
    && [ -f src/runner/io.rs ] && [ -f src/runner/fmt.rs ] \
    || { echo "FAIL: runner non splittato"; return 1; }
}

run_all() { for fn in $(declare -F | awk '$3 ~ /^check_/ {print $3}'); do
  printf '%-30s ' "$fn"; $fn && echo OK; done; }

"${1:-run_all}"
```

Rendilo eseguibile:
```bash
chmod +x scripts/checks.sh
```

- [ ] **Step 0.0.3 (Verify)**:
```bash
bash scripts/checks.sh check_tests && echo "baseline test verde"
```
Atteso: stampa "OK" (109 pass è la baseline).

- [ ] **Step 0.0.4 (Commit)**:
```bash
git add scripts/checks.sh && git commit -m "chore: scripts/checks.sh — verification gates per piano idiomatico"
```

### Task 0.1: Repository pulito da fork macOS

- [ ] **Step 0.1.1 (Red)**:
```bash
bash scripts/checks.sh check_no_macos_forks
```
Atteso: `FAIL: macOS forks presenti` (~100 file `._*`).

- [ ] **Step 0.1.2 (Green)**: rimuovi i fork
```bash
find . -name '._*' -not -path './target/*' -not -path './.git/*' -delete
```

- [ ] **Step 0.1.3 (Verify)**:
```bash
bash scripts/checks.sh check_no_macos_forks   # → OK
bash scripts/checks.sh check_tests             # → OK (109)
```

- [ ] **Step 0.1.4 (Commit)**:
```bash
git add -A && git commit -m "chore: rimuovi fork macOS (._*)"
```

### Task 0.2: .gitignore previene reintroduzione

**Files:**
- Modify: `.gitignore`

- [ ] **Step 0.2.1 (Red)**: regression test — ricrea un fork fittizio e verifica che git lo veda

```bash
touch ._test_fork
git status --porcelain | grep -q '._test_fork' \
  && echo "RED-OK: gitignore non protegge"
rm ._test_fork
```

- [ ] **Step 0.2.2 (Green)**: appendi al `.gitignore`

```
# macOS resource forks
._*
.DS_Store

# Scratch / experiments at root
/scratch
/scratch.rs
/parse_test.rs
/pest_test.rs
/debug.rs
/f1.txt
/f2.txt
/out.txt
```

- [ ] **Step 0.2.3 (Verify)**:
```bash
touch ._test_fork
git status --porcelain | grep -q '._test_fork' && echo FAIL || echo OK
rm ._test_fork
```

- [ ] **Step 0.2.4 (Commit)**:
```bash
git add .gitignore && git commit -m "chore(gitignore): previeni reintroduzione macOS forks + scratch"
```

### Task 0.3: Artefatti scratch a root eliminati

- [ ] **Step 0.3.1 (Red)**: `bash scripts/checks.sh check_no_scratch_root` → atteso FAIL su 7 file.

- [ ] **Step 0.3.2 (Green)**:
```bash
rm -f parse_test.rs pest_test.rs scratch.rs debug.rs f1.txt f2.txt out.txt
rm -rf scratch
```

- [ ] **Step 0.3.3 (Verify)**:
```bash
bash scripts/checks.sh check_no_scratch_root && bash scripts/checks.sh check_tests
```

- [ ] **Step 0.3.4 (Commit)**:
```bash
git add -A && git commit -m "chore: rimuovi artefatti scratch a root (parse_test.rs, scratch/, ecc.)"
```

### Task 0.4: cargo fmt allineato

- [ ] **Step 0.4.1 (Red)**: `bash scripts/checks.sh check_fmt` → atteso FAIL (30+ diff).

- [ ] **Step 0.4.2 (Green)**: `cargo fmt --all`

- [ ] **Step 0.4.3 (Verify)**:
```bash
bash scripts/checks.sh check_fmt && bash scripts/checks.sh check_tests
```

- [ ] **Step 0.4.4 (Commit)**:
```bash
git add -A && git commit -m "style: cargo fmt --all"
```

---

## Phase 1 — Zero clippy warning

### Task 1.0: Lint attivati nel manifest

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1.0.1 (Red)**:
```bash
grep -q '\[lints.clippy\]' Cargo.toml && echo present || echo "RED-OK: lints assenti"
```

- [ ] **Step 1.0.2 (Green)**: appendi a `Cargo.toml`:
```toml

[lints.clippy]
all = { level = "deny", priority = -1 }
```

- [ ] **Step 1.0.3 (Verify)**:
```bash
cargo clippy --all-targets 2>&1 | tee /tmp/rawk-clippy.txt | grep -E 'error|warning' | wc -l
```
Atteso: 13 errori (li elencheremo sotto). Test ancora verdi:
```bash
bash scripts/checks.sh check_tests
```

- [ ] **Step 1.0.4 (Commit)**:
```bash
git add Cargo.toml && git commit -m "build: [lints.clippy] all=deny (priority -1)"
```

### Task 1.1: Fix collapsible_if con if-let-chains 2024

**Files:**
- Modify: `src/runner.rs:118`
- Modify: `src/runner.rs:401`
- Modify: `src/runner.rs:407`
- Modify: `src/runner.rs:412-413`
- Modify: `src/runner.rs:939`
- Modify: `src/bin/diffrun.rs:59`

Pattern da applicare a ciascuno:

Prima (nested):
```rust
if let Some(stream) = streams.get_mut(&key) {
    if let Ok(n) = stream.read_line(&mut buf) {
        if n > 0 { /* … */ }
    }
}
```

Dopo (Rust 2024 if-let-chains, già stabili in edition 2024):
```rust
if let Some(stream) = streams.get_mut(&key)
    && let Ok(n) = stream.read_line(&mut buf)
    && n > 0
{
    /* … */
}
```

- [ ] **Step 1.1.1 (Red)**:
```bash
cargo clippy --all-targets 2>&1 | grep -c 'collapsible_if'
```
Atteso: ≥6.

- [ ] **Step 1.1.2 (Green)**: applica il pattern alle 6 righe elencate sopra (una alla volta per sicurezza, lanciando `cargo test` dopo ciascuna).

- [ ] **Step 1.1.3 (Verify)**:
```bash
cargo clippy --all-targets 2>&1 | grep -c 'collapsible_if'   # 0
bash scripts/checks.sh check_tests                            # 109
```

- [ ] **Step 1.1.4 (Commit)**:
```bash
git add -A && git commit -m "refactor: collapsible_if → if-let-chains (Rust 2024)"
```

### Task 1.2: Fix expect_fun_call in diffrun.rs

**Files:**
- Modify: `src/bin/diffrun.rs:134`
- Modify: `src/bin/diffrun.rs:136`

Sostituire `.expect(&format!("…{}…", x))` con `.unwrap_or_else(|_| panic!("…{}…", x))`. (Motivo: `expect` valuta sempre l'argomento; `unwrap_or_else` solo in caso di errore.)

- [ ] **Step 1.2.1 (Red)**: `cargo clippy --all-targets 2>&1 | grep -c 'expect_fun_call'` → 2.

- [ ] **Step 1.2.2 (Green)**: edita le due righe.

- [ ] **Step 1.2.3 (Verify)**: `cargo clippy --all-targets 2>&1 | grep -c 'expect_fun_call'` → 0; `bash scripts/checks.sh check_tests`.

- [ ] **Step 1.2.4 (Commit)**: `git add -A && git commit -m "perf: diffrun expect → unwrap_or_else (clippy::expect_fun_call)"`

### Task 1.3: Fix redundant_pattern_matching in parser.rs:348

- [ ] **Step 1.3.1 (Red)**: `cargo clippy --all-targets 2>&1 | grep redundant_pattern_matching` → 1 hit.

- [ ] **Step 1.3.2 (Green)**: `if let Some(_) = x` → `if x.is_some()` (riga 348).

- [ ] **Step 1.3.3 (Verify)**: clippy clean su quel lint + test 109.

- [ ] **Step 1.3.4 (Commit)**: `git commit -am "refactor(parser): is_some() invece di if let Some(_)"`

### Task 1.4: Fix or_insert_with in types.rs:366

- [ ] **Step 1.4.1 (Red)**: `cargo clippy --all-targets 2>&1 | grep or_insert_with` → 1.

- [ ] **Step 1.4.2 (Green)**: `.or_insert(Vec::new())` → `.or_default()`.

- [ ] **Step 1.4.3 (Verify)**: clippy clean + test 109.

- [ ] **Step 1.4.4 (Commit)**: `git commit -am "refactor(types): or_default() per HashMap entry"`

### Task 1.5: Gate clippy zero

- [ ] **Step 1.5.1 (Verify finale)**:
```bash
bash scripts/checks.sh check_clippy && bash scripts/checks.sh check_tests
```
Entrambi DEVONO stampare OK. Se clippy ancora segnala altri lint emersi (es. consequenza di edit precedenti), affrontali con lo stesso pattern Red→Green→Commit.

---

## Phase 2 — Runtime exit-free

### Task 2.0: Verification test per invariante

- [ ] **Step 2.0.1 (Red)**: `bash scripts/checks.sh check_no_exit_outside_main`
Atteso: `FAIL: 10 process::exit() fuori main.rs`.

### Task 2.1: Test integrazione che pin-na il comportamento di `exit N`

**Files:**
- Create: `tests/exit_codes.rs`

- [ ] **Step 2.1.1 (Red)**: scrivi test integrazione

`tests/exit_codes.rs`:
```rust
use std::process::Command;

const RAWK: &str = env!("CARGO_BIN_EXE_rawk");

#[test]
fn exit_zero_default() {
    let out = Command::new(RAWK).args(["BEGIN{exit}"]).status().unwrap();
    assert_eq!(out.code(), Some(0), "exit senza arg → 0");
}

#[test]
fn exit_with_value() {
    let out = Command::new(RAWK).args(["BEGIN{exit 7}"]).status().unwrap();
    assert_eq!(out.code(), Some(7), "exit 7 → 7");
}

#[test]
fn exit_from_end_block() {
    let out = Command::new(RAWK)
        .args(["BEGIN{} END{exit 3}"])
        .status().unwrap();
    assert_eq!(out.code(), Some(3), "exit in END → 3");
}

#[test]
fn syntax_error_nonzero() {
    let out = Command::new(RAWK).args(["BEGIN{(("]).output().unwrap();
    assert_ne!(out.status.code(), Some(0), "errore sintassi → non-zero");
}
```

- [ ] **Step 2.1.2 (Verify Red)**:
```bash
cargo test --test exit_codes 2>&1 | tail -10
```
I primi 3 test devono GIÀ passare oggi (semantica corretta tramite `process::exit` interni). Il 4° serve a documentare. Se passano tutti, **questo è il test di regressione che useremo per Phase 2** — la semantica deve restare invariata anche dopo refactor.

- [ ] **Step 2.1.3 (Commit)**:
```bash
git add tests/exit_codes.rs && git commit -m "test: integrazione exit-codes (regressione per Phase 2)"
```

### Task 2.2: Funnel process::exit verso main.rs

**Files:**
- Modify: `src/runner.rs` (linee 105, 112, 131, 136, 139, 145, 151, 157, 161, 170)
- Modify: `src/main.rs`

- [ ] **Step 2.2.1 (Red)**: `bash scripts/checks.sh check_no_exit_outside_main` → FAIL (10).

- [ ] **Step 2.2.2 (Green)**: refactor in due passi.

(a) Cambia firma di `run()` se non già `Result<i32>`: deve restituire il codice di exit.
Esempio shape attesa:
```rust
pub fn run(cli: Cli) -> anyhow::Result<i32> {
    // ... logica esistente, ma sostituisci ogni process::exit(c) con `return Ok(c);`
}
```
Per `FlowControl::Exit(c)` che bubbla fuori da `process_lines`/`execute_action`, intercettalo a livello di `run()` e mappalo nel valore di ritorno.

(b) Aggiorna `src/main.rs` a essere l'unico chiamante di `std::process::exit`:
```rust
fn main() {
    let cli = Cli::parse();
    let code = match run(cli) {
        Ok(c) => c,
        Err(e) => { eprintln!("rawk: {e:#}"); 2 }
    };
    std::process::exit(code);
}
```
Rimuovi/promuovi a `eprintln!` qualsiasi `println!` di debug rimasto in `main.rs:19-20`.

- [ ] **Step 2.2.3 (Verify)**:
```bash
bash scripts/checks.sh check_no_exit_outside_main   # OK
cargo test --test exit_codes                         # 4/4 (semantica preservata)
bash scripts/checks.sh check_tests                   # 109
```

- [ ] **Step 2.2.4 (Commit)**:
```bash
git add -A && git commit -m "refactor: runtime exit-free — process::exit confluisce in main()"
```

---

## Phase 3 — Hardening error handling

### Task 3.1: Test che asseriscono il messaggio di errore I/O

**Files:**
- Create: `tests/error_messages.rs`

- [ ] **Step 3.1.1 (Red)**:
```rust
// tests/error_messages.rs
use std::process::Command;
const RAWK: &str = env!("CARGO_BIN_EXE_rawk");

#[test]
fn missing_program_file_mentions_filename() {
    let out = Command::new(RAWK)
        .args(["-f", "/nonexistent/path/foo.awk"])
        .output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("/nonexistent/path/foo.awk"),
        "stderr deve nominare il file mancante; got: {stderr}");
    assert!(stderr.contains("programfile") || stderr.contains("lettura"),
        "stderr deve descrivere l'operazione; got: {stderr}");
}

#[test]
fn output_to_unwritable_path_mentions_filename() {
    // Tentiamo di scrivere su /proc o path inesistente come append
    let out = Command::new(RAWK)
        .args(["BEGIN { print \"x\" >> \"/nonexistent/dir/out\" }"])
        .output().unwrap();
    assert_ne!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("/nonexistent/dir/out"),
        "stderr deve nominare il path output; got: {stderr}");
}
```

- [ ] **Step 3.1.2 (Verify Red)**:
```bash
cargo test --test error_messages 2>&1 | tail -10
```
Atteso: FAIL (bare `?` non aggiunge il filename al messaggio).

- [ ] **Step 3.1.3 (Commit test)**:
```bash
git add tests/error_messages.rs && git commit -m "test: messaggi di errore I/O devono includere filename"
```

### Task 3.2: Aggiungi `.context()` agli I/O utente

**Files:**
- Modify: `src/runner.rs:1024` (file open append)
- Modify: `src/runner.rs:1036` (file open write)
- Modify: `src/runner.rs:67` (read programfile)

- [ ] **Step 3.2.1 (Green)**: ogni I/O con path noto:

`runner.rs:1024`:
```rust
let f = OpenOptions::new().append(true).create(true).open(&filename)
    .with_context(|| format!("apertura file output '{filename}' in append"))?;
```

`runner.rs:1036`:
```rust
let f = OpenOptions::new().write(true).truncate(true).create(true).open(&filename)
    .with_context(|| format!("apertura file output '{filename}' in scrittura"))?;
```

`runner.rs:67`:
```rust
std::fs::read_to_string(pf)
    .with_context(|| format!("lettura programfile '{pf}'"))?;
```
(Aggiungi `use anyhow::Context;` in cima al file se non già presente.)

- [ ] **Step 3.2.2 (Verify Green)**:
```bash
cargo test --test error_messages 2>&1 | tail -10   # 2/2 pass
bash scripts/checks.sh check_tests                  # 109
bash scripts/checks.sh check_clippy
```

- [ ] **Step 3.2.3 (Commit)**:
```bash
git add -A && git commit -m "fix: with_context() su file open / programfile read"
```

### Task 3.3: Documenta gli unwrap del parser

**Files:**
- Modify: `src/parser.rs` (76 `.unwrap()`)

- [ ] **Step 3.3.1 (Red — verification test custom)**:
```bash
test "$(grep -c '\.unwrap()' src/parser.rs)" -eq 0 && echo OK || echo "FAIL: $(grep -c '\.unwrap()' src/parser.rs) unwrap restanti"
```
Atteso: FAIL con conteggio iniziale.

- [ ] **Step 3.3.2 (Green)**: per ogni `.unwrap()` in `src/parser.rs`, sostituisci con `.expect("pest: <regola> garantisce <invariante>")`. Lavora a blocchi (es. 10–15 per commit per non perdere il filo) e dopo OGNI blocco esegui `bash scripts/checks.sh check_tests`.

Esempio:
```rust
// prima:
let inner = pair.into_inner().next().unwrap();
// dopo:
let inner = pair.into_inner().next()
    .expect("pest: regola Statement ha sempre almeno un figlio");
```

- [ ] **Step 3.3.3 (Verify)**:
```bash
grep -c '\.unwrap()' src/parser.rs   # 0
bash scripts/checks.sh check_tests   # 109
```

- [ ] **Step 3.3.4 (Commit)**:
```bash
git add -A && git commit -m "docs(parser): unwrap → expect con invariante pest dichiarata"
```

---

## Phase 4 — Scomporre runner.rs

Strategia: rinomina file in modulo directory, poi estrai *uno alla volta* builtin, I/O, fmt. Ogni estrazione è un Red→Green→Commit basato sul gate `check_tests` (109/109 deve restare verde). I 109 testcase XML SONO il test di regressione.

### Task 4.1: runner.rs → runner/mod.rs (preparatorio)

- [ ] **Step 4.1.1 (Red)**: `bash scripts/checks.sh check_runner_split` → FAIL.

- [ ] **Step 4.1.2 (Green)**:
```bash
mkdir -p src/runner
git mv src/runner.rs src/runner/mod.rs
```

- [ ] **Step 4.1.3 (Verify)**:
```bash
cargo build && bash scripts/checks.sh check_tests
```

- [ ] **Step 4.1.4 (Commit)**: `git commit -m "refactor: runner.rs → runner/mod.rs"`

### Task 4.2: Estrai `runner::builtins`

**Files:**
- Create: `src/runner/builtins.rs`
- Modify: `src/runner/mod.rs`

API attesa esposta da builtins.rs:
```rust
pub(super) fn dispatch_builtin(
    name: &str,
    args: &[Expr],
    ctx: &mut EvalContext,
    eval: &mut dyn FnMut(&Expr, &mut EvalContext) -> Result<AwkValue>,
) -> Result<Option<AwkValue>>;
```
Ritorna `None` se `name` non è un builtin (callsite proverà user-defined).

- [ ] **Step 4.2.1 (Red, sub-test mirato)**: aggiungi al volo un test di sanity che esercita 5 builtin chiave per pin-nare comportamento:

In `tests/builtins_sanity.rs` (NUOVO):
```rust
use std::process::Command;
const RAWK: &str = env!("CARGO_BIN_EXE_rawk");

fn run(prog: &str) -> String {
    let out = Command::new(RAWK).args([prog]).output().unwrap();
    String::from_utf8(out.stdout).unwrap()
}

#[test] fn length_string() { assert_eq!(run("BEGIN{print length(\"hello\")}").trim(), "5"); }
#[test] fn substr_basic()  { assert_eq!(run("BEGIN{print substr(\"hello\",2,3)}").trim(), "ell"); }
#[test] fn split_basic()   { assert_eq!(run("BEGIN{n=split(\"a:b:c\",a,\":\"); print n,a[1],a[3]}").trim(), "3 a c"); }
#[test] fn sprintf_int()   { assert_eq!(run("BEGIN{print sprintf(\"%05d\",42)}").trim(), "00042"); }
#[test] fn match_basic()   { assert_eq!(run("BEGIN{print match(\"hello\",/ll/),RSTART,RLENGTH}").trim(), "3 3 2"); }
```

- [ ] **Step 4.2.2 (Verify baseline)**: `cargo test --test builtins_sanity` → 5/5 pass (servirà come regression check durante l'estrazione).

- [ ] **Step 4.2.3 (Green)**: crea `src/runner/builtins.rs`, sposta l'intera dispatch del ramo `Expr::FunctionCall` (righe ~459–724 nell'originale). In `src/runner/mod.rs` dichiarare `mod builtins;` e nella eval_expr() chiamare `builtins::dispatch_builtin(...)`. Mantieni cambiamento minimo: NO refactor delle singole builtin in questo step.

- [ ] **Step 4.2.4 (Verify)**:
```bash
cargo build
bash scripts/checks.sh check_tests       # 109
cargo test --test builtins_sanity         # 5/5
bash scripts/checks.sh check_clippy
```

- [ ] **Step 4.2.5 (Commit)**:
```bash
git add -A && git commit -m "refactor: estrai runner/builtins.rs (~400 LOC, semantica invariata)"
```

### Task 4.3: Estrai `runner::io`

**Files:**
- Create: `src/runner/io.rs`
- Modify: `src/runner/mod.rs`

Sposta apertura/chiusura file e pipe, `getline` da file/comando, `close()`, `fflush()`. Tipicamente le strutture `InputStream`/`OutputStream` di `types.rs` collaborano qui (lasciale dove sono, sposta solo la logica di gestione).

- [ ] **Step 4.3.1 (Red verifica test gate)**: `bash scripts/checks.sh check_tests` deve essere già verde (109).

- [ ] **Step 4.3.2 (Green)**: estrai il codice. Aggiungi `mod io;` in `mod.rs`. La superficie pubblica del modulo deve essere minima (es. `open_for_append`, `open_for_write`, `pipe_to_command`, `close_stream`, `flush_stream`).

- [ ] **Step 4.3.3 (Verify)**:
```bash
cargo build
bash scripts/checks.sh check_tests             # 109
cargo test --test exit_codes                    # 4/4
cargo test --test error_messages                # 2/2
bash scripts/checks.sh check_clippy
```

- [ ] **Step 4.3.4 (Commit)**: `git commit -am "refactor: estrai runner/io.rs (stream lifecycle)"`

### Task 4.4: Estrai `runner::fmt` (printf engine)

**Files:**
- Create: `src/runner/fmt.rs`
- Modify: `src/runner/mod.rs`

Sposta `format_awk()` e helper `format_number_awk` (Step 16). La superficie pubblica è una sola funzione `pub(super) fn format_awk(fmt: &str, args: &[AwkValue]) -> Result<String>`.

- [ ] **Step 4.4.1 (Red sub-test mirato)**:

In `tests/printf_sanity.rs` (NUOVO):
```rust
use std::process::Command;
const RAWK: &str = env!("CARGO_BIN_EXE_rawk");
fn run(p: &str) -> String { String::from_utf8(Command::new(RAWK).args([p]).output().unwrap().stdout).unwrap() }

#[test] fn d_padding()  { assert_eq!(run(r#"BEGIN{printf "%05d",42}"#), "00042"); }
#[test] fn f_precision(){ assert_eq!(run(r#"BEGIN{printf "%.2f",3.14159}"#), "3.14"); }
#[test] fn s_truncate() { assert_eq!(run(r#"BEGIN{printf "%.3s","hello"}"#), "hel"); }
#[test] fn x_hex()      { assert_eq!(run(r#"BEGIN{printf "%x",255}"#), "ff"); }
#[test] fn e_scientific(){ let s = run(r#"BEGIN{printf "%e",1234.5}"#); assert!(s.starts_with("1.234500e"), "got {s}"); }
```

- [ ] **Step 4.4.2 (Verify baseline)**: 5/5 pass.

- [ ] **Step 4.4.3 (Green)**: estrai.

- [ ] **Step 4.4.4 (Verify)**:
```bash
cargo build
bash scripts/checks.sh check_tests
cargo test --test printf_sanity
bash scripts/checks.sh check_clippy
```

- [ ] **Step 4.4.5 (Commit)**: `git commit -am "refactor: estrai runner/fmt.rs (printf engine)"`

### Task 4.5: Sanity finale Phase 4

- [ ] **Step 4.5.1 (Verify)**:
```bash
bash scripts/checks.sh check_runner_split   # OK
wc -l src/runner/*.rs                        # mod.rs ≤ ~350 desiderato
bash scripts/checks.sh check_clippy
bash scripts/checks.sh check_tests
cargo test                                   # tutti i suite (xml, exit, error, builtins, printf, proptest TBD)
```

---

## Phase 5 — Fix infrastruttura proptest

**Files:**
- Modify: `tests/proptest_diff.rs:14` (path errato al binario)

### Task 5.1: Diagnosi e fix

- [ ] **Step 5.1.1 (Red)**:
```bash
cargo test --test proptest_diff 2>&1 | tail -10
```
Atteso: 6 fail con "No such file or directory" (path hard-coded).

- [ ] **Step 5.1.2 (Green)**: in `tests/proptest_diff.rs` linea 14 sostituisci il path hard-coded con:
```rust
const RAWK_BIN: &str = env!("CARGO_BIN_EXE_rawk");
```
e usa `Command::new(RAWK_BIN)`. (Pattern già usato da `tests/xml_runner_test.rs:75-92` e dai test creati in Phase 2-4.)

- [ ] **Step 5.1.3 (Verify)**:
```bash
cargo test --test proptest_diff 2>&1 | tail -10   # tutti pass
cargo test                                          # tutti i suite verdi
```

- [ ] **Step 5.1.4 (Commit)**:
```bash
git add -A && git commit -m "fix(tests): proptest_diff usa env!(CARGO_BIN_EXE_rawk)"
```

---

## Phase 6 — Documentazione & visibilità

### Task 6.1: Visibilità `pub` → `pub(crate)` in types.rs

**Files:**
- Modify: `src/types.rs`

Strategia: cambia `pub` → `pub(crate)` su tutti gli item che NON sono usati da `src/bin/diffrun.rs`. Per ogni cambio: `cargo check` per individuare il primo errore di visibilità; ripristina selettivamente quel singolo item se serve esposto al binario `diffrun`.

- [ ] **Step 6.1.1 (Red verification)**:
```bash
test "$(grep -c '^pub ' src/types.rs)" -gt 0 && echo "pub-count=$(grep -c '^pub ' src/types.rs)"
```
Annota il numero di partenza (atteso ~25).

- [ ] **Step 6.1.2 (Green)**: itera. Sostituisci `pub fn` con `pub(crate) fn`, `pub struct` con `pub(crate) struct`, `pub enum` con `pub(crate) enum` salvo che `cargo check` riporti accesso da `src/bin/diffrun.rs`.

- [ ] **Step 6.1.3 (Verify)**:
```bash
cargo check && bash scripts/checks.sh check_tests && bash scripts/checks.sh check_clippy
```

- [ ] **Step 6.1.4 (Commit)**: `git commit -am "refactor(types): restringi visibilità a pub(crate) dove possibile"`

### Task 6.2: Docstring `///` sui tipi pubblici principali

**Files:**
- Modify: `src/types.rs` (AwkValue varianti, EvalContext, OutputStream, InputStream)
- Modify: `src/ast.rs` (Expr, Statement, BinaryOperator, GetlineSource, Rule, Program)

- [ ] **Step 6.2.1 (Red — abilita lint missing_docs come gate)**: aggiungi temporaneamente in cima a `src/main.rs` (o `src/lib.rs` se esiste): `#![warn(missing_docs)]`. Esegui `cargo build 2>&1 | grep -c 'missing documentation'`. Atteso: numero elevato sui tipi pubblici di types.rs e ast.rs.

- [ ] **Step 6.2.2 (Green)**: aggiungi una riga `///` di descrizione su ogni tipo pubblico (varianti, struct, enum). Per `AwkValue` varianti includi anche un esempio di quando vengono prodotte:

```rust
/// Valore AWK polimorfo. Le coercioni Number↔String seguono le regole POSIX.
pub(crate) enum AwkValue {
    /// Variabile mai assegnata. Coerce a 0 / "".
    Uninitialized,
    /// Valore numerico (double-precision IEEE 754).
    Number(f64),
    /// Stringa esplicita (literal o concat).
    String(String),
    /// Dual-typed: input da getline o $n che parsea numericamente.
    /// Cache di (testo, valore) evita re-parse a ogni confronto.
    StrNum(String, f64),
}
```

- [ ] **Step 6.2.3 (Verify)**:
```bash
cargo build 2>&1 | grep -c 'missing documentation'   # 0 sui tipi target
bash scripts/checks.sh check_tests
```

- [ ] **Step 6.2.4 (Commit)**: `git commit -am "docs: /// su tipi pubblici (AwkValue, EvalContext, AST enums)"`

### Task 6.3: README sezione Build & Test

**Files:**
- Modify: `README.md`

- [ ] **Step 6.3.1 (Red verification)**:
```bash
grep -q '## Build & Test' README.md && echo present || echo "RED-OK: assente"
```

- [ ] **Step 6.3.2 (Green)**: in `README.md` aggiungi sezione:

```markdown
## Build & Test

```bash
cargo build --release
cargo test                         # 109 testcase XML + property test + integrazione
cargo run -- -f program.awk file.txt
cargo run --bin diffrun -- tests/testsuite.xml   # confronto vs /usr/bin/awk
```

**Quality gates:**
- `cargo clippy --all-targets -- -D warnings`
- `cargo fmt --check`
- `bash scripts/checks.sh` (tutti i verification gate del piano di adeguamento)
```

- [ ] **Step 6.3.3 (Verify)**:
```bash
grep -q '## Build & Test' README.md && echo OK
```

- [ ] **Step 6.3.4 (Commit)**: `git commit -am "docs(README): sezione Build & Test"`

### Task 6.4: Append Step 19 al diario

**Files:**
- Modify: `NEXT_STEPS.md`

- [ ] **Step 6.4.1 (Green)**: in fondo a `NEXT_STEPS.md` (o nella sezione "Audit log" se esiste) aggiungi:

```markdown
## Step 19 — Idiomatic Cleanup (post-closure, 2026-05-19)
Adeguamento idiomatico Rust 2024 + chiusura debito Step 15.

| Fase | Esito |
|------|-------|
| Phase 0 — workspace hygiene (no macOS forks, no scratch, fmt) | ✅ |
| Phase 1 — clippy `all=deny` zero warning | ✅ |
| Phase 2 — runtime exit-free (chiude debito Step 15) | ✅ |
| Phase 3 — error context (with_context) + expect documentati | ✅ |
| Phase 4 — runner.rs splittato in {mod,builtins,io,fmt}.rs | ✅ |
| Phase 5 — proptest infrastructure fix (env!CARGO_BIN_EXE) | ✅ |
| Phase 6 — visibilità pub(crate) + docstring + README Build&Test | ✅ |

Invariante: 109/109 testcase XML verdi a ogni step. Esperimento resta chiuso, ora idiomatico.
```

- [ ] **Step 6.4.2 (Commit)**: `git commit -am "docs(diary): Step 19 — idiomatic cleanup completato"`

### Task 6.5: Sanity finale (tutti i gate verdi)

- [ ] **Step 6.5.1**:
```bash
bash scripts/checks.sh    # tutti i check OK
cargo test                # tutti i suite verdi
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

- [ ] **Step 6.5.2 (Push opzionale)**: comunica all'utente che il piano è concluso. NON pushare senza autorizzazione esplicita.

---

## Verification end-to-end

Al termine, da `/Volumes/Extreme Pro/Claude/testag-awk/rawk`:

```bash
bash scripts/checks.sh                                      # tutti OK
cargo clippy --all-targets -- -D warnings                   # zero
cargo fmt --check                                           # zero diff
cargo test                                                  # tutti i suite (xml_runner: 109)
cargo run --bin diffrun -- tests/testsuite.xml              # 109 vs system awk
wc -l src/runner/*.rs                                       # mod.rs ≤ ~350
test "$(grep -rn 'std::process::exit\|process::exit' src/ | grep -v 'src/main.rs' | wc -l)" -eq 0 && echo OK
test "$(grep -c '\.unwrap()' src/parser.rs)" -eq 0 && echo OK
```

## Out of scope (esplicitamente non in piano)

- **`length()` byte-count POSIX**: cambio comportamentale potrebbe rompere testcase; richiede decisione separata e regression diff vs gawk/mawk.
- **Sostituzione `String`→`Vec<u8>`**: rewrite massivo dei tipi base; alta resa, alto rischio; va trattato come "Phase 7" autonoma.
- **Sostituzione `chrono`→`time`**: micro-ottimizzazione, deferibile.
- **Doc coverage 100%**: oltre i tipi pubblici principali, marginale.
- **Property-test suite ampliata**: l'attuale `proptest_diff` basta per ora.

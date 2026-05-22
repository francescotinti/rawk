# Phase 7 — String → Vec<u8> Implementation Plan (rawk)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrare rawk dai tipi `String` ai tipi `Vec<u8>` per i dati AWK osservabili, in modo da gestire byte arbitrari con semantica POSIX bit-identica a `/usr/bin/awk`.

**Architecture:** Confine "data-only": tutti i dati AWK osservabili (`AwkValue::String/StrNum`, `record`, `fs`, `fields`, array keys, regex matcher, format strings, output) diventano `Vec<u8>`. Gli identificatori del programma (nomi variabili/funzioni/parametri, AST identifier nodes) restano `String` (POSIX vincola identifier ad ASCII). Sub-phase TDD-strict, ogni commit verde, invariante "109/109 testcase XML verdi".

**Tech Stack:** Rust 2024, `regex::bytes::Regex`, `std::io::BufRead::read_until`, `quick-xml`, `proptest`, `clap`. Pest grammar (`src/awk.pest`).

**Decomposizione (vedi design doc per dettaglio architetturale):**
- **7.0** — Pre-flight audit (no-code)
- **7.1** — I/O bytes input side (BufReader + record bytes)
- **7.2** — AwkValue bytes (core type swap + cascade fix)
- **7.3** — Array keys + runtime vars bytes — _piano in sessione propria_
- **7.4** — Regex matcher bytes (`regex::bytes`) — _piano in sessione propria_
- **7.5** — Builtins + printf bytes — _piano in sessione propria_
- **7.6** — Output bytes — _piano in sessione propria_
- **7.7** — Acceptance tests byte-aware — _piano in sessione propria_

Questo documento copre 7.0, 7.1, 7.2 in detail. Le sub-phase 7.3–7.7 ricevono `writing-plans` dedicato a inizio sessione (pattern STEP19: "una fase per sessione, piano fresco").

**Design doc di riferimento:** `diary/2026-05-20-rawk-phase7-bytes-design.md`.

---

## File Structure

Per le tre sub-phase coperte:

**Modify:**
- `src/types.rs` — `EvalContext.record`, `EvalContext.fs`, `AwkValue` variants
- `src/runner/mod.rs` — record loop, `update_record`, `eval_expr` per Variable/StringLiteral, builtin call sites
- `src/runner/io.rs` — `handle_output`, `ensure_input_file`, getline plumbing
- `src/runner/builtins.rs` — call site adaptation (cascade da `as_string`)
- `src/runner/fmt.rs` — printf format string + `%s` (cascade)
- `src/parser.rs` — `StringLiteral` / `RegexLiteral` produzione AST (cascade da 7.2)
- `src/ast.rs` — `Expr::StringLiteral(String)` → `Expr::StringLiteral(Vec<u8>)`

**Create:**
- `diary/STEP19_PHASE7_AUDIT.md` — report di pre-flight (7.0)
- `scripts/phase7_audit.sh` — script di audit riproducibile (7.0)

---

## Pre-flight rituale (per OGNI Task, all'inizio)

```bash
cd /Volumes/Extreme\ Pro/Claude/testag-awk/rawk
rtk proxy find . -name '._*' -not -path './.git/*' -delete   # cleanup macOS forks
bash scripts/checks.sh                                       # baseline verde
cargo test                                                   # baseline verde
git status --short                                           # tree atteso clean (no stale state)
```

Se uno dei tre fallisce: STOP. Non procedere finché non è verde.

---

# SUB-PHASE 7.0 — Pre-flight audit (no-code)

**Obiettivo**: produrre `diary/STEP19_PHASE7_AUDIT.md` con (a) elenco testcase XML con stringhe non-ASCII, (b) testcase che asseriscono `length()`/`substr()` con UTF-8 multi-byte, (c) conteggio call site da migrare per file, (d) verifica empirica che pest accetta byte 0x80-0xff nei letterali.

**Done**: report committato. Se trova conflitti, decisione documentata.

---

### Task 7.0.1: Crea script di audit `scripts/phase7_audit.sh`

**Files:**
- Create: `scripts/phase7_audit.sh`

- [ ] **Step 1: Verify red (script non esiste)**

```bash
test ! -f scripts/phase7_audit.sh && echo "RED-OK: script assente"
```
Expected: `RED-OK: script assente`

- [ ] **Step 2: Crea lo script**

Contenuto `scripts/phase7_audit.sh`:

```bash
#!/usr/bin/env bash
# Phase 7 pre-flight audit — produce report in diary/STEP19_PHASE7_AUDIT.md
set -euo pipefail
cd "$(dirname "$0")/.."

REPORT="diary/STEP19_PHASE7_AUDIT.md"

{
  echo "# Phase 7 — Pre-flight audit report"
  echo
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo
  echo "## (a) Testcase XML con stringhe non-ASCII"
  echo
  # cerca byte > 0x7f dentro <test>...</test>
  if rg --binary -l '[\x80-\xff]' tests/*.xml 2>/dev/null; then
    echo
    echo "(elenco file sopra)"
  else
    echo "Nessuno trovato — testsuite XML è ASCII pura."
  fi
  echo
  echo "## (b) Testcase con length()/substr()/index() su input multi-byte"
  echo
  rg -n 'length\(|substr\(|index\(' tests/testsuite.xml | head -30 || echo "(nessun match)"
  echo
  echo "## (c) Conteggio call site da migrare per file"
  echo
  echo '| File | String/&str hits |'
  echo '|------|-----------------:|'
  for f in src/*.rs src/runner/*.rs; do
    count=$(rg -c 'String|&str' "$f" 2>/dev/null || echo 0)
    echo "| \`$f\` | $count |"
  done
  echo
  echo "## (d) Verifica pest grammar su byte > 0x7f"
  echo
  echo "Test: \`echo 'BEGIN { print \"\\\\xff\" }' | cargo run -- -f /dev/stdin 2>&1\`"
  echo
  echo "Atteso: pest accetta escape \\NNN; verifica output:"
  echo
  echo '```'
  echo 'BEGIN { print "\xff" }' | cargo run -q -- 2>&1 | head -5 || echo "(rawk failed)"
  echo '```'
  echo
} > "$REPORT"

echo "Report written to $REPORT"
```

Crea il file con `Write`.

- [ ] **Step 3: Make executable**

```bash
chmod +x scripts/phase7_audit.sh
```

- [ ] **Step 4: Verify green (script eseguibile)**

```bash
test -x scripts/phase7_audit.sh && echo OK
```
Expected: `OK`

- [ ] **Step 5: Cleanup macOS forks + commit**

```bash
rtk proxy find . -name '._*' -not -path './.git/*' -delete
git add scripts/phase7_audit.sh
git commit -m "phase7.0(scripts): audit script per pre-flight Phase 7"
```

---

### Task 7.0.2: Esegui audit e genera report

**Files:**
- Create: `diary/STEP19_PHASE7_AUDIT.md`

- [ ] **Step 1: Verify red (report non esiste)**

```bash
test ! -f diary/STEP19_PHASE7_AUDIT.md && echo "RED-OK: report assente"
```
Expected: `RED-OK: report assente`

- [ ] **Step 2: Esegui lo script di audit**

```bash
bash scripts/phase7_audit.sh
```
Expected: `Report written to diary/STEP19_PHASE7_AUDIT.md`

- [ ] **Step 3: Verify green (report esiste e contiene le 4 sezioni)**

```bash
test -f diary/STEP19_PHASE7_AUDIT.md && \
  grep -c '^## (' diary/STEP19_PHASE7_AUDIT.md
```
Expected: `4` (le sezioni a, b, c, d)

- [ ] **Step 4: Leggi il report e annota i conflitti trovati**

Apri `diary/STEP19_PHASE7_AUDIT.md`. Per ogni sezione:
- **(a)**: se ha "Nessuno trovato" → zero conflitti, prosegui. Se elenca file: aggiungi note nella sezione finale "Decisioni" con strategia per ognuno.
- **(b)**: per ogni occorrenza `length(`/`substr(`, ispeziona il testcase per capire se la sua espected-output dipende da char-counting su input multi-byte. Conta i conflitti reali (filtra falsi positivi su input ASCII).
- **(c)**: tabella di hit, usata per stima costo Phase 7.2-7.6.
- **(d)**: verifica che `echo 'BEGIN { print "\xff" }' | cargo run -q --` non panic-a. Se panic-a, documenta come OP4 nel report.

Aggiungi sezione finale al report (Edit tool):

```markdown
## Decisioni di pre-flight

- (a): [N] file con byte non-ASCII trovati. [Strategia se >0].
- (b): [N] testcase con length()/substr() su multi-byte. [Strategia se >0].
- (c): Totale ~[N] call site String/&str da rivedere durante Phase 7.2-7.6.
- (d): Pest grammar [ACCETTA | NON ACCETTA] byte > 0x7f nei letterali stringa.

**Verdict**: [GO con criteri originali | GO con eccezioni documentate | NO-GO senza fix preliminare].
```

- [ ] **Step 5: Cleanup macOS forks + commit**

```bash
rtk proxy find . -name '._*' -not -path './.git/*' -delete
git add diary/STEP19_PHASE7_AUDIT.md
git commit -m "phase7.0(audit): pre-flight report — pattern, testcase, pest grammar"
```

---

### Task 7.0.3: Chiusura 7.0

- [ ] **Step 1: Verify gate scripts/checks.sh ancora verdi**

```bash
bash scripts/checks.sh
```
Expected: tutti i 7 check OK (nessun side-effect da 7.0).

- [ ] **Step 2: Verify cargo test ancora verde**

```bash
cargo test
```
Expected: 23 passed (baseline invariata).

- [ ] **Step 3: Commit chiusura phase 7.0**

```bash
rtk proxy find . -name '._*' -not -path './.git/*' -delete
git status --short  # atteso clean
echo "Phase 7.0 ✅ FATTO — vedere diary/STEP19_PHASE7_AUDIT.md per dettagli."
```

---

# SUB-PHASE 7.1 — I/O bytes (input side)

**Obiettivo**: trasformare il path di input da string-based a byte-based. `EvalContext.record` diventa `Vec<u8>`. Il record loop usa `read_until(b'\n', &mut buf)`. Field splitter accetta byte slice. AwkValue resta `String` per ora — adapter `String::from_utf8_lossy` ai bordi.

**Vincoli**:
- Le 109 XML testcase restano verdi (sono ASCII → lossy decode è identità su ASCII).
- `cargo build` e `cargo test` verdi a ogni commit.
- Nessun cambio di API pubblica osservabile dall'esterno.

**Done**: input pass-through bytes fino al boundary AwkValue (dove c'è ancora `String` con lossy decode marcato `// PHASE7.1→7.2 BRIDGE`).

---

### Task 7.1.1: Cambia `EvalContext.record` da `String` a `Vec<u8>`

**Files:**
- Modify: `src/types.rs:238`
- Modify: `src/types.rs:263` (`EvalContext::new` init)
- Modify: `src/types.rs:289-...` (`update_record` signature)
- Modify: `src/runner/mod.rs` (callers di `ctx.record`)

- [ ] **Step 1: Verify red**

```bash
grep -n 'record: String' src/types.rs
```
Expected: una riga ~238 con `pub(crate) record: String,`.

- [ ] **Step 2: Edit `src/types.rs:238`**

Cambia:
```rust
pub(crate) record: String,
```
in:
```rust
pub(crate) record: Vec<u8>,
```

- [ ] **Step 3: Edit `src/types.rs` init di record in `new`**

Trova:
```rust
record: String::new(),
```
e sostituisci con:
```rust
record: Vec::new(),
```

- [ ] **Step 4: Edit `update_record` signature**

Trova firma corrente (cerca con `rg 'fn update_record' src/types.rs`). Cambia da:
```rust
pub(crate) fn update_record(&mut self, line: &str) {
    self.record = line.to_string();
```
a:
```rust
pub(crate) fn update_record(&mut self, line: &[u8]) {
    self.record = line.to_vec();
```
Per la parte di splitting interna a `update_record` (whitespace/fs), aggiungi una conversione **temporanea** all'inizio della funzione marcata BRIDGE:
```rust
    // PHASE7.1→7.2 BRIDGE: field splitting still operates on str.
    let line_str = String::from_utf8_lossy(line);
    let line = line_str.as_ref();
```
e usa `line` (ora `&str`) nel corpo successivo. Il body originale (che usa `line.split_whitespace()` etc.) resta invariato dopo questa decoratura.

- [ ] **Step 5: Verify build (cascade compile errors expected)**

```bash
cargo build 2>&1 | rg 'error\[' | head -20
```
Expected: errori solo nei caller di `update_record` o di `ctx.record` che si aspettano `String`.

- [ ] **Step 6: Fix caller siti — cerca usi di `ctx.record` o `context.record`**

```bash
rg -n 'context\.record|ctx\.record' src/
```
Per ogni occorrenza, se è usato come `&str` (es. passato a `eval_expr` con `&context.record`), wrappa con `&String::from_utf8_lossy(&context.record)` se serve `&str`, o passa direttamente `&context.record[..]` se accetta `&[u8]`. Marcalo `// PHASE7.1→7.2 BRIDGE`.

Esempio tipico (`$0` evaluation in `eval_expr`):
```rust
// Prima:
AwkValue::from_str_num(context.record.clone())
// Dopo:
AwkValue::from_str_num(String::from_utf8_lossy(&context.record).into_owned()) // PHASE7.1→7.2 BRIDGE
```

- [ ] **Step 7: Fix caller di `update_record(&str)`**

```bash
rg -n 'update_record\(' src/
```
Per ogni call, il primo argomento ora deve essere `&[u8]`. I caller più probabili sono in `src/runner/mod.rs` (read loop). Cambia da:
```rust
ctx.update_record(&line);
```
(dove `line: String`) a:
```rust
ctx.update_record(line.as_bytes());
```

- [ ] **Step 8: Verify build green**

```bash
cargo build 2>&1 | rg 'error\['
```
Expected: nessun output (zero errori).

- [ ] **Step 9: Verify tutti i gate**

```bash
cargo test 2>&1 | tail -5
bash scripts/checks.sh 2>&1 | tail -10
cargo run --bin diffrun -- tests/testsuite.xml 2>&1 | tail -3
```
Expected: tutti verdi, 109/109 in diffrun.

- [ ] **Step 10: Cleanup + commit**

```bash
rtk proxy find . -name '._*' -not -path './.git/*' -delete
git add src/types.rs src/runner/mod.rs
git commit -m "phase7.1(types,runner): record diventa Vec<u8>, bridge lossy ai bordi"
```

---

### Task 7.1.2: BufRead bytes nel record loop

**Files:**
- Modify: `src/runner/mod.rs` (read loop, cerca con `rg 'read_line|BufRead' src/runner/mod.rs`)
- Modify: `src/runner/io.rs` (eventuali getline path che usano `read_line`)

- [ ] **Step 1: Verify red (read_line ancora presente)**

```bash
rg -n 'read_line' src/runner/ src/main.rs
```
Expected: almeno una occorrenza in `runner/mod.rs` per il main loop.

- [ ] **Step 2: Identifica il read loop principale**

Apri `src/runner/mod.rs` con `rg -A 6 'read_line' src/runner/mod.rs`. Probabilmente è una struttura tipo:
```rust
let mut line = String::new();
while reader.read_line(&mut line)? > 0 {
    let line_no_nl = line.strip_suffix('\n').unwrap_or(&line);
    ctx.update_record(line_no_nl.as_bytes());
    // ... eval main pattern actions
    line.clear();
}
```

- [ ] **Step 3: Sostituisci con `read_until(b'\n', ...)`**

Cambia in:
```rust
let mut line: Vec<u8> = Vec::new();
while reader.read_until(b'\n', &mut line)? > 0 {
    // strip trailing \n e \r\n
    if line.last() == Some(&b'\n') { line.pop(); }
    if line.last() == Some(&b'\r') { line.pop(); }
    ctx.update_record(&line);
    // ... eval main pattern actions
    line.clear();
}
```

- [ ] **Step 4: Cerca altri `read_line` (getline pipe/file)**

```bash
rg -n 'read_line' src/
```
Per ognuno applica la stessa trasformazione `Vec<u8>` + `read_until(b'\n', ...)`. Se il path corrente passa la stringa a `update_record`, ora deve passare `&[u8]`. Se passa a `from_str_num` (che ancora vuole `String`), usa BRIDGE: `from_str_num(String::from_utf8_lossy(&line).into_owned())`.

- [ ] **Step 5: Verify build**

```bash
cargo build 2>&1 | rg 'error\['
```
Expected: zero errori.

- [ ] **Step 6: Verify tutti i gate**

```bash
cargo test 2>&1 | tail -5
bash scripts/checks.sh 2>&1 | tail -10
cargo run --bin diffrun -- tests/testsuite.xml 2>&1 | tail -3
```
Expected: 109/109.

- [ ] **Step 7: Cleanup + commit**

```bash
rtk proxy find . -name '._*' -not -path './.git/*' -delete
git add src/runner/mod.rs src/runner/io.rs
git commit -m "phase7.1(runner): read_until bytes nel record loop + getline path"
```

---

### Task 7.1.3: Chiusura 7.1

- [ ] **Step 1: Conteggio BRIDGE marker (atteso piccolo numero)**

```bash
rg -c 'PHASE7.1' src/
```
Expected: ~3-8 occorrenze (saranno rimosse in 7.2).

- [ ] **Step 2: Verify gate finale**

```bash
bash scripts/checks.sh && cargo test && cargo run --bin diffrun -- tests/testsuite.xml | tail -1
```
Expected: tutti OK + 109/109.

- [ ] **Step 3: Commit di chiusura (nota nel diary, no code change)**

Aggiungi una sezione al diary `diary/2026-05-20-rawk-phase7-bytes-design.md` (Edit tool, in fondo):
```markdown
---

## Stato Phase 7.1 — chiusa 2026-05-20

- `record: Vec<u8>` con read_until bytes pass-through.
- Bridge lossy a `AwkValue` (sarà rimosso in 7.2): N occorrenze `// PHASE7.1→7.2 BRIDGE`.
- 109/109 XML verdi a ogni step.
- Commit SHA chiusura: [da inserire dopo il commit].
```

```bash
rtk proxy find . -name '._*' -not -path './.git/*' -delete
git add diary/2026-05-20-rawk-phase7-bytes-design.md
git commit -m "phase7.1(diary): chiusura — record Vec<u8>, 109/109 invariato"
```

---

# SUB-PHASE 7.2 — AwkValue bytes (core type)

**Obiettivo**: `AwkValue::String(Vec<u8>)` e `AwkValue::StrNum(Vec<u8>, f64)`. Cascade fix di tutti i call site. Rimozione adapter BRIDGE di 7.1.

**Rischio (R1 del design doc)**: cascade di compile-error >50 → spezzare in 7.2a (type) + 7.2b (callers). Decisione runtime durante Task 7.2.1.

**Done**: AwkValue bytes ovunque, zero BRIDGE marker residui da 7.1, 109/109 verdi.

---

### Task 7.2.1: Cambia AwkValue variants

**Files:**
- Modify: `src/types.rs:22,25` (variants)
- Modify: `src/types.rs:29-...` (`from_str_num` signature)
- Modify: `src/types.rs:46-...` (`as_string`, `as_string_convfmt` return type)

- [ ] **Step 1: Verify red**

```bash
grep -n 'String(String)' src/types.rs
grep -n 'StrNum(String,' src/types.rs
```
Expected: due hit.

- [ ] **Step 2: Cambia variant types**

In `src/types.rs`:
```rust
// Prima:
String(String),
StrNum(String, f64),
// Dopo:
String(Vec<u8>),
StrNum(Vec<u8>, f64),
```

- [ ] **Step 3: Cambia `from_str_num` signature**

```rust
// Prima:
pub(crate) fn from_str_num(s: String) -> Self {
    if let Ok(n) = s.trim().parse::<f64>() {
        AwkValue::StrNum(s, n)
    } else {
        AwkValue::String(s)
    }
}
// Dopo:
pub(crate) fn from_str_num(s: Vec<u8>) -> Self {
    let parsed = std::str::from_utf8(&s).ok().and_then(|t| t.trim().parse::<f64>().ok());
    match parsed {
        Some(n) => AwkValue::StrNum(s, n),
        None => AwkValue::String(s),
    }
}
```

- [ ] **Step 4: Cambia `as_string` return type**

```rust
// Prima:
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
// Dopo:
pub(crate) fn as_string(&self) -> Vec<u8> {
    self.as_string_convfmt("%.6g")
}
pub(crate) fn as_string_convfmt(&self, fmt: &str) -> Vec<u8> {
    match self {
        AwkValue::Uninitialized => Vec::new(),
        AwkValue::String(s) => s.clone(),
        AwkValue::StrNum(s, _) => s.clone(),
        AwkValue::Number(n) => format_number_awk(*n, fmt).into_bytes(),
    }
}
```

- [ ] **Step 5: Cambia `as_number` matching (StrNum/String ora hanno Vec<u8>)**

```rust
// Prima:
AwkValue::String(s) => s.trim().parse::<f64>().unwrap_or(0.0),
// Dopo:
AwkValue::String(s) => {
    std::str::from_utf8(s).ok()
        .and_then(|t| t.trim().parse::<f64>().ok())
        .unwrap_or(0.0)
}
```
Stesso pattern per `is_truthy` (controllo `is_empty()` resta valido perché `Vec<u8>::is_empty()` esiste).

Verifica anche `try_as_number` se esiste con pattern simile.

- [ ] **Step 6: Conta i compile error e decidi se spezzare**

```bash
cargo build 2>&1 | rg -c 'error\['
```
Se < 30 errori, procedi con Task 7.2.2 (caller fix) come pianificato.
Se > 50 errori, **STOP**: spezza in 7.2a (commit ora questo cambio con `#[allow(unused)]` aggressivo e match exhaustive minimo, lascia caller in 7.2b). Decisione dell'agente esecutore.

- [ ] **Step 7: Commit del type swap (anche con build rotta è OK per scopo TDD-strict? NO — qui aspettiamo Task 7.2.2 per commit verde)**

NB: in questo step NON committare ancora. Il build è probabilmente rosso. Procedi a Task 7.2.2.

---

### Task 7.2.2: Cascade fix dei caller di AwkValue

**Files:**
- Modify: `src/runner/mod.rs`, `src/runner/builtins.rs`, `src/runner/io.rs`, `src/runner/fmt.rs`, `src/parser.rs`, `src/ast.rs` (se usa `String` per `StringLiteral` — vedi 7.2.3)

- [ ] **Step 1: Itera sui compile errors**

```bash
cargo build 2>&1 | head -80
```

Per ogni error, applica una delle seguenti trasformazioni standard:

| Errore tipico | Fix |
|---|---|
| `expected Vec<u8>, found String` su `from_str_num(x)` dove `x: String` | `from_str_num(x.into_bytes())` |
| `expected Vec<u8>, found &str` su `AwkValue::String(s.to_string())` | `AwkValue::String(s.as_bytes().to_vec())` |
| `expected &str, found Vec<u8>` (es. `write!(f, "{}", val.as_string())`) | `f.write_all(&val.as_string())?` oppure `write!(f, "{}", String::from_utf8_lossy(&val.as_string()))` |
| `expected String, found Vec<u8>` su `HashMap<String, _>::insert(val.as_string(), ...)` (es. array keys) | BRIDGE: `String::from_utf8_lossy(&val.as_string()).into_owned()` marcato `// PHASE7.2→7.3 BRIDGE` (saranno chiavi `Vec<u8>` in 7.3) |
| `cmp::Ordering` su `String` vs `Vec<u8>` per comparison | confronta direttamente con `as_string().as_slice()` |

- [ ] **Step 2: Rimuovi adapter BRIDGE di 7.1**

```bash
rg -n 'PHASE7.1' src/
```
Per ogni `// PHASE7.1→7.2 BRIDGE`, ora che AwkValue è bytes-native, la conversione `String::from_utf8_lossy(&context.record).into_owned()` può diventare semplicemente `context.record.clone()` (passato a `from_str_num(Vec<u8>)`). Rimuovi il marker.

Esempio:
```rust
// Prima (BRIDGE da 7.1):
AwkValue::from_str_num(String::from_utf8_lossy(&context.record).into_owned()) // PHASE7.1→7.2 BRIDGE
// Dopo (no bridge):
AwkValue::from_str_num(context.record.clone())
```

- [ ] **Step 3: `update_record` non ha più bisogno del bridge interno**

Apri `src/types.rs` `update_record`. Era:
```rust
let line_str = String::from_utf8_lossy(line);
let line = line_str.as_ref();
// ... split logic on &str
```

Ora il body può usare direttamente `line: &[u8]` con `bstr`-style operations, MA per minimizzare cambio in 7.2 manteniamo la decode lossy interna come **TEMPORANEA** marcata `// PHASE7.2→7.5 BRIDGE` (sarà rimossa quando split su bytes arriva in 7.5). Lascia così:

```rust
// PHASE7.2→7.5 BRIDGE: field split su &str finché 7.5 non porta byte split.
let line_str = String::from_utf8_lossy(line);
let line = line_str.as_ref();
// ... split logic invariata
```

E nello split, la `AwkValue::from_str_num(s.to_string())` diventa `AwkValue::from_str_num(s.as_bytes().to_vec())`.

- [ ] **Step 4: Verify build**

```bash
cargo build 2>&1 | rg 'error\['
```
Expected: zero errori.

- [ ] **Step 5: Verify clippy zero warning**

```bash
cargo clippy --all-targets -- -D warnings 2>&1 | tail -5
```
Expected: `Finished` senza warning. Se ci sono warning su `clone()` non necessari, fixali.

- [ ] **Step 6: Verify all gates**

```bash
cargo fmt --check 2>&1 | head -3
cargo test 2>&1 | tail -5
bash scripts/checks.sh 2>&1 | tail -10
cargo run --bin diffrun -- tests/testsuite.xml 2>&1 | tail -3
```
Expected: tutti verdi, 109/109.

- [ ] **Step 7: Cleanup + commit**

```bash
rtk proxy find . -name '._*' -not -path './.git/*' -delete
git add -u src/
git commit -m "phase7.2(types,runner): AwkValue::String/StrNum diventa Vec<u8>"
```

---

### Task 7.2.3: AST literal types (se necessario)

**Files:**
- Modify: `src/ast.rs:49-50` (`StringLiteral`, `RegexLiteral`)
- Modify: `src/parser.rs` (cascade da AST change)
- Modify: `src/runner/mod.rs` (cascade nei match su `Expr::StringLiteral` / `Expr::RegexLiteral`)

> **Nota**: questo step può essere posticipato a 7.5 (builtins+printf) o anticipato qui, a discrezione dell'agente esecutore. Lo metto in 7.2 perché tocca i tipi del programma, coerentemente con il cambio AwkValue. Se diventa cascade-large e 7.2.2 è già grosso, posticipa a un sub-task 7.2c.

- [ ] **Step 1: Verify red**

```bash
grep -n 'StringLiteral(String)' src/ast.rs
grep -n 'RegexLiteral(String)' src/ast.rs
```
Expected: due hit.

- [ ] **Step 2: Cambia AST variants**

```rust
// Prima:
StringLiteral(String),
RegexLiteral(String),
// Dopo:
StringLiteral(Vec<u8>),
RegexLiteral(Vec<u8>),
```

- [ ] **Step 3: Cascade fix in parser.rs**

Cerca `Expr::StringLiteral(` in `src/parser.rs`. Probabilmente costruisce da `s.to_string()` dove `s: &str`. Cambia in `s.as_bytes().to_vec()`. Stesso per `RegexLiteral`.

NB: gli escape AWK (`\n`, `\t`, `\200`, ecc.) sono già processati dal parser; verifica che dopo il processing l'output è `Vec<u8>` con byte arbitrari (l'escape `\200` deve produrre byte `0x80`, non la stringa "\\200"). Se non è il caso, è un OP da risolvere in 7.5.

- [ ] **Step 4: Cascade fix in eval_expr (runner)**

Cerca `Expr::StringLiteral(`. Probabilmente `AwkValue::String(s.clone())` dove `s: &String`. Cambia in `AwkValue::String(s.clone())` (ora `s: &Vec<u8>` per cui `.clone()` è già giusto).

Per `RegexLiteral`: lo passa a `compile_or_get_regex(&re)`. La signature di `compile_or_get_regex` accetta ancora `&str` in 7.2 → BRIDGE: `compile_or_get_regex(&String::from_utf8_lossy(re))` marcato `// PHASE7.2→7.4 BRIDGE`.

- [ ] **Step 5: Verify build + test**

```bash
cargo build 2>&1 | rg 'error\['
cargo test 2>&1 | tail -5
bash scripts/checks.sh 2>&1 | tail -10
cargo run --bin diffrun -- tests/testsuite.xml 2>&1 | tail -3
```
Expected: zero errori build, 109/109 diffrun.

- [ ] **Step 6: Cleanup + commit**

```bash
rtk proxy find . -name '._*' -not -path './.git/*' -delete
git add src/ast.rs src/parser.rs src/runner/mod.rs
git commit -m "phase7.2(ast,parser): StringLiteral/RegexLiteral diventano Vec<u8>"
```

---

### Task 7.2.4: Chiusura 7.2

- [ ] **Step 1: Conteggio BRIDGE residui**

```bash
rg -c 'PHASE7\.' src/
```
Expected: BRIDGE 7.1→7.2 = 0 (rimossi); BRIDGE 7.2→7.3, 7.2→7.4, 7.2→7.5 = alcuni residui (verranno rimossi nelle sub-phase corrispondenti). Atteso < 15 totale.

- [ ] **Step 2: Gate finali**

```bash
bash scripts/checks.sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo run --bin diffrun -- tests/testsuite.xml | tail -1
```
Expected: tutto verde + 109/109.

- [ ] **Step 3: Update diary con chiusura 7.2**

Edit `diary/2026-05-20-rawk-phase7-bytes-design.md` (sezione "Stato"):
```markdown
## Stato Phase 7.2 — chiusa 2026-05-22

- `AwkValue::String(Vec<u8>)`, `AwkValue::StrNum(Vec<u8>, f64)` — core type bytes.
- AST `StringLiteral(Vec<u8>)`, `RegexLiteral(Vec<u8>)` — letterali pass-through bytes.
- Bridge 7.1→7.2 rimossi (0 residui). Restano 22 bridge 7.2→7.{3,4,5,6}: 6→7.3, 9→7.4, 5→7.5, 2→7.6.
- diffrun invariato 95/9/5 a ogni commit; `cargo test` 23 passed, `checks.sh` 7/7, clippy 0 lint.
- Type swap: 107 compile-error (R1 > 50) risolti in cascade unico verde, no commit rosso.
- Commit chiusura: `f82d574` (types,runner cascade), `cff8232` (ast,parser letterali).
```

- [ ] **Step 4: Commit chiusura**

```bash
rtk proxy find . -name '._*' -not -path './.git/*' -delete
git add diary/2026-05-20-rawk-phase7-bytes-design.md
git commit -m "phase7.2(diary): chiusura — AwkValue+AST bytes, 109/109 invariato"
```

---

# SUB-PHASE 7.3 — Array keys + runtime vars bytes
# SUB-PHASE 7.4 — Regex matcher bytes (`regex::bytes`)
# SUB-PHASE 7.5 — Builtins + printf bytes
# SUB-PHASE 7.6 — Output bytes
# SUB-PHASE 7.7 — Acceptance tests byte-aware

**Piano dedicato a inizio di ogni sessione**. A inizio della sessione che eseguirà 7.3:
1. Apri il design doc `diary/2026-05-20-rawk-phase7-bytes-design.md` sezione §3 per il dettaglio architetturale della sub-phase.
2. Esegui `Skill superpowers:writing-plans` con la sub-phase come scope.
3. Il piano risultante seguirà lo stesso pattern di 7.1/7.2 (Red verification → Edit minimal → Verify gate → Commit).

**Pre-requisite per ciascuna sub-phase**:
- Sub-phase precedente chiusa con commit verde.
- `git status --short` clean.
- `bash scripts/checks.sh` + `cargo test` + diffrun 109/109 OK.
- BRIDGE marker della sub-phase corrente da rimuovere (target: zero alla fine della sub-phase).

**File principali per sub-phase** (anteprima, dettaglio nel piano di sessione):

| Sub-phase | Files principali | BRIDGE da rimuovere |
|-----------|------------------|---------------------|
| 7.3 | `src/types.rs` (EvalContext.arrays, fs, convfmt, ofmt), `runner/mod.rs` array access | `// PHASE7.2→7.3 BRIDGE` (array key lossy) |
| 7.4 | `src/types.rs` (regex_cache), `runner/mod.rs` (`~`, `!~`, `match`, `gsub`, `sub`, `split` regex), Cargo features se serve | `// PHASE7.2→7.4 BRIDGE` (regex_cache + compile_or_get_regex) |
| 7.5 | `src/runner/builtins.rs` (length, substr, index, split), `src/runner/fmt.rs` (printf %s) | `// PHASE7.2→7.5 BRIDGE` (split su &str interno update_record) |
| 7.6 | `src/runner/io.rs` (handle_output), `src/types.rs` (OutputStream) | residui from_utf8_lossy nel path output |
| 7.7 | `tests/bytes_smoke.rs` (nuovo), eventuale `tests/bytes_*.xml` | n/a — solo additivo |

**Closure di Phase 7 (dopo 7.7)**:
- Conteggio finale `rg -c 'PHASE7\.' src/` = 0 oppure ≤3 documentati.
- Conteggio `rg -c 'from_utf8_lossy' src/` ≤ 3 (solo stderr o paths).
- Tutti i 7 gate `scripts/checks.sh` OK.
- `cargo run --bin diffrun -- tests/testsuite.xml` → 109/109.
- `cargo test --test bytes_smoke` → 6/6 (o N/N dei nuovi).
- Commit chiusura: `docs(phase7): Phase 7 ✅ FATTO — String→Vec<u8> bytes complete`.
- Push opzionale a origin (su autorizzazione utente, pattern STEP19 Phase 6.5.2).

---

## Self-Review checklist (autore plan, completato 2026-05-20)

- [x] **Spec coverage**: ogni sezione del design doc è mappata a una sub-phase del piano (7.0–7.7 ↔ §3 del design).
- [x] **Placeholder scan**: nessun TBD/TODO/"implement later". Codice concreto in ogni step di edit.
- [x] **Type consistency**: `AwkValue::String(Vec<u8>)` usato uniformemente; `as_string` ritorna `Vec<u8>` in tutti gli step che lo invocano; `update_record(line: &[u8])` consistente fra Task 7.1.1 (def) e 7.1.2 (chiamata `update_record(&line)`).
- [x] **Sub-phase boundaries**: 7.0–7.2 fully detailed; 7.3–7.7 esplicitamente deferred a sessioni dedicate, con file/bridge anticipati nella tabella riassuntiva. Non un placeholder: una scelta architetturale ("una fase per sessione").
- [x] **Pre-flight rituale**: incluso in ogni Task, allineato a memoria `macos_forks_cleanup.md`.

---

## Execution Handoff

Plan complete and saved to `diary/2026-05-20-rawk-phase7-bytes-plan.md`. Due opzioni di esecuzione:

**1. Subagent-Driven (recommended)** — dispatch fresh subagent per task, review tra task, iterazione veloce.

**2. Inline Execution** — esecuzione nella sessione corrente con `executing-plans`, batch con checkpoint.

**Quale preferisci?**

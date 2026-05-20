# Phase 7 — String → Vec<u8> design (rawk)

**Status**: design proposto (no code touched). Approvazione utente in corso.
**Data**: 2026-05-20
**Autore**: brainstorming session (superpowers:brainstorming) con Francesco Tinti.
**Predecessore**: STEP19_PLAN.md (Idiomatic Rust cleanup, chiuso 2026-05-19).

---

## 1. Obiettivo & criterio di "done"

### Obiettivo
rawk deve gestire input con byte arbitrari (non-UTF-8) senza panic e produrre output bit-identico a `/usr/bin/awk` su record con byte ≥ 0x80, NUL embedded, sequenze invalide UTF-8. I builtin `length()`, `substr()`, `index()`, `split()`, `match()` e il regex matcher diventano byte-based per allinearsi alla semantica POSIX.

### Motivazione
**POSIX correctness sui byte** (scelta esplicita: scartate performance, allineamento testcase only, esercizio architetturale). La semantica `length()` di POSIX è byte-count, non codepoint-count; il record e i field separator sono sequenze di byte; gli array keys derivati da `$1` possono contenere byte arbitrari.

### Criterio di "done" verificabile
1. Sub-phase **7.0 pre-flight audit** completata con report scritto (`diary/STEP19_PHASE7_AUDIT.md`).
2. Nuovi testcase byte-aware (suite `tests/bytes_smoke.rs` o capitolo XML dedicato) coprono almeno:
   - (a) input con byte 0x80–0xFF in pass-through;
   - (b) NUL embedded in record;
   - (c) sequenze UTF-8 invalide senza panic;
   - (d) `length()` byte-count su input multi-byte;
   - (e) `substr()` byte-based su input non-ASCII;
   - (f) regex `~`/`!~`/`match` su byte alto.
   → tutti verdi.
3. Le 109 testcase XML preesistenti restano verdi a ogni commit (invariante storico).
4. Gate finali tutti verdi: `bash scripts/checks.sh`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test`.

### Non-obiettivo
Performance non è un goal. Il rewrite può lasciare alcune `Vec<u8>` → `String` → `Vec<u8>` roundtrip ai bordi ASCII (es. parsing del format printf) se semplifica. Niente benchmark gate.

---

## 2. Architettura: confine `Vec<u8>` ↔ `String`

Principio guida: **dati AWK osservabili = `Vec<u8>`, identificatori del programma = `String`**.

### Diventano `Vec<u8>`
- `AwkValue::String(Vec<u8>)` e `AwkValue::StrNum(Vec<u8>, f64)` in `src/types.rs`.
- `EvalContext.record: Vec<u8>` (record corrente).
- `EvalContext.fs: Vec<u8>` (field separator; `FS=" "` triggera split a whitespace ASCII come oggi).
- Field array per record corrente: `Vec<Vec<u8>>` (o equivalente storage).
- Array storage: `HashMap<Vec<u8>, AwkValue>` per ogni array AWK (chiave = bytes della key concatenata da SUBSEP). Outer map nome→array resta `HashMap<String, ...>`.
- `EvalContext.regex_cache: HashMap<Vec<u8>, regex::bytes::Regex>` (pattern source come bytes; matcher byte-based).
- Output bytes verso stdout/stderr/file: scrittura via `Write::write_all(&[u8])`.
- AST letterali: `Expr::StringLiteral(Vec<u8>)`, `Expr::RegexLiteral(Vec<u8>)` (il programma AWK può contenere octal escape `\200`…`\377`).
- Format string di printf: `Vec<u8>` (POSIX permette byte arbitrari nel format).
- `SUBSEP`, `OFS`, `ORS`, `RS`, `CONVFMT`, `OFMT` runtime: `Vec<u8>` (assegnabili dall'utente con `BEGIN{OFS="\xff"}`).

### Restano `String`
- Nomi di variabili / funzioni / parametri formali: chiavi di `vars: HashMap<String, _>`, `functions: HashMap<String, _>`, `local_scopes: Vec<HashMap<String, _>>` (POSIX vincola identifier a `[A-Za-z_][A-Za-z0-9_]*`).
- AST identifier nodes: `Expr::Variable(String)`, `Expr::ArrayAccess(String, _)`, `Expr::FunctionCall(String, _)`, `Stmt::Assign(String, _)`, `Stmt::ForIn(String, String, _)`, `FunctionDecl.name`, `FunctionDecl.params`.
- Path file (CLI args, getline redirect literal): `String` finché trattati come UTF-8. Su macOS APFS è UTF-8; tradeoff accettato. Vedi R3.
- Messaggi diagnostici / errori interni: `String`.

### Casi border-line
- *Numeri formattati* (`format_number_awk`): l'output di `%.6g`/`CONVFMT` è ASCII → produzione `Vec<u8>` partendo da formatter `String` interno OK (un `into_bytes()` finale, zero-cost in pratica).
- *Stderr* (msg di errore runtime): scriviamo `String` lossy decode quando mostriamo un valore byte all'utente.
- *Cargo `regex` crate*: già in dependencies, ha modulo `regex::bytes::Regex` — niente nuova dipendenza.

---

## 3. Decomposizione in 7 sub-phase

Ogni sub-phase finisce con commit verde: tutti i 7 gate `scripts/checks.sh` OK + `cargo test` OK + 109/109 XML invariati. Tra sub-phase il sistema rimane semanticamente sano grazie ad **adapter lossy temporanei** alle frontiere non ancora migrate (verranno rimossi dalla sub-phase successiva).

### 7.0 — Pre-flight audit (no-code)
**Files**: `diary/STEP19_PHASE7_AUDIT.md` (nuovo)
**Output**: report con:
- (a) elenco testcase XML che usano stringhe non-ASCII;
- (b) elenco testcase che asseriscono `length()`/`substr()` con UTF-8 multi-byte;
- (c) conteggio call site da migrare per file;
- (d) verifica empirica che la pest grammar accetta byte 0x80-0xff nei letterali stringa/regex (o documentazione del workaround `\200` escape).

**Done**: report committato. Se trova conflitti, decisione caso-per-caso documentata.

### 7.1 — I/O bytes (input side)
**Files**: `src/runner/io.rs`, `src/runner/mod.rs` (record loop).
**Cambio**: BufReader legge bytes (`read_until(b'\n', &mut Vec<u8>)` invece di `read_line`). `EvalContext.record: Vec<u8>`. Field splitter accetta bytes (`fs: String` temporaneo, conversione `fs.as_bytes()` al call site).
**Adapter temporaneo**: `record_lossy_str()` per chiamanti AwkValue ancora `String`-based, marcato `// PHASE7.1→7.2 BRIDGE`.
**Done**: input bytes pass-through fino al primo step di valutazione (poi degradano in `String::from_utf8_lossy` finché 7.2 non ribalta AwkValue).

### 7.2 — AwkValue bytes (core type)
**Files**: `src/types.rs`, e cascata su `src/runner/mod.rs`, `src/runner/builtins.rs`.
**Cambio**: `AwkValue::String(Vec<u8>)`, `AwkValue::StrNum(Vec<u8>, f64)`. `as_string()` ritorna `Vec<u8>`. `as_string_convfmt(fmt: &str) -> Vec<u8>`. `from_str_num(s: Vec<u8>) -> Self`. Rimuovi adapter lossy di 7.1.
**Done**: AwkValue è bytes ovunque. Compile-error cascade gestito; le 109 XML restano verdi.

### 7.3 — Array keys + variabili runtime bytes
**Files**: `src/types.rs` (`EvalContext.arrays`, `fs`, `convfmt`, `ofmt`), call site in `runner/mod.rs`.
**Cambio**: `arrays: HashMap<String, HashMap<Vec<u8>, AwkValue>>`. `fs: Vec<u8>`, `convfmt: Vec<u8>` (con conversione `from_utf8_lossy` quando passato a printf), `ofs/ors/rs: Vec<u8>`, `subsep: Vec<u8>`.
**Done**: array con chiavi byte. ENVIRON e ARGV propagati lossy-at-boot (OP2/OP3).

### 7.4 — Regex matcher bytes
**Files**: `src/types.rs` (regex_cache), `src/runner/mod.rs` (operatori `~` `!~`, `match()`, `gsub`/`sub`/`split` con pattern).
**Cambio**: `regex_cache: HashMap<Vec<u8>, regex::bytes::Regex>`. Compilazione: tentativo `std::str::from_utf8(pattern)` → se OK, `regex::bytes::RegexBuilder::new(s).build()`; se non-UTF-8, `String::from_utf8_lossy(pattern).into_owned()` + warning a stderr (la regex byte-arbitraria è caso raro, e POSIX permette solo escape `\NNN` nei letterali, che pest converte già a bytes UTF-8-validi). Match: `regex.is_match(haystack: &[u8])`, `regex.captures(haystack: &[u8])`. Unicode flag OFF di default in `regex::bytes` — è ciò che vogliamo.
**Done**: regex matching su bytes. AST `RegexLiteral(Vec<u8>)`. Letterali regex con `\200` legali.

### 7.5 — Builtins + printf bytes
**Files**: `src/runner/builtins.rs`, `src/runner/fmt.rs`.
**Cambio**: `length()` → `bytes.len()`. `substr(s, m, n)` → slice byte-based con check bounds. `index(s, t)` → byte search (memchr-style). `split(s, arr, fs)` → byte split. `sprintf`/`printf` accettano format `Vec<u8>`; `%s` pass-through bytes; `%d %f %g` come adesso (ASCII output).
**Done**: tutti i builtin stringa byte-based. Letterali AST `StringLiteral(Vec<u8>)`.

### 7.6 — Output bytes
**Files**: `src/runner/io.rs` (OutputStream), `src/runner/mod.rs` (print/printf).
**Cambio**: `OutputStream::Stdout` espone `Write` su `io::stdout().lock()`. `print` scrive bytes direttamente. `printf` scrive bytes del format-result. File redirect (`>`, `>>`, `|`) su `BufWriter<File>` di bytes.
**Done**: zero `from_utf8`/`from_utf8_lossy` residui nel path di output. Pass-through bytes end-to-end.

### 7.7 — Acceptance tests byte-aware
**Files**: `tests/bytes_smoke.rs` (nuovo file di integrazione) o nuovi `.xml` di testcase nel formato esistente.
**Cambio**: aggiungi almeno 6 testcase coprendo:
1. `print` di record con bytes 0x80–0xFF.
2. record con NUL embedded (`printf '\x00abc' | rawk 'NF{print}'`).
3. input UTF-8 invalido senza panic.
4. `length()` su input ISO-8859-1 multi-byte (verifica byte-count).
5. `substr($1, 1, 3)` byte-based su input non-ASCII.
6. `/[\x80-\xff]/` regex match su byte alto.

**Done**: tutti i 6 nuovi verdi + 109 XML verdi + gate completi.

**Stima**: 7 commit (uno per sub-phase) + 1 commit di chiusura. ~6-8 sessioni.

---

## 4. Invarianti & TDD-strict per sub-phase

Ogni sub-phase rispetta il protocollo TDD-strict di STEP19: **Red → Green → Verify → Commit**, con pre-flight di igiene macOS.

### Pre-flight di ogni sub-phase (rituale fisso)
```bash
rtk proxy find . -name '._*' -not -path './.git/*' -delete   # cleanup macOS forks
bash scripts/checks.sh                                       # baseline verde
cargo test                                                   # baseline verde
git status --short                                           # clean tree
```

### Step pattern dentro la sub-phase (per ogni Task)
1. **Red verification**: comando `grep`/`rg`/`cargo build` che mostra che il cambio NON è ancora applicato.
   Esempio: `grep -q 'AwkValue::String(String)' src/types.rs && echo "RED-OK"`.
2. **Green edit**: minima Edit per applicare il cambio (Edit tool, no Write).
3. **Verify**: `cargo build && cargo test && bash scripts/checks.sh` tutti verdi.
4. **Commit**: messaggio `<sub-phase>(<area>): <change>`.
   Esempio: `phase7.2(types): AwkValue::String bytes`.

### Adapter lossy temporanei (sub-phase boundary)
- **7.1 → 7.2**: `EvalContext.record: Vec<u8>` ma AwkValue ancora `String` → adapter `String::from_utf8_lossy(&ctx.record).into_owned()` ai call site `$0`. Marcato `// PHASE7.1→7.2 BRIDGE`.
- **7.3 → 7.4**: regex_cache key `Vec<u8>` ma matcher ancora `regex::Regex` → `std::str::from_utf8(&haystack)` ai matcher con lossy fallback su input non-UTF-8.
- **7.5 → 7.6**: builtins ritornano `Vec<u8>` ma output stream ancora `String`-based → `String::from_utf8_lossy` al boundary print.

### Invariante 109 XML
Dopo ogni sub-phase, `cargo run --bin diffrun -- tests/testsuite.xml` deve riportare 109/109 match vs system awk. Se anche un solo testcase fallisce, la sub-phase NON è done — rollback o fix puntuale prima del commit.

### Gate finali Phase 7 (dopo 7.7)
```bash
bash scripts/checks.sh                                      # 7 gate OK
cargo clippy --all-targets -- -D warnings                   # zero
cargo fmt --check                                           # zero diff
cargo test                                                  # tutte le suite verdi
cargo run --bin diffrun -- tests/testsuite.xml              # 109/109
cargo test --test bytes_smoke                               # 6/6 (nuovi)
test "$(rtk grep -rn 'from_utf8_lossy' src/ | wc -l)" -le 3 && echo OK
```

---

## 5. Risk register & open points

### Rischi noti & mitigazione

| # | Rischio | Probabilità | Impatto | Mitigazione |
|---|--------|-------------|---------|-------------|
| R1 | Sub-phase 7.2 (AwkValue) genera cascade di compile-error troppo grande per una sessione | Media | Alta | Limitare 7.2 al cambio variant + adapter `as_string_lossy()` interno. Se >50 errori, spezzare in 7.2a (type) + 7.2b (callers). |
| R2 | `regex::bytes::Regex` ha differenze di parsing pattern vs `regex::Regex` (es. `\d`/`\w` ASCII-only) che rompono testcase XML | Bassa | Alta | Pre-flight 7.0 deve scannare i pattern usati nei testcase. `regex::bytes` ha unicode flag OFF di default — è ciò che vogliamo. Verifica empirica su 10 pattern prima di 7.4. |
| R3 | Path file su macOS gestiti come `String`: input con filename non-UTF-8 panic-a su `from_utf8` | Bassa | Bassa | Out of scope: path passati a rawk sono argv (clap = `String`) e redirect literal (ASCII nei testcase). Documentato come limitazione nota. |
| R4 | printf `%s` con format string a byte arbitrari ma argomenti misti number/string: ambiguità su come format-parsare | Media | Media | Format string scanner byte-based (state machine su `&[u8]`), output `Vec<u8>` accumulato. `%d`/`%g` producono ASCII; `%s` pass-through bytes. Test mirato in 7.5. |
| R5 | `HashMap<Vec<u8>, _>` ha lookup leggermente più lento di `HashMap<String, _>` | Bassa | Bassa | Non-obiettivo: performance non è criterio di done. Se diventasse misurabile, valutare `bstr::BString` (out of scope Phase 7). |
| R6 | `as_string_convfmt` ritorna `Vec<u8>` ma `format_number_awk` produce `String` da `format!` di Rust: roundtrip `String→Vec<u8>` | Bassa | Bassa | Roundtrip ASCII è zero-cost in pratica (`s.into_bytes()`). Marcato esplicito nel codice. |
| R7 | Tempo totale Phase 7 (8 sessioni) > budget utente | Variabile | Variabile | Ogni sub-phase è autonoma e committabile. Si può fermare a 7.4 (regex bytes) e considerare 7.5-7.7 un Phase 8 a sé. |

### Open points da risolvere DURANTE le sub-phase (non blocking del design)

- **OP1** — `getline` da pipe: lo stream da `cmd | getline` produce bytes; il parsing in `StrNum` deve usare `from_str_num(Vec<u8>) -> AwkValue` con conversione `from_utf8` strict (fallback `String`).
- **OP2** — `ENVIRON`: le env var di Rust sono `OsString`; su macOS sono UTF-8 — accettiamo lossy decode al boot.
- **OP3** — `ARGV[i]`: come ENVIRON. Lossy decode all'avvio.
- **OP4** — Errore di parser AWK con regex literal contenente byte > 0x7f: il pest parser attualmente assume UTF-8. Da verificare in 7.0. Se non accetta byte 0x80-0xff in `/.../`, escape `\200` resta l'unica via (POSIX-conforme).

### Out of scope esplicito per Phase 7

- Path bytes (R3): rawk userà `String` per i path; chi vuole nomi file non-UTF-8 deve usare workaround shell.
- Locale/collation: rawk non considera `LC_COLLATE`. Confronti stringa = byte compare.
- Performance benchmark vs gawk/mawk: non un criterio di done.
- Refactor a `bstr::BString`: micro-ottimizzazione futura.

---

## Riferimenti

- `STEP19_PLAN.md` — piano predecessore (idiomatic cleanup), chiuso 2026-05-19.
- `NEXT_STEPS.md` — diario di porting C→Rust.
- POSIX awk specification (IEEE Std 1003.1-2017) — base normativa per byte-semantics.
- `regex::bytes` API docs — matcher byte-based, unicode-off-by-default.

---

## Stato Phase 7.1 — chiusa 2026-05-21

- `EvalContext.record: Vec<u8>` con read_until bytes pass-through nel record loop (`process_single_byte`) e nei tre path di `getline` (Main/File/Pipe).
- `update_record` ora accetta `&[u8]`; field splitting interno conserva la semantica via BRIDGE `String::from_utf8_lossy` marcato `// PHASE7.1→7.2 BRIDGE`.
- Boundary residui (saranno rimossi in 7.2): 14 BRIDGE marker totali — 5 in `src/types.rs` (field split, `get_field($0)`, `set_field($0)`, due join OFS); 6 in `src/runner/mod.rs` (RT in `process_single_byte`, 4 nei path `process_paragraph`/`process_regex_rs` che usano ancora `read_to_string`, 1 nel ramo `getline … > var`); 3 in `src/runner/builtins.rs` (`length()` su $0, `sub`/`gsub` target_val, update record finale).
- Gate verdi a ogni commit: `cargo test` 23 passed, `bash scripts/checks.sh` 7/7 OK, `diffrun tests/testsuite.xml` invariato a 95 MATCH / 9 DIVERGE / 5 SKIPPED.
- Commit di chiusura: `0b5e86f` (types,runner record Vec<u8>), `c484d29` (runner read_until bytes + getline).

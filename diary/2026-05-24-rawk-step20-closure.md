# rawk Step 20 — diffrun annotated divergence (CLOSURE)

**Data:** 2026-05-24
**Commits:** `44e8858` (plan) · `a75c76a` (phase 1) · `1eeb46c` (phase 2) · `85c4243` (phase 3)
**Modalità:** una sessione, TDD strict (RED → GREEN → VERIFY → COMMIT).

## Premessa e motivazione

`xml_runner_test` passa **109/109** testcase contro `expected_stdout`. `diffrun`, il tool di confronto rawk vs BWK awk di sistema (`/usr/bin/awk` 20200816), riportava 8-9 DIVERGE su 109 — ma in **tutti** gli 8 stabili rawk seguiva `expected_stdout` mentre BWK divergeva (parser più strict, fatal su warning, NUL-truncation, escape handling, ecc.). Il bucket `DIVERGE` confondeva quindi divergenze "intenzionali" (oracoli del testsuite) e potenziali regressioni reali — rendendo il segnale inutilizzabile come gate CI.

**Why:** dare a `diffrun` un segnale binario affidabile come gate CI (`UNEXPECTED-DIVERGE > 0 ⇒ regression`).

**Out of scope:** chiudere i 5 SKIPPED gawk-extensions, performance, locale, gensub, modificare runtime AWK.

## Fasi

### Phase 0 — Plan & hygiene (commit `44e8858`)
- Verificata baseline verde: `cargo test --test xml_runner_test` → 109/109.
- Snapshot baseline diffrun: `96 MATCH / 8 DIVERGE / 5 SKIPPED` (con flakiness 95/9 dovuta a `test_gawk_manual_word_frequency`, iter HashMap non-det).
- `STEP20_PLAN.md` con 3 fasi e tabella di stato.

### Phase 1 — Schema + 4-bucket logic (commit `a75c76a`)

**RED:** nuovo `tests/diffrun_buckets.rs` con 2 test:
- `diffrun_emits_four_bucket_summary` — asserisce presenza dei 4 label (MATCH/EXPECTED-DIVERGE/UNEXPECTED-DIVERGE/SKIPPED) nello stdout.
- `diffrun_exits_nonzero_when_unexpected_divergences_present` — invariante su exit code in funzione del bucket UNEXPECTED.

**GREEN:** in `src/bin/diffrun.rs`:
- Nuovo struct `ExpectedDivergence { reason: String }` con `#[serde(rename = "@reason")]`.
- Campo `expected_divergence: Option<ExpectedDivergence>` su `TestCase`.
- Main loop: divergenze annotated → `expected_diverge_cases`; non annotate → `unexpected_diverge_cases`.
- Output a 4 righe + sezioni `== UNEXPECTED DIVERGENCES (regressions) ==` e `== EXPECTED DIVERGENCES (annotated) ==` separate.
- `exit(1)` iff `unexpected_diverge_cases > 0`.

**Verify:** `cargo test --test diffrun_buckets` → 2 passed. `xml_runner_test` ancora 109/109. clippy 0 errori. fmt clean.

### Phase 2 — Annotazione 9 testcase (commit `1eeb46c`)

Verificato BWK awk su ciascun caso con `/usr/bin/awk` standalone per nailare la `reason`. Categorie usate:

| `reason` | Significato |
|----------|-------------|
| `bwk-parser-quirk` | BWK syntax error / parser strictness |
| `bwk-fatal-on-warning` | BWK aborta dove rawk emette warning + continua |
| `bwk-truncates-at-nul` | BWK C-string vs rawk byte (Phase 7) |
| `bwk-escape-handling` | backslash-escape policy diversa |
| `bwk-different-number-format` | OFMT per huge floats |
| `non-deterministic-iteration-order` | array iter (HashMap) order non garantito |

Annotation applicate (9 file `tests/cases/*.xml`):

| File | `reason` |
|------|----------|
| `0003_test_type_coercion.xml` | `bwk-parser-quirk` |
| `0017_test_gawk_manual_word_frequency.xml` | `non-deterministic-iteration-order` |
| `0037_test_concat_func_call_disambig.xml` | `bwk-fatal-on-warning` |
| `0064_test_escape_octal_zero.xml` | `bwk-truncates-at-nul` |
| `0066_test_escape_unknown_preserved.xml` | `bwk-escape-handling` |
| `0085_test_nextfile_in_user_function.xml` | `bwk-fatal-on-warning` |
| `0107_test_print_huge_uses_scientific.xml` | `bwk-different-number-format` |
| `0108_test_div_by_zero_warning_continues.xml` | `bwk-fatal-on-warning` |
| `0109_test_unknown_function_warning_continues.xml` | `bwk-fatal-on-warning` |

Risultato (stable su 5 run consecutivi):
```
  MATCH:               95
  EXPECTED-DIVERGE:    9
  UNEXPECTED-DIVERGE:  0
  SKIPPED:             5
  exit = 0
```

### Phase 3 — `scripts/checks.sh` gate (commit `85c4243`)

Aggiunta funzione `check_diffrun_no_unexpected` che builda diffrun release, lo esegue, parsa la riga `UNEXPECTED-DIVERGE:` e fallisce se > 0.

Verifica red→green: rimossa temporaneamente l'annotation da `0064_test_escape_octal_zero.xml` ⇒ gate exit=1; ripristinata ⇒ exit=0.

## Stato finale

**Gate verdi (8/8):**
- `check_clippy`                   OK
- `check_diffrun_no_unexpected`    OK
- `check_fmt`                      OK
- `check_no_exit_outside_main`     OK
- `check_no_macos_forks`           OK
- `check_no_scratch_root`          OK
- `check_runner_split`             OK
- `check_tests`                    OK

**Test:**
- `cargo test --test xml_runner_test` → 109/109 (invariante mantenuta)
- `cargo test --test diffrun_buckets` → 2 passed

**Diffrun:**
- 95 MATCH / 9 EXPECTED-DIVERGE / 0 UNEXPECTED-DIVERGE / 5 SKIPPED
- exit 0 stabile, deterministic w.r.t. il bucketing (flakiness HashMap assorbita nel bucket EXPECTED via annotazione `non-deterministic-iteration-order`)

## Implicazioni operative

Da Step 20 in poi, una regressione reale è banalmente rilevabile:
- Una modifica a rawk che fa divergere un testcase prima MATCH → l'output sarà spostato a `UNEXPECTED-DIVERGE: 1` e `bash scripts/checks.sh` fallisce.
- Modifiche intenzionali al testsuite (es. nuovo edge case POSIX in cui BWK diverge) si annotano con `<expected_divergence reason="..."/>` durante il commit, evitando rumore CI.

Il vocabolario di `reason` è esteso quando emerge un caso nuovo non riducibile alle 6 categorie attuali.

## Out of scope, candidati Step 21+

- Chiusura 5 SKIPPED (gawk-extensions): bitwise, RT, BEGINFILE/ENDFILE, gensub/mktime, srand.
- Performance pass: rimozione `.clone()` su record/fs/printf path ora che la base è `Vec<u8>`.
- `test_gawk_manual_word_frequency` ⇒ migrazione testcase per usare `for (k in arr) | "sort"` o iteratore deterministico, e rimozione annotation `non-deterministic-iteration-order`.

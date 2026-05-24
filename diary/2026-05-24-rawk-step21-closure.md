# rawk Step 21 — chiusura SKIP stale bitwise+srand (CLOSURE)

**Data:** 2026-05-24
**Commits:** `fd8aa01` (plan) · `04e8a6b` (phase 1) · *(questo diary)*
**Modalità:** una sessione, TDD strict (RED → GREEN → VERIFY → COMMIT).

## Premessa e motivazione

Dopo Step 20 `diffrun` riportava `MATCH 96 / EXPECTED 8 / UNEXPECTED 0 / SKIPPED 5` (con la solita oscillazione 95/9 ↔ 96/8 dovuta a `0017_test_gawk_manual_word_frequency`, ammortizzata nel bucket EXPECTED). Indagine: di quei **5 SKIPPED, 2 erano stale** — l'euristica `is_skip()` in `src/bin/diffrun.rs` continuava a marcarli come "gawk-only feature" anche se rawk le ha implementate.

- `0006_test_bitwise_and_time.xml` → skip "uses gawk-only bitwise functions", ma `and`/`or`/`xor`/`lshift`/`rshift` sono implementate in `src/runner/builtins.rs` (linee 166–190). rawk produce `1 4` (= `expected_stdout`); BWK awk fallisce con `calling undefined function and` (exit 2).
- `0012_test_gawk_manual_seven_random_numbers.xml` → skip per `script.contains("srand")`, ma rawk implementa `srand`/`rand` con `rand::rngs::StdRng` e l'oracolo è calibrato sull'RNG di rawk. BWK produce un'altra sequenza (PRNG diverso).

In entrambi i casi rawk segue l'oracolo, BWK diverge → la classificazione corretta è **EXPECTED-DIVERGE**, non SKIPPED.

## Fasi

### Phase 0 — Plan & hygiene (commit `fd8aa01`)

- Verificata baseline `bash scripts/checks.sh` → 8/8.
- Snapshot baseline: `MATCH 95-96 / EXPECTED 8-9 / UNEXPECTED 0 / SKIPPED 5`.
- `STEP21_PLAN.md` con 2 fasi operative + Phase 2 di chiusura.

### Phase 1 — Snellire `is_skip()` + annotare 2 testcase (commit `04e8a6b`)

**RED:** aggiunto in `tests/diffrun_buckets.rs` un nuovo test `diffrun_step21_target_counts` che asserisce gli invarianti post-Step 21:
- `MATCH + EXPECTED-DIVERGE == 106`
- `UNEXPECTED-DIVERGE == 0`
- `SKIPPED == 3`
- `exit == 0`

Forma stabile rispetto alla flakiness MATCH↔EXPECTED su `0017_test_gawk_manual_word_frequency`. Lanciato con `--test-threads=1` (i 3 test del file invocano `diffrun` e in parallelo collidono sul filesystem usato da `redirect_append`): fallimento atteso `left: (104, 0, 5) vs right: (106, 0, 3)`.

**GREEN:**
- In `src/bin/diffrun.rs::is_skip()` rimossi due blocchi:
  - `and(`/`or(`/`xor(`/`lshift(`/`rshift(` → era falso positivo (rawk implementa)
  - `srand` → era over-skip (rawk implementa con sequenza deterministica)
- Annotate due `tests/cases/*.xml`:
  - `0006_test_bitwise_and_time.xml`: `<expected_divergence reason="bwk-missing-gawk-extension"/>`
  - `0012_test_gawk_manual_seven_random_numbers.xml`: `<expected_divergence reason="bwk-different-rng"/>`

**Vocabolario `reason` esteso** (da 6 → 8 categorie):

| `reason` (nuovo) | Significato |
|------------------|-------------|
| `bwk-missing-gawk-extension` | BWK awk non implementa la builtin che rawk supporta (bitwise et al.) |
| `bwk-different-rng` | rawk e BWK usano PRNG diversi; l'oracolo è calibrato su rawk |

**Verify:**
- `cargo test --test diffrun_buckets --release -- --test-threads=1` → 3 passed.
- `cargo test --test xml_runner_test --release` → 1 suite ok (109 testcase, annotazioni ignorate dal runner).
- `cargo run --release --bin diffrun` su 5 run consecutivi:
  - `UNEXPECTED-DIVERGE = 0` ✓ stabile
  - `SKIPPED = 3` ✓ stabile
  - `MATCH + EXPECTED = 106` ✓ stabile (oscillazione 95/11 ↔ 96/10 confinata a quei due bucket, da `0017` non-determinismo)
- `bash scripts/checks.sh` → 8/8 OK (dopo cleanup macOS forks).

## Stato finale

**Gate verdi (8/8):**

```
check_clippy                   OK
check_diffrun_no_unexpected    OK
check_fmt                      OK
check_no_exit_outside_main     OK
check_no_macos_forks           OK
check_no_scratch_root          OK
check_runner_split             OK
check_tests                    OK
```

**Diffrun (stabile sugli invarianti):**

```
  MATCH:               95 o 96
  EXPECTED-DIVERGE:    11 o 10
  UNEXPECTED-DIVERGE:  0          # invariante
  SKIPPED:             3          # invariante
  exit = 0                        # invariante
```

I 3 SKIPPED restanti sono feature genuinamente non implementate in rawk:

| File | Feature mancante |
|------|------------------|
| `0029_test_record_terminator_rt.xml` | variabile `RT` |
| `0080_test_rs_rt_paragraph.xml` | variabile `RT` (RS="" paragraph mode) |
| `0083_test_nextfile_single_file_endfile_runs.xml` | `BEGINFILE` / `ENDFILE` |

## Implicazioni operative

Dopo Step 21 la regola operativa di `diffrun` resta invariata (vedi Step 20):

- `UNEXPECTED-DIVERGE > 0` ⇒ regressione, gate CI fallisce.
- Nuovo testcase XML in cui rawk e BWK divergono per ragioni note ⇒ aggiungere `<expected_divergence reason="…"/>` scegliendo dal vocabolario (ora 8 categorie).
- `SKIPPED` ora identifica esclusivamente *feature gawk non implementate in rawk*. Quando una feature gawk viene implementata, il rispettivo testcase va promosso da SKIPPED a MATCH/EXPECTED-DIVERGE rimuovendo la sua euristica da `is_skip()`.

## Out of scope, candidati Step 22+

- **Step 22** — Implementazione `RT` (variabile aggiornata dopo split su RS regex / paragraph-mode). 2 testcase coinvolti.
- **Step 23** — Implementazione `BEGINFILE` / `ENDFILE`. 1 testcase coinvolto.
- **Step 24** — Performance pass post Phase 7 bytes: rimozione `.clone()` su record/fs/printf path.
- **Step 25** — Migrazione `0017_test_gawk_manual_word_frequency` a iterazione deterministica (rimozione annotation `non-deterministic-iteration-order`).

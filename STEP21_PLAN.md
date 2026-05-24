# rawk — Step 21: chiusura SKIP stale (bitwise + srand già implementate) (TDD strict)

> **Per l'agent che esegue:** SUB-SKILL CONSIGLIATA: `superpowers:test-driven-development`. Step in checkbox (`- [ ]`). Disciplina TDD strict (Red → Green → Verify → Commit). Una fase per sessione salvo richiesta esplicita.

---

## Stato di avanzamento (aggiornare a fine di ogni sessione)

| Fase | Stato | Commit di chiusura | Note |
|------|-------|--------------------|------|
| Phase 0 — Plan & hygiene | ⏳ TODO | — | STEP21_PLAN.md + macOS forks clean |
| Phase 1 — Rimozione skip stale + annotation 2 testcase | ⏳ TODO | — | is_skip() snellito + 2 XML annotati + diffrun_buckets.rs aggiornato |
| Phase 2 — Verify + diary | ⏳ TODO | — | scripts/checks.sh 8/8 + diary di chiusura |

Legenda stato: ⏳ TODO · 🚧 IN CORSO · ✅ FATTO · ⚠️ PARZIALE · ❌ BLOCCATO

---

## Motivazione

**Why:** Dopo Step 20 `diffrun` riporta `MATCH 96 / EXPECTED-DIVERGE 8 / UNEXPECTED-DIVERGE 0 / SKIPPED 5`. Indagine: di quei 5 SKIP, **2 sono ormai stale**:

- `0006_test_bitwise_and_time.xml` → marca skip "uses gawk-only bitwise functions". Ma `and`/`or`/`xor`/`lshift`/`rshift` **sono implementate** in `src/runner/builtins.rs` (linee 166–190). rawk produce `1 4` (= `expected_stdout`); BWK awk fallisce con `calling undefined function and` (exit 2). È quindi una **EXPECTED-DIVERGE**, non uno SKIP.
- `0012_test_gawk_manual_seven_random_numbers.xml` → marca skip per via di `srand` (heuristic `script.contains("srand")`). Ma rawk implementa `srand`/`rand` con `rand::rngs::StdRng` e l'expected_stdout è calibrato sull'RNG di rawk (`98/69/43/17/25/97/39`). BWK awk produce un'altra sequenza (`84/39/79/80/92/19/33`) → DIVERGE per via di RNG diverso, ma rawk segue l'oracolo. È quindi una **EXPECTED-DIVERGE** con reason dedicato.

Gli altri 3 SKIPPED restano legittimi (feature non implementata in rawk):

- `0029_test_record_terminator_rt.xml` — variabile `RT`
- `0080_test_rs_rt_paragraph.xml` — variabile `RT` (RS="" paragraph mode)
- `0083_test_nextfile_single_file_endfile_runs.xml` — `BEGINFILE`/`ENDFILE`

**Scope confine:** modifiche solo a `src/bin/diffrun.rs` (snellire `is_skip()`), 2 file `tests/cases/*.xml` (annotation), e `tests/diffrun_buckets.rs` (nuova attesa numerica). Nessuna modifica a `src/runner/**`.

**Out of scope esplicito:** implementazione di `RT`, `BEGINFILE`/`ENDFILE`; performance pass; locale; `gensub`/`mktime` (non presenti in alcun testcase attivo).

---

## Stato target

```
MATCH:               96
EXPECTED-DIVERGE:    10        # 8 + 2 nuove annotation
UNEXPECTED-DIVERGE:  0
SKIPPED:             3         # solo RT + BEGINFILE/ENDFILE
```

## Vocabolario `reason` (estensione)

I 6 reason esistenti restano invariati. Si aggiungono:

| reason | semantica | testcase |
|--------|-----------|----------|
| `bwk-missing-gawk-extension` | BWK awk non implementa la builtin (e.g. `and`/`or`/`xor`/`lshift`/`rshift`) — rawk sì | `0006_test_bitwise_and_time` |
| `bwk-different-rng` | rawk e BWK usano generatori PRNG diversi; l'oracolo è calibrato su rawk | `0012_test_gawk_manual_seven_random_numbers` |

---

## Fasi

### Phase 0 — Plan & workspace hygiene

- [ ] Verificare baseline verde: `bash scripts/checks.sh` → 8/8.
- [ ] Snapshot `diffrun` corrente: `MATCH 96 / EXPECTED 8 / UNEXPECTED 0 / SKIPPED 5`.
- [ ] Cleanup macOS forks (`rtk proxy find . -name '._*' -delete`).
- [ ] Commit `STEP21_PLAN.md`.

**Gate Phase 0:** `git status` pulito; tabella di stato in cima aggiornata.

### Phase 1 — Snellire `is_skip()` + annotare 2 testcase (RED → GREEN)

**Sotto-step TDD:**

1. **RED (test):** aggiornare `tests/diffrun_buckets.rs` cambiando le attese:
   - `MATCH: 96` → invariato
   - `EXPECTED-DIVERGE: 8` → `10`
   - `SKIPPED: 5` → `3`
   - `exit = 0` → invariato
   Lanciare `cargo test --test diffrun_buckets` → deve fallire con mismatch su EXPECTED e SKIPPED.

2. **GREEN (impl):**
   - In `src/bin/diffrun.rs::is_skip()`:
     - Rimuovere blocco `and(`/`or(`/`xor(`/`lshift(`/`rshift(` (linee ~89–96).
     - Rimuovere blocco `srand` (linee ~106–108).
   - Annotare `tests/cases/0006_test_bitwise_and_time.xml`: `<expected_divergence reason="bwk-missing-gawk-extension"/>`.
   - Annotare `tests/cases/0012_test_gawk_manual_seven_random_numbers.xml`: `<expected_divergence reason="bwk-different-rng"/>`.

3. **VERIFY:**
   - `cargo test --test diffrun_buckets` → verde.
   - `cargo test --test xml_runner_test` → ancora 109/109 (annotation ignorata dal runner XML).
   - `cargo run --release --bin diffrun -- tests/testsuite.xml` → output stabile su 3 run consecutivi con i valori target.

4. **COMMIT:** un solo commit `phase21.1(green): retire stale bitwise/srand SKIPs in diffrun`.

**Gate Phase 1:** test verde, suite preesistente invariata, `cargo clippy --all-targets -- -D warnings` zero.

### Phase 2 — Verify gate + diary

- [ ] `bash scripts/checks.sh` → 8/8 verdi (in particolare `check_diffrun_no_unexpected`).
- [ ] Diary in `diary/2026-MM-DD-rawk-step21-closure.md` con tabella before/after e SHA dei commit.
- [ ] Update memory file (`rawk_step21_state.md` + indice `MEMORY.md`) con stato CHIUSA.
- [ ] Commit `step21(diary): closure — stale SKIP cleanup` + push origin/master.

**Gate Phase 2:** push pushato, `git status` pulito, gate verdi su master.

---

## Out of scope (passi successivi candidati)

- **Step 22** — Implementazione `RT` (variabile dopo split su RS regex/paragraph-mode). 2 testcase coinvolti.
- **Step 23** — Implementazione `BEGINFILE`/`ENDFILE`. 1 testcase coinvolto.
- **Step 24** — Performance pass: rimozione `.clone()` su record/fs/printf path (post Phase 7 bytes migration).
- **Step 25** — Migrazione `test_gawk_manual_word_frequency` a iterazione deterministica per rimuovere l'annotation `non-deterministic-iteration-order`.

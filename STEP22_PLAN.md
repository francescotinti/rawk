# rawk — Step 22: chiusura SKIP stale `RT` (TDD strict)

> **Per l'agent che esegue:** SUB-SKILL CONSIGLIATA: `superpowers:test-driven-development`. Step in checkbox (`- [ ]`). Disciplina TDD strict (Red → Green → Verify → Commit). Una fase per sessione salvo richiesta esplicita.

---

## Stato di avanzamento (aggiornare a fine di ogni sessione)

| Fase | Stato | Commit di chiusura | Note |
|------|-------|--------------------|------|
| Phase 0 — Plan & hygiene | ✅ FATTO | `ea3725e` | STEP22_PLAN.md + macOS forks clean |
| Phase 1 — Rimozione skip `RT` + annotation 2 testcase | ✅ FATTO | `2933e86` | is_skip() snellito + 2 XML annotati + invariante MATCH+EXPECTED=108 |
| Phase 2 — Verify + diary | ✅ FATTO | *(questo commit)* | scripts/checks.sh 8/8 + `diary/2026-05-24-rawk-step22-closure.md` |

Legenda stato: ⏳ TODO · 🚧 IN CORSO · ✅ FATTO · ⚠️ PARZIALE · ❌ BLOCCATO

---

## Motivazione

**Why:** Dopo Step 21 `diffrun` riporta `MATCH 95 / EXPECTED-DIVERGE 11 / UNEXPECTED-DIVERGE 0 / SKIPPED 3`. Indagine: di quei 3 SKIP, **2 sono ormai stale** (gli ultimi `RT`):

- `0029_test_record_terminator_rt.xml` → marca skip per `RT`. Ma `RT` è **implementato** in rawk: `src/runner/mod.rs` linee 326, 357, 371, 405, 418 lo aggiornano sia per il path single-char RS, sia per il path regex RS, sia per il path paragraph (`RS=""`).
- `0080_test_rs_rt_paragraph.xml` → idem (RS="" paragraph mode). Stesso path già coperto.

Verifica manuale (rawk vs BWK awk):

```
$ printf "1A2A3" | ./target/release/rawk 'BEGIN { RS="A" } { print $0, "RT:", RT }'
1 RT: A
2 RT: A
3 RT: 

$ printf "1A2A3" | awk 'BEGIN { RS="A" } { print $0, "RT:", RT }'
1 RT: 
2 RT: 
3 RT: 
```

rawk segue l'oracolo (`expected_stdout`); BWK awk diverge perché **non implementa `RT`** (variabile sempre vuota). Stessa dinamica per il paragraph mode.

Lo SKIP residuo legittimo resta uno solo: `0083_test_nextfile_single_file_endfile_runs.xml` (`BEGINFILE`/`ENDFILE` non implementati in rawk — sarà oggetto di Step 23).

**Scope confine:** modifiche solo a `src/bin/diffrun.rs` (snellire `is_skip()`), 2 file `tests/cases/*.xml` (annotation), e `tests/diffrun_buckets.rs` (nuova attesa numerica). Nessuna modifica a `src/runner/**`.

**Out of scope esplicito:** implementazione di `BEGINFILE`/`ENDFILE` (Step 23); performance pass (Step 24); locale; `gensub`/`mktime`.

---

## Stato target

```
MATCH:               95
EXPECTED-DIVERGE:    13        # 11 + 2 nuove annotation
UNEXPECTED-DIVERGE:  0
SKIPPED:             1         # solo BEGINFILE/ENDFILE
```

Invariante stabile (somma — `MATCH` ed `EXPECTED-DIVERGE` oscillano per via di `0017_test_gawk_manual_word_frequency`):

```
MATCH + EXPECTED-DIVERGE = 108
UNEXPECTED-DIVERGE       = 0
SKIPPED                  = 1
exit                     = 0
```

## Vocabolario `reason` (nessuna estensione)

Riuso il `reason` già in vocabolario: `bwk-missing-gawk-extension`. Semantica: BWK awk non implementa una feature gawk-style. Si applica simmetricamente a builtin (bitwise) e variabili speciali (RT). Mantenere il vocabolario a 8 categorie evita frammentazione.

---

## Fasi

### Phase 0 — Plan & workspace hygiene

- [ ] Verificare baseline verde: `bash scripts/checks.sh` → 8/8.
- [ ] Snapshot `diffrun` corrente: `MATCH 95 / EXPECTED 11 / UNEXPECTED 0 / SKIPPED 3`.
- [ ] Cleanup macOS forks (`rtk proxy find . -name '._*' -not -path './target/*' -delete`).
- [ ] Commit `STEP22_PLAN.md`.

**Gate Phase 0:** `git status` pulito; tabella di stato in cima aggiornata.

### Phase 1 — Snellire `is_skip()` + annotare 2 testcase (RED → GREEN)

**Sotto-step TDD:**

1. **RED (test):** aggiornare `tests/diffrun_buckets.rs` cambiando le attese di `diffrun_step21_target_counts` → nuovo test `diffrun_step22_target_counts`:
   - `MATCH+EXPECTED: 106` → `108`
   - `SKIPPED: 3` → `1`
   - `UNEXPECTED: 0` → invariato
   - `exit = 0` → invariato

   Lanciare `cargo test --test diffrun_buckets --release -- --test-threads=1` → deve fallire con mismatch.

2. **GREEN (impl):**
   - In `src/bin/diffrun.rs::is_skip()`: rimuovere il blocco `if script.contains("RT") { return Some("uses gawk-only RT variable"); }`.
   - Aggiungere `<expected_divergence reason="bwk-missing-gawk-extension"/>` ai 2 file:
     - `tests/cases/0029_test_record_terminator_rt.xml`
     - `tests/cases/0080_test_rs_rt_paragraph.xml`

3. **VERIFY:** `cargo test --test diffrun_buckets --release -- --test-threads=1` → verde.

**Gate Phase 1:**
- `cargo test --test xml_runner_test --release` → 109/109 (nessuna regressione interna).
- `cargo run --bin diffrun --release` → bucket: `MATCH+EXPECTED=108`, `UNEXPECTED=0`, `SKIPPED=1`, exit 0.
- Commit `phase22.1(green): retire stale RT SKIPs in diffrun`.

### Phase 2 — Verify finale + diary + push

- [ ] `bash scripts/checks.sh` → 8/8.
- [ ] Cleanup macOS forks.
- [ ] Scrivere `diary/2026-05-24-rawk-step22-closure.md` con: prima/dopo bucket, diff riassuntiva, candidati Step 23+.
- [ ] Aggiornare tabella di stato in cima a `STEP22_PLAN.md` con stati ✅ FATTO e commit-hash.
- [ ] Commit `step22(diary): closure — stale SKIP cleanup RT`.
- [ ] `git push origin master`.

**Gate Phase 2:** working tree pulito; tutti i commit di Step 22 su `origin/master`; memoria aggiornata (`rawk_step22_state.md` + entry in `MEMORY.md`).

---

## Candidati Step 23+

- **Step 23** — Implementazione `BEGINFILE`/`ENDFILE` (1 testcase rimanente). Non è uno SKIP stale: richiede nuovo codice in `src/runner/mod.rs`.
- **Step 24** — Performance pass: rimozione `.clone()` post Phase 7.
- **Step 25** — Migrazione `0017_test_gawk_manual_word_frequency` a iterazione deterministica (ridurre l'oscillazione `MATCH/EXPECTED`).

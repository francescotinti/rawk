# rawk — Step 23: chiusura SKIP stale `BEGINFILE`/`ENDFILE` (TDD strict)

> **Per l'agent che esegue:** SUB-SKILL CONSIGLIATA: `superpowers:test-driven-development`. Step in checkbox (`- [ ]`). Disciplina TDD strict (Red → Green → Verify → Commit). Una fase per sessione salvo richiesta esplicita.

---

## Stato di avanzamento (aggiornare a fine di ogni sessione)

| Fase | Stato | Commit di chiusura | Note |
|------|-------|--------------------|------|
| Phase 0 — Plan & hygiene | ✅ FATTO | `2abe0fc` | STEP23_PLAN.md + macOS forks clean |
| Phase 1 — Rimozione skip `BEGINFILE/ENDFILE` + annotation 1 testcase | ✅ FATTO | `fa96efe` | `is_skip()` ridotta a `None` no-op + 0083 XML annotato + invariante `MATCH+EXPECTED=109`, `SKIPPED=0` |
| Phase 2 — Verify + diary | ✅ FATTO | *(questo commit)* | `scripts/checks.sh` 8/8 + `diary/2026-05-24-rawk-step23-closure.md` |

Legenda stato: ⏳ TODO · 🚧 IN CORSO · ✅ FATTO · ⚠️ PARZIALE · ❌ BLOCCATO

---

## Motivazione

**Why:** Dopo Step 22 `diffrun` riporta `MATCH 95 / EXPECTED-DIVERGE 13 / UNEXPECTED-DIVERGE 0 / SKIPPED 1`. L'unico SKIP residuo è `0083_test_nextfile_single_file_endfile_runs.xml`, marcato come `uses gawk-only BEGINFILE/ENDFILE`. Indagine: lo SKIP è **stale**.

**Stato reale del runner:**
- `BEGINFILE`/`ENDFILE` sono già parsati come `Pattern::BeginFile`/`Pattern::EndFile` (vedi [src/parser.rs:101-104](src/parser.rs#L101-L104), [src/awk.pest:23](src/awk.pest#L23), [src/ast.rs:112-113](src/ast.rs#L112-L113)).
- I blocchi `BeginFile`/`EndFile` sono già eseguiti dal runner: vedi [src/runner/mod.rs:166-206](src/runner/mod.rs#L166-L206) (chiamate `execute_special_blocks(...SpecialBlock::BeginFile|EndFile)` sia all'apertura sia alla chiusura di ogni file argv, inclusa la chiusura forzata da `nextfile`).
- `nextfile` è uno statement implementato e funzionante (Step 6 storico).

**Verifica manuale (rawk vs BWK awk)** sul test case 0083:

```
$ printf "a\nb\nc\nd" | ./target/release/rawk \
    'NR == 2 { nextfile } { print NR ":" $0 } ENDFILE { print "EF" } END { print "END:" NR }'
1:a
EF
END:2

$ printf "a\nb\nc\nd" | awk \
    'NR == 2 { nextfile } { print NR ":" $0 } ENDFILE { print "EF" } END { print "END:" NR }'
1:a
END:2
```

rawk produce l'oracolo (`expected_stdout` del testcase) **byte-per-byte**. BWK awk diverge perché **non implementa `ENDFILE`** (lo tratta come pattern letterale che non match-a mai, quindi non stampa "EF"). Stessa famiglia di divergenza già catalogata come `bwk-missing-gawk-extension` per bitwise (Step 21) e `RT` (Step 22).

`cargo test --test xml_runner_test --release` passa già (suite XML interna verde con questo testcase incluso). L'unico ostacolo a rimuovere lo SKIP è il guard euristico in [src/bin/diffrun.rs:92-94](src/bin/diffrun.rs#L92-L94).

**Scope confine:** modifiche solo a `src/bin/diffrun.rs` (snellire `is_skip()`), 1 file `tests/cases/0083_test_nextfile_single_file_endfile_runs.xml` (annotation), e `tests/diffrun_buckets.rs` (nuova attesa numerica). **Nessuna modifica a `src/runner/**`, `src/parser*`, `src/ast.rs`, `src/awk.pest`.**

**Out of scope esplicito:** performance pass (Step 24); migrazione `0017_test_gawk_manual_word_frequency` (Step 25); test multi-file argv (richiederebbe infrastruttura testharness diversa — `xml_runner_test` oggi inietta solo `stdin`); `BEGINFILE` con `ERRNO` (gawk extension non in scope rawk); locale; `gensub`/`mktime`.

---

## Stato target

```
MATCH:               95
EXPECTED-DIVERGE:    14        # 13 + 1 nuova annotation
UNEXPECTED-DIVERGE:  0
SKIPPED:             0         # primo step a zero SKIP
```

Invariante stabile (somma — `MATCH` ed `EXPECTED-DIVERGE` oscillano per via di `0017_test_gawk_manual_word_frequency`):

```
MATCH + EXPECTED-DIVERGE = 109
UNEXPECTED-DIVERGE       = 0
SKIPPED                  = 0
exit                     = 0
```

**Nota di rilievo:** Step 23 chiude il ciclo di "stale SKIP retirement" iniziato con Step 21. Dopo questo commit, `is_skip()` non avrà più euristiche e diventerà un no-op (`None`), oppure verrà rimossa la funzione del tutto se non ha più caller path utili. **Decisione di design:** mantenere la funzione `is_skip()` ridotta a `None` (vuota) per facilitare l'aggiunta futura di SKIP genuini senza riarrangiare il dispatch; alternativa è rimuoverla e inline-are. Sceglierò in Phase 1 in base alla pulizia del diff.

## Vocabolario `reason` (nessuna estensione)

Riuso il `reason` già in vocabolario: `bwk-missing-gawk-extension`. Semantica: BWK awk non implementa una feature gawk-style. Si applica simmetricamente a builtin (bitwise), variabili speciali (RT), e ora pattern speciali (BEGINFILE/ENDFILE). Vocabolario `reason` resta a 8 categorie.

---

## Fasi

### Phase 0 — Plan & workspace hygiene

- [x] Verificare baseline verde: `bash scripts/checks.sh` → 8/8.
- [x] Snapshot `diffrun` corrente: `MATCH 95 / EXPECTED 13 / UNEXPECTED 0 / SKIPPED 1`.
- [x] Cleanup macOS forks (`rtk proxy find . -name '._*' -not -path './target/*' -delete`).
- [x] Verifica manuale che rawk produce l'oracolo del testcase 0083 byte-per-byte.
- [x] Verifica manuale che BWK awk diverge (non stampa "EF").
- [x] Conferma che `0083_test_nextfile_single_file_endfile_runs.xml` è l'**unico** testcase del corpus che usa `BEGINFILE`/`ENDFILE` (`grep -l "BEGINFILE\\|ENDFILE" tests/cases/*.xml` → 1 hit).
- [ ] Commit `STEP23_PLAN.md`.

**Gate Phase 0:** `git status` pulito; tabella di stato in cima aggiornata; checks 8/8 ancora verdi dopo `STEP23_PLAN.md` aggiunto.

### Phase 1 — Snellire `is_skip()` + annotare 1 testcase (RED → GREEN)

**Sotto-step TDD:**

1. **RED (test):** in `tests/diffrun_buckets.rs` rinominare/duplicare `diffrun_step22_target_counts` → `diffrun_step23_target_counts` aggiornando le attese:
   - `MATCH+EXPECTED: 108` → `109`
   - `SKIPPED: 1` → `0`
   - `UNEXPECTED: 0` → invariato
   - `exit = 0` → invariato

   Lanciare `cargo test --test diffrun_buckets --release -- --test-threads=1` → deve fallire con mismatch su SKIPPED (atteso 0, trovato 1) e MATCH+EXPECTED (atteso 109, trovato 108).

2. **GREEN (impl):**
   - In `src/bin/diffrun.rs::is_skip()`: rimuovere il blocco `if script.contains("BEGINFILE") || script.contains("ENDFILE") { return Some("uses gawk-only BEGINFILE/ENDFILE"); }`. Se questa era l'ultima euristica, valutare se ridurre `is_skip()` a `pub fn is_skip(_script: &str) -> Option<&'static str> { None }` o rimuovere la funzione (vedi note di design sopra).
   - Aggiungere `<expected_divergence reason="bwk-missing-gawk-extension"/>` al file `tests/cases/0083_test_nextfile_single_file_endfile_runs.xml`.

3. **VERIFY:** `cargo test --test diffrun_buckets --release -- --test-threads=1` → verde.

**Gate Phase 1:**
- `cargo test --test xml_runner_test --release` → conteggio interno invariato (nessuna regressione).
- `cargo run --bin diffrun --release` → bucket: `MATCH+EXPECTED=109`, `UNEXPECTED=0`, `SKIPPED=0`, exit 0.
- `bash scripts/checks.sh` → 8/8.
- Commit `phase23.1(green): retire stale BEGINFILE/ENDFILE SKIPs in diffrun`.

### Phase 2 — Verify finale + diary + push

- [ ] `bash scripts/checks.sh` → 8/8.
- [ ] Cleanup macOS forks.
- [ ] Scrivere `diary/2026-05-24-rawk-step23-closure.md` con: prima/dopo bucket, diff riassuntiva, riflessione sulla chiusura della "stale SKIP retirement series" (Step 21→22→23), candidati Step 24+.
- [ ] Aggiornare tabella di stato in cima a `STEP23_PLAN.md` con stati ✅ FATTO e commit-hash.
- [ ] Commit `step23(diary): closure — stale SKIP cleanup BEGINFILE/ENDFILE`.
- [ ] `git push origin master`.

**Gate Phase 2:** working tree pulito; tutti i commit di Step 23 su `origin/master`; memoria aggiornata (`rawk_step23_state.md` + entry in `MEMORY.md`).

---

## Candidati Step 24+

- **Step 24** — Performance pass: rimozione `.clone()` post Phase 7 (era pianificato come Step 24 in chiusura Step 22).
- **Step 25** — Migrazione `0017_test_gawk_manual_word_frequency` a iterazione deterministica (ridurre l'oscillazione `MATCH/EXPECTED`).
- **Step 26** — Test harness multi-file argv: estendere `xml_runner_test` per accettare `<argv>` multipli e abilitare un secondo testcase BEGINFILE/ENDFILE che dimostri il pattern su 2+ file (oggi out of scope perché manca l'infrastruttura test, non il runtime).

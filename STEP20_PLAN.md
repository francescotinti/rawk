# rawk — Step 20: diffrun annotated divergence (TDD strict)

> **Per l'agent che esegue:** SUB-SKILL CONSIGLIATA: `superpowers:test-driven-development`. Step in checkbox (`- [ ]`). Disciplina TDD strict (Red → Green → Verify → Commit) anche per i cleanup. Una fase per sessione salvo richiesta esplicita.

---

## Stato di avanzamento (aggiornare a fine di ogni sessione)

| Fase | Stato | Commit di chiusura | Note |
|------|-------|--------------------|------|
| Phase 0 — Plan & hygiene | ✅ FATTO | (vedi commit precedente) | Plan file + macOS forks clean |
| Phase 1 — Schema + diffrun bucket | ✅ FATTO | (vedi commit Phase 1) | `<expected_divergence reason="…"/>` + 4-bucket output, exit 1 se UNEXPECTED > 0 |
| Phase 2 — Annotate 9 testcase noti | ✅ FATTO | (vedi commit Phase 2) | 8 stable + 1 non-deterministic (word_frequency); UNEXPECTED-DIVERGE = 0; exit 0 |
| Phase 3 — `checks.sh` gate | ✅ FATTO | (vedi commit Phase 3) | `check_diffrun_no_unexpected`; verificato red→green su annotation rimossa/ripristinata |

Legenda stato: ⏳ TODO · 🚧 IN CORSO · ✅ FATTO · ⚠️ PARZIALE · ❌ BLOCCATO

---

## Motivazione

**Why:** `diffrun` confronta `rawk` vs `/usr/bin/awk` (BWK 20200816 su macOS) e marca come **DIVERGE** ogni differenza. Ma `xml_runner_test` passa **109/109** contro `expected_stdout`: gli 8 DIVERGE attuali sono testcase dove **rawk segue `expected_stdout` (POSIX) ma BWK awk diverge**. Senza annotation, diffrun confonde divergenze intenzionali (oracoli) e regressioni vere — rendendo il segnale inutile come gate CI.

**Scope confine:** modifiche solo a `tests/cases/*.xml` (annotation), `src/bin/diffrun.rs` (schema+bucketing), e `scripts/checks.sh` (gate). Nessuna modifica a `src/runner/**`, `src/types.rs`, runtime AWK.

**Out of scope esplicito:** chiusura dei 5 SKIPPED (gawk-extensions), performance pass, locale, gensub.

---

## Fasi

### Phase 0 — Plan & workspace hygiene

- [x] Verificare baseline verde: `cargo test --test xml_runner_test` → 109/109.
- [x] Snapshot delle 8 DIVERGE attuali via `cargo run --release --bin diffrun -- tests/testsuite.xml` → `96 MATCH / 8 DIVERGE / 5 SKIPPED`.
- [x] Cleanup macOS forks (`rtk proxy find . -name '._*' -delete`).
- [ ] Commit `STEP20_PLAN.md`.

**Gate Phase 0:** `git status` pulito; tabella di stato in cima aggiornata.

### Phase 1 — Schema + diffrun bucket (RED → GREEN)

**Schema XML (additivo, opzionale):**

```xml
<testcase name="test_xxx">
    <awk>…</awk>
    <expected_stdout match="exact">…</expected_stdout>
    <expected_divergence reason="bwk-stops-on-warning"/>  <!-- nuovo, opzionale -->
</testcase>
```

**Categorie di `reason` (vocabolario fisso):**
- `bwk-stops-on-warning` — BWK awk termina su warning, rawk continua come da `expected_stdout` (es. div/0, unknown function).
- `bwk-truncates-at-nul` — BWK awk usa C strings (NUL-terminated), rawk usa byte (Phase 7 win).
- `bwk-different-number-format` — BWK awk usa OFMT diverso (es. notazione decimale per huge numbers).
- `bwk-escape-handling` — BWK awk fa escape handling diverso (es. drop unknown backslash escapes).
- `bwk-parser-quirk` — divergenza di parsing (es. spazio tra nome funzione e `(`).

**diffrun output target (esempio):**

```
=== rawk differential test report ===
Reference awk: /usr/bin/awk (awk version 20200816)
Total testcase: 109
  MATCH:                104
  EXPECTED-DIVERGE:       0
  UNEXPECTED-DIVERGE:     0
  SKIPPED:                5
```

(Numeri attesi DOPO Phase 2: `96 + 8 + 0 + 5 = 109` ⇒ `MATCH 96 / EXPECTED 8 / UNEXPECTED 0 / SKIPPED 5`. Nota: la riga MATCH resta 96 perché EXPECTED-DIVERGE è un bucket separato — la riallocazione avviene da "DIVERGE 8" a "EXPECTED-DIVERGE 8". Il totale 109 si compone come `MATCH + EXPECTED-DIVERGE + UNEXPECTED-DIVERGE + SKIPPED`.)

**Exit code:** `0` sse `UNEXPECTED-DIVERGE == 0` (gate CI-friendly).

**Sotto-step TDD:**

1. **RED (shell test):** aggiungere `tests/diffrun_buckets.rs` che invoca `target/release/diffrun` e asserisce le 4 righe del summary + presenza di una sezione `== EXPECTED DIVERGENCES ==` con un campione fittizio. Deve **fallire** sul codice attuale.
2. **GREEN (schema):** estendere `TestCase` in `src/bin/diffrun.rs` con `expected_divergence: Option<ExpectedDivergence>` + struct `ExpectedDivergence { reason: String }` con `#[serde(rename = "@reason")]`. Aggiornare main loop: se DIVERGE && `expected_divergence.is_some()` → push in `expected_diverge_cases` invece di `diverge_cases`. Aggiornare output e exit code.
3. **VERIFY:** `cargo test --test diffrun_buckets` verde + `cargo test --test xml_runner_test` ancora 109/109.
4. **COMMIT:** un solo commit `phase20.1(green): diffrun expected_divergence bucket`.

**Gate Phase 1:** test nuovo verde, suite preesistente invariata, `cargo clippy --all-targets -- -D warnings` zero.

### Phase 2 — Annotare gli 8 testcase noti

Per ciascuno dei testcase elencati sotto, aggiungere `<expected_divergence reason="…"/>` con categoria appropriata:

| Test | Reason |
|------|--------|
| `0003_test_type_coercion.xml` | `bwk-stops-on-warning` (verifica perché BWK fallisce — se è warning su coercion) |
| `0037_test_concat_func_call_disambig.xml` | `bwk-parser-quirk` (spazio prima di `(`) |
| `0064_test_escape_octal_zero.xml` | `bwk-truncates-at-nul` |
| `0066_test_escape_unknown_preserved.xml` | `bwk-escape-handling` |
| `0085_test_nextfile_in_user_function.xml` | `bwk-stops-on-warning` (nextfile in funzione) |
| `0107_test_print_huge_uses_scientific.xml` | `bwk-different-number-format` |
| `0108_test_div_by_zero_warning_continues.xml` | `bwk-stops-on-warning` |
| `0109_test_unknown_function_warning_continues.xml` | `bwk-stops-on-warning` |

**Sotto-step TDD:**

1. **RED:** dopo Phase 1, diffrun deve mostrare `EXPECTED-DIVERGE: 0 / UNEXPECTED-DIVERGE: 8`.
2. **GREEN:** annotare i file → `EXPECTED-DIVERGE: 8 / UNEXPECTED-DIVERGE: 0`.
3. **VERIFY:** `cargo test --test xml_runner_test` ancora 109/109 (l'annotation è ignorata da xml_runner_test).
4. **COMMIT:** un solo commit `phase20.2(green): annotate 8 expected divergences`.

**Gate Phase 2:** `diffrun` exit code = 0.

### Phase 3 — `checks.sh` gate

Aggiungere in `scripts/checks.sh` un check `check_diffrun_no_unexpected`:

```sh
check_diffrun_no_unexpected() {
    local output
    output=$(cargo run --release --bin diffrun -- tests/testsuite.xml 2>&1)
    local unexpected
    unexpected=$(echo "$output" | grep -E '^\s+UNEXPECTED-DIVERGE:' | awk '{print $2}')
    if [ "$unexpected" != "0" ]; then
        echo "FAIL: $unexpected unexpected divergences (vedi diffrun output)"
        echo "$output"
        return 1
    fi
    echo "OK: diffrun unexpected divergences = 0"
    return 0
}
```

**Sotto-step:**

1. **RED:** test della funzione (mocked output) o invocazione diretta — deve fallire se simuliamo `UNEXPECTED-DIVERGE: 1`.
2. **GREEN:** integrare in main `checks.sh`; eseguire `bash scripts/checks.sh` localmente → 8/8 OK.
3. **COMMIT:** `phase20.3(green): scripts/checks.sh diffrun gate`.

**Gate Phase 3:** `bash scripts/checks.sh` 8/8.

---

## Chiusura Step 20

Dopo Phase 3:
- Diary di chiusura in `diary/2026-MM-DD-rawk-step20-closure.md` con tabella before/after, commits SHA, e nota sul significato del nuovo gate.
- Aggiornare memory `rawk_phase7_state.md` (o creare `rawk_step20_state.md`) con stato CHIUSA.
- Push origin/master.

# rawk — Step 22 closure (2026-05-24)

**Tema:** ritiro stale SKIP per la variabile `RT` in `diffrun`.

## Riassunto in una riga

Lo SKIP `script.contains("RT")` era stale: `RT` è implementato in rawk dal Phase 7. Promossi 2 testcase a EXPECTED-DIVERGE (`bwk-missing-gawk-extension`) — BWK awk non implementa `RT`. Bucket finale: `MATCH 95 / EXPECTED 13 / UNEXPECTED 0 / SKIPPED 1`.

## Stato prima / dopo

| Metric                       | Pre-Step 22 | Post-Step 22 |
|------------------------------|-------------|--------------|
| `MATCH`                      | 95          | 95           |
| `EXPECTED-DIVERGE`           | 11          | 13           |
| `UNEXPECTED-DIVERGE`         | 0           | 0            |
| `SKIPPED`                    | 3           | 1            |
| `MATCH + EXPECTED` (invar.)  | 106         | 108          |
| `scripts/checks.sh` gate     | 8/8         | 8/8          |

Invariante numerico stabile (5 run consecutivi): `(m+e, u, s) = (108, 0, 1)`, `exit=0`. L'oscillazione su `MATCH/EXPECTED` resta confinata a `0017_test_gawk_manual_word_frequency` (`non-deterministic-iteration-order`).

## Commit (3, da pushare)

1. `ea3725e` — `phase22.0: step 22 plan — retire stale RT SKIPs (TDD strict)`
2. `2933e86` — `phase22.1(green): retire stale RT SKIPs in diffrun`
3. *(questo commit)* — `step22(diary): closure — stale SKIP cleanup RT`

## Diff riassuntiva

- `src/bin/diffrun.rs::is_skip()`: eliminato il blocco `if script.contains("RT") { return Some("uses gawk-only RT variable"); }` (3 righe).
- `tests/cases/0029_test_record_terminator_rt.xml`: aggiunto `<expected_divergence reason="bwk-missing-gawk-extension"/>`.
- `tests/cases/0080_test_rs_rt_paragraph.xml`: aggiunto `<expected_divergence reason="bwk-missing-gawk-extension"/>`.
- `tests/diffrun_buckets.rs`: rinominato `diffrun_step21_target_counts` → `diffrun_step22_target_counts` e aggiornata l'attesa a `(108, 0, 1)`.

Nessuna modifica a `src/runner/**`.

## Diagnosi della divergenza BWK vs rawk

```
$ printf "1A2A3" | ./target/release/rawk 'BEGIN{RS="A"}{print $0,"RT:",RT}'
1 RT: A         # rawk popola RT con il match
2 RT: A
3 RT:           # ultimo record: RT vuoto (nessun terminatore)

$ printf "1A2A3" | awk 'BEGIN{RS="A"}{print $0,"RT:",RT}'
1 RT:           # BWK: RT sempre vuoto (variabile non gestita)
2 RT:
3 RT:
```

`RT` è una gawk-style extension non implementata in BWK awk. rawk segue l'oracolo (la `expected_stdout` del testcase), quindi la divergenza è attesa.

## Vocabolario `reason` (invariato)

Riusato `bwk-missing-gawk-extension` (già introdotto in Step 21 per bitwise). Mantiene il vocabolario a 8 categorie; nessuna frammentazione.

## SKIPPED rimasti (1)

| File | Feature |
|------|---------|
| `tests/cases/0083_test_nextfile_single_file_endfile_runs.xml` | `BEGINFILE` / `ENDFILE` (non implementate in rawk) |

Questo è l'unico SKIP **legittimo**: feature realmente assente in rawk. Sarà oggetto di Step 23 (implementazione, non cleanup).

## Lezioni

- Lo Step 22 conferma il pattern di Step 21: dopo Phase 7 la maggior parte degli SKIP heuristici sono diventati stale. Il workflow è semplice e a basso rischio: verifica manuale rawk vs BWK → annotation XML → invariante in `diffrun_buckets.rs`.
- La separazione "SKIP = feature mancante" vs "EXPECTED-DIVERGE = comportamento divergente noto" si conferma utile: ora `SKIPPED=1` è una metrica perfettamente leggibile.
- Riusare il reason esistente (`bwk-missing-gawk-extension`) invece di crearne uno nuovo per `RT` evita di gonfiare il vocabolario; la semantica regge.

## Candidati Step 23+

- **Step 23** — Implementare `BEGINFILE`/`ENDFILE` in `src/runner/mod.rs`. Sarà il primo step "non-cleanup" della serie: richiede nuovo codice nel runner. 1 testcase target.
- **Step 24** — Performance pass: rimuovere `.clone()` accumulati post Phase 7 (lossy bytes path, output pipeline).
- **Step 25** — Determinismo: migrare `0017_test_gawk_manual_word_frequency` a un'iterazione ordinata, ridurre l'oscillazione `MATCH/EXPECTED` (oggi assertabile solo come somma).

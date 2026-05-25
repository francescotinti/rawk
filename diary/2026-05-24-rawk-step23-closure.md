# rawk — Step 23 closure (2026-05-24)

**Tema:** ritiro stale SKIP per `BEGINFILE`/`ENDFILE` in `diffrun`. Chiusura della serie *stale SKIP retirement* iniziata con Step 21.

## Riassunto in una riga

Lo SKIP `script.contains("BEGINFILE") || script.contains("ENDFILE")` era stale: i pattern speciali sono già implementati end-to-end (pest grammar → AST → parser → runner con `execute_special_blocks` su open/close/nextfile). Promosso 1 testcase a EXPECTED-DIVERGE (`bwk-missing-gawk-extension`) — BWK awk non implementa `ENDFILE`. `is_skip()` ridotta a `None` no-op: la serie 21→22→23 ha rimosso ogni euristica blanket-skip. Bucket finale: `MATCH 95 / EXPECTED 14 / UNEXPECTED 0 / SKIPPED 0` — **primo step a zero SKIP**.

## Stato prima / dopo

| Metric                       | Pre-Step 23 | Post-Step 23 |
|------------------------------|-------------|--------------|
| `MATCH`                      | 95          | 95           |
| `EXPECTED-DIVERGE`           | 13          | 14           |
| `UNEXPECTED-DIVERGE`         | 0           | 0            |
| `SKIPPED`                    | 1           | **0**        |
| `MATCH + EXPECTED` (invar.)  | 108         | 109          |
| `scripts/checks.sh` gate     | 8/8         | 8/8          |

Invariante numerico stabile: `(m+e, u, s) = (109, 0, 0)`, `exit=0`. L'oscillazione su `MATCH/EXPECTED` resta confinata a `0017_test_gawk_manual_word_frequency` (`non-deterministic-iteration-order`) — singolarmente possono valere `(95, 14)` o `(96, 13)`; la somma è stabile.

## Commit (3)

1. `2abe0fc` — `phase23.0: step 23 plan — retire stale BEGINFILE/ENDFILE SKIPs (TDD strict)`
2. `fa96efe` — `phase23.1(green): retire stale BEGINFILE/ENDFILE SKIPs in diffrun`
3. *(questo commit)* — `step23(diary): closure — stale SKIP cleanup BEGINFILE/ENDFILE`

## Diff riassuntiva

- `src/bin/diffrun.rs::is_skip()`: la funzione è stata **ridotta a `None` no-op**. Rimossi tutti e tre i blocchi euristici (systime/strftime, gensub/mktime, BEGINFILE/ENDFILE) — di cui solo l'ultimo era ancora attivo sul corpus corrente. Mantenuta la firma `fn is_skip(_case: &TestCase) -> Option<&'static str>` e il caller per facilitare future estensioni (SKIP genuini per feature realmente assenti).
- `tests/cases/0083_test_nextfile_single_file_endfile_runs.xml`: aggiunto `<expected_divergence reason="bwk-missing-gawk-extension"/>`.
- `tests/diffrun_buckets.rs`: rinominato `diffrun_step22_target_counts` → `diffrun_step23_target_counts` con attesa `(109, 0, 0)`.

Nessuna modifica a `src/runner/**`, `src/parser*`, `src/ast.rs`, `src/awk.pest`.

## Diagnosi della divergenza BWK vs rawk

```
$ printf "a\nb\nc\nd" | ./target/release/rawk \
    'NR == 2 { nextfile } { print NR ":" $0 } ENDFILE { print "EF" } END { print "END:" NR }'
1:a
EF        # rawk esegue ENDFILE quando il file si chiude (anche per nextfile)
END:2

$ printf "a\nb\nc\nd" | awk \
    'NR == 2 { nextfile } { print NR ":" $0 } ENDFILE { print "EF" } END { print "END:" NR }'
1:a
END:2     # BWK: ENDFILE non riconosciuto, blocco mai eseguito
```

`BEGINFILE`/`ENDFILE` sono pattern speciali gawk-style non implementati in BWK awk (lì sono trattati come identificatori che non match-ano mai alcun record). rawk segue l'oracolo (`expected_stdout` del testcase) byte-per-byte; la divergenza è attesa e annotata.

## Vocabolario `reason` (invariato)

Riusato `bwk-missing-gawk-extension` (introdotto in Step 21 per bitwise, esteso in Step 22 a `RT`). Vocabolario `reason` resta a 8 categorie. La semantica regge per builtin, variabili speciali, e ora pattern speciali — è la categoria "naturale" per ogni gawk extension assente in BWK.

## SKIPPED rimasti (0)

Per la prima volta nella storia di `diffrun`, **zero testcase vengono saltati**. Ogni testcase del corpus viene eseguito sia da rawk sia da BWK awk, e ogni divergenza è esplicitamente annotata o categorizzata come regressione (`UNEXPECTED-DIVERGE`).

## Chiusura della "stale SKIP retirement series" (21→22→23)

| Step | Heuristic rimossa             | Testcase liberati | SKIPPED  | EXPECTED |
|------|-------------------------------|-------------------|----------|----------|
| 21   | `bitwise`, `srand`            | 2                 | 5 → 3    | 8 → 11   |
| 22   | `RT`                          | 2                 | 3 → 1    | 11 → 13  |
| 23   | `BEGINFILE`/`ENDFILE`         | 1                 | 1 → **0**| 13 → 14  |

Totale: 5 testcase promossi da blanket-skip a EXPECTED-DIVERGE, con annotation per-caso. Tutte le heuristic euristiche stale sono state ritirate. `is_skip()` è ora `None`-no-op: pronta ad accogliere SKIP *genuini* (feature realmente non implementata) senza che debba essere riprogettata.

**Pattern metodologico confermato** (3 ripetizioni):
1. Snapshot baseline + check 8/8 verdi.
2. Verifica manuale rawk vs BWK sul testcase target (oracolo + diff).
3. RED: nuovo `diffrun_stepN_target_counts` con somma `MATCH+EXPECTED` aggiornata.
4. GREEN: rimozione heuristic + annotation XML.
5. Verify (`--test-threads=1` per evitare la collisione filesystem sui testcase di redirect — vedi nota qui sotto).
6. Diary + push.

## Nota di processo: collisione filesystem dei testcase di redirect

Lanciando `cargo test --test diffrun_buckets` senza `--test-threads=1`, 3 testcase falliscono in modo non-deterministico:

- `test_redirect_append_after_close`
- `test_redirect_overwrite_after_close_truncates`
- `test_printf_with_redirect`

I tre testcase scrivono su file condivisi e, quando 3 istanze di `diffrun` girano in parallelo (3 funzioni `#[test]` in `diffrun_buckets.rs`), le scritture si sovrappongono → `UNEXPECTED-DIVERGE` spuri. Con `--test-threads=1` tutto verde. Questa è una nota di workflow, non un bug di rawk; sarà oggetto di un futuro step di hardening del testcase corpus (output filename per-PID, oppure isolation via tempdir). `scripts/checks.sh` invoca già `cargo test` con flag corretti, quindi la 8/8 gate non è affetta.

## Lezioni

- La chiusura della serie 21→22→23 dimostra il valore dell'annotation esplicita rispetto allo skip euristico: ogni divergenza ora ha un *reason* leggibile, e la metrica `SKIPPED` è diventata semanticamente pura ("feature non implementata in rawk").
- Mantenere `is_skip()` come funzione `None`-no-op (anziché rimuoverla) costa pochissimo e preserva il dispatch point: il giorno in cui un testcase invocherà una feature realmente non implementata in rawk (es. `getline` da pipe, in scope per Step 26+), basterà aggiungere una `if` senza riarrangiare il caller.
- Lo Step 23 era stato originariamente pensato (in chiusura Step 22) come "implementazione di BEGINFILE/ENDFILE nel runner". L'audit Phase 0 ha mostrato che il runner *li implementa già*. La lezione: prima di pianificare implementazione, verificare lo stato reale del codice — la coperta dello SKIP nasconde feature già funzionanti.

## Candidati Step 24+

- **Step 24** — Performance pass: rimozione `.clone()` accumulati post Phase 7 (lossy bytes path, output pipeline). Era pianificato in chiusura Step 22 e ancora valido.
- **Step 25** — Migrazione `0017_test_gawk_manual_word_frequency` a iterazione ordinata sulle hash table (per-record `BEGIN` setup + sort), così da poter assertare separatamente `MATCH=96 EXPECTED=13` invece della sola somma.
- **Step 26** — Test harness multi-file argv: estendere `xml_runner_test` per accettare `<argv>` multipli (oggi solo `<stdin>`). Sbloccherebbe un secondo testcase BEGINFILE/ENDFILE che dimostra il pattern su 2+ file, oltre a coprire `FILENAME` switching e `nextfile` cross-file.
- **Step 27** — Hardening del corpus redirect (workaround alla collisione filesystem documentata sopra): introdurre tempdir per i testcase con `>`, `>>`, `|` redirects.

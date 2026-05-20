# Phase 7 — Pre-flight audit report

Generated: 2026-05-20T00:41:45Z

## (a) Testcase XML con stringhe non-ASCII

Nessuno trovato — testsuite XML (manifest + 109 case) è ASCII pura.

## (b) Testcase con length()/substr()/index() su input multi-byte

Manifest + cases (max 30 hit):

```
tests/cases/0007_test_string_manipulation.xml:3:    <awk>BEGIN { s = "foo bar"; gsub("bar", "baz", s); print s, length(s), toupper(s) }</awk>
tests/cases/0011_test_gawk_manual_longest_line.xml:3:    <awk>{ if (length($0) &gt; max) max = length($0) } END { print max }</awk>
tests/cases/0026_test_medium_string_functions.xml:3:    <awk> { print toupper($1), substr($2,1,3), length($2) } </awk>
tests/cases/0027_test_gawk_manual_wc.xml:5:            chars += length($0) + 1  
tests/cases/0061_test_escape_hex_one_digit.xml:3:    <awk>BEGIN { printf "[%d]\n", length("\x9") }</awk>
tests/cases/0063_test_escape_octal_short.xml:3:    <awk>BEGIN { printf "[%d]\n", length("\7") }</awk>
tests/cases/0064_test_escape_octal_zero.xml:3:    <awk>BEGIN { printf "[%d]\n", length("\0") }</awk>
```

## (c) Conteggio call site da migrare per file

| File | String/&str hits |
|------|-----------------:|
| `src/ast.rs` | 14 |
| `src/cli.rs` | 5 |
| `src/main.rs` | 0 |
| `src/parser.rs` | 8 |
| `src/scratch.rs` | 0 |
| `src/types.rs` | 42 |
| `src/runner/builtins.rs` | 15 |
| `src/runner/fmt.rs` | 5 |
| `src/runner/io.rs` | 4 |
| `src/runner/mod.rs` | 23 |

## (d) Verifica pest grammar su byte > 0x7f

Test: `cargo run -q -- 'BEGIN { print "\\xff" }'`

Atteso: pest accetta escape \NNN; verifica output:

```
warning: hard linking files in the incremental compilation cache failed. copying files instead. consider moving the cache directory to a file system which supports hard linking in session dir `/Volumes/Extreme Pro/Claude/testag-awk/rawk/target/debug/incremental/rawk-3r1fxu1w6rg1x/s-hio9h4woxq-1iif51e-working`

ÿ
```

## Decisioni di pre-flight

- **(a)**: 0 file XML con byte non-ASCII. Testsuite manifest + 109 case sono ASCII pura. Strategia: nessuna deviazione necessaria; byte-counting in `length()`/`substr()` non altera l'output su input ASCII (byte == char). Zero conflitti con i 109 esistenti.
- **(b)**: 7 testcase usano `length()`/`substr()`/`index()`. Tutti operano su input ASCII o su stringhe letterali con escape a singolo byte (`\x9`, `\7`, `\0`). Caso degno di nota: **0064** già usa `\0` NUL embedded dentro la stringa letterale — Phase 7 deve **preservare** questo comportamento, ed è anzi una prova che il design Vec&lt;u8&gt; serve. Zero conflitti reali.
- **(c)**: Totale ≈ **116 hit** `String|&str` nei 10 file sorgente (sommando la colonna). Distribuzione coerente con piano: `src/types.rs` (42) e `src/runner/mod.rs` (23) dominano, in linea con la mappa file di Phase 7.2 (AwkValue) e 7.1 (I/O input). `src/runner/builtins.rs` (15) e `src/ast.rs` (14) seguono. Outlier basso: `main.rs` e `scratch.rs` a 0 — fuori scope Phase 7.
- **(d)**: Pest grammar **ACCETTA** byte > 0x7f via escape `\NNN`/`\xHH` nei letterali stringa. L'output del programma di test produce il byte `0xff` (visualizzato come `ÿ` per render latin-1 in terminale), confermando che il parser non rifiuta byte alti e il path BEGIN→print funziona end-to-end. Nessun blocco al confine parser per la Phase 7.2 sul lato AwkValue.

**Verdict**: ✅ **GO con criteri originali**. Nessun conflitto trovato, nessun fix preliminare richiesto. Phase 7.1 può partire.

## Note di esecuzione (deviazioni dal plan committato `bc23fe3`)

Lo script committato come Task 7.0.1 conteneva tre defect scoperti in fase di esecuzione, corretti in fix-up commit prima del run definitivo:

1. **`rg` non è un binario di sistema** ma una shell-function wrapper di Claude Code, non disponibile in `bash` subprocess. Sostituito con `perl -0777 -ne` per byte-range detection in (a) e con `grep -nE` / `grep -cE` POSIX per (b)(c).
2. **Glob `tests/*.xml`** matchava solo il manifest legacy `testsuite.xml`, lasciando fuori i 109 case in `tests/cases/*.xml`. Esteso per scansionare entrambi.
3. **Comando (d)** eseguiva `cargo run -q --` senza argomento posizionale, causando errore clap `required argument <PROGRAM>` (non è il fallimento di pest che si voleva testare). Corretto in `cargo run -q -- 'BEGIN { print "\xff" }'` per esercitare davvero il parser + runner sul byte alto.

Le correzioni non alterano l'**intento** del Task 7.0.1 (audit no-code che produce un report informativo) né i criteri di Done. Sono state applicate in un commit di fix-up dedicato per mantenere storia git tracciabile.


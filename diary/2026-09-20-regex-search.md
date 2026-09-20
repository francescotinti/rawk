# Seconda priorità prestazioni: regex e sostituzioni UTF-8

## Obiettivo e stato

Proseguimento autorizzato dopo la consegna dei campi. Base `bab2f7c`, runtime
precedente `42f35e6`, working tree inizialmente pulita. Questa tranche riguarda
la priorità 2; I/O/memoria generale e misure multipiattaforma restano successive.
Stato: misure e convalida in corso.

## Profiling e modifiche

Il campionamento nativo del booleano breve mostra due percorsi `is_match`:
il ramo dell'operatore e la valutazione ricorsiva del letterale destro su `$0`.
Questa valutazione aggiuntiva occupa 1.544/5.530 campioni sotto main (27,9%).
Il suo risultato numerico viene scartato, poiché `~`/`!~` usano il sorgente del
letterale come pattern. Ora tale valutazione viene evitata esclusivamente
quando il RHS è un `RegexLiteral` di questi due operatori. Le regex dinamiche
mantengono la valutazione ordinaria, comprese le espressioni con effetti
collaterali. Anche i letterali in contesto booleano autonomo e sul LHS restano
valutati normalmente. Nessuna modifica al parser o al motore di matching.

Nel campionamento gsub, 676/5.978 campioni sotto main (11,3%) attraversano la
codifica Unicode, con riallocazioni dei vettori di chiavi e offset. La capacità
iniziale diventa almeno 32 byte per chiavi non vuote e otto elementi per gli
offset. I vettori crescono normalmente oltre questa capacità; non cambia il
decoder né l'allineamento delle chiavi. Il caso booleano vuoto continua a non
allocare chiavi. Questi valori delimitano una piccola allocazione iniziale,
non un riconoscimento degli input di benchmark.

`sub`/`gsub` espandono ora le sostituzioni direttamente nel buffer del risultato,
eliminando il vettore temporaneo per ciascun match. Nel profilo precedente
sono visibili allocazioni e liberazioni nel ciclo del builtin. Le regole per
`&`, backslash e `POSIXLY_CORRECT` sono le stesse; il valore destinazione viene
ancora assegnato soltanto dopo la costruzione completa del risultato.

File runtime: `src/runner/mod.rs`, `src/runner/builtins.rs`, `src/unicode_ere.rs`.
Non cambiano DFA, scelta leftmost-longest, decoder BWK/libc/RS, locale, stream,
NUL, stampa atomica, sorgenti C, fixture originali o home/autori.

## Regressioni e misure

Due regressioni aggiunte in `tests/unicode_contract.rs`, incluse nel gate Linux:
valutazione di RHS letterali/dinamici, LHS impliciti e operatori concatenati;
sostituzioni con prefissi, espansioni, escape, match vuoti e mancati, modalità
POSIX e conservazione deliberata del NUL. Il confronto con il C copre C e UTF-8;
il NUL ha un expected Rust esplicito.

`scripts/benchmark.py` aggiunge `utf8-mixed`: regex dinamiche, match su input
variabili e lunghi, sostituzioni espansive/senza match e record vuoti. Il
profilo UTF-8 originario e i carichi byte restano controlli separati.

Build iniziali: `make -C ../c_awk`, `cargo build --locked --release --bins`.
Oracolo `5739fd79bcfc75ba7526773d0cf634521f8aca3c`; contenuti tracciati locali
invariati, permessi e artefatti preesistenti preservati. Ambiente, toolchain,
flag e revisioni in [environment.json](verification-regex-search-2026-09-20/environment.json).
Binario precedente copiato prima delle modifiche in
`/tmp/rawk-before-regex-search-2026-09-20`.

Baseline e confronto: nove ripetizioni intercalate, un warmup per binario,
stdout uguale al C ad ogni esecuzione e stato 0; distribuzioni, mediana RSS,
programmi e SHA-256 di input/binari nei JSON. Tempo wall clock del processo
con wrapper e alimentazione stdin; RSS da `/usr/bin/time -l`. Nessuna build,
suite o sessione di profiling locale contemporanea ai benchmark.

```sh
python3 scripts/benchmark.py --profile utf8 --scale 5 --runs 9 --output /tmp/baseline.json
python3 scripts/benchmark.py --profile utf8 --scale 5 --runs 9 \
  --before /tmp/rawk-before-regex-search-2026-09-20 --output /tmp/after-utf8.json
python3 scripts/benchmark.py --profile utf8-mixed --scale 5 --runs 9 \
  --before /tmp/rawk-before-regex-search-2026-09-20 --output /tmp/after-utf8-mixed.json
python3 scripts/benchmark.py --profile byte --scale 20 --runs 9 \
  --before /tmp/rawk-before-regex-search-2026-09-20 --output /tmp/after-byte.json
```

[Baseline](verification-regex-search-2026-09-20/baseline-utf8.json),
[metodo di profiling](verification-regex-search-2026-09-20/profiling.json),
[profilo booleano](verification-regex-search-2026-09-20/profile-boolean-before.log),
[profilo gsub](verification-regex-search-2026-09-20/profile-gsub-before.log).
I profili usano dieci milioni di record da file, `sample PID 8 1` e locale
`en_US.UTF-8`; non sono misure di velocità da confrontare con i benchmark.

## Verifiche e risultati

Test mirati preesistenti `unicode_contract`, `remaining_contract` e
`bytes_regex_match` verdi; anche le due nuove regressioni passano.
| Carico | Prima (s) | Dopo (s) | Variazione | Dopo/C |
|---|---:|---:|---:|---:|
| utf8_boolean_short | 0.6294 | 0.3879 | -38.4% | 1.88× |
| utf8_boolean_long | 0.2995 | 0.2135 | -28.7% | 0.44× |
| utf8_boolean_miss | 0.5967 | 0.3822 | -35.9% | 2.10× |
| utf8_match | 0.6594 | 0.5518 | -16.3% | 2.39× |
| utf8_gsub | 0.8915 | 0.7160 | -19.7% | 2.27× |
| utf8_dynamic_boolean | 0.1009 | 0.0980 | -2.9% | 1.63× |
| utf8_match_mixed | 0.1107 | 0.1054 | -4.8% | 2.05× |
| utf8_gsub_expanding | 0.1847 | 0.1721 | -6.8% | 2.27× |
| utf8_gsub_miss | 0.0856 | 0.0797 | -6.9% | 1.33× |
| utf8_match_long | 0.2161 | 0.2153 | -0.4% | 0.07× |
| utf8_empty_regex | 1.0589 | 0.9985 | -5.7% | 4.72× |
| sum_fields | 1.2130 | 1.2146 | +0.1% | 1.85× |
| regex_fields | 0.2387 | 0.2378 | -0.4% | 2.92× |
| array_aggregation | 1.8966 | 1.8980 | +0.1% | 2.33× |

Misure su Apple M4, macOS 26.6.2, Rust 1.96.0 in release standard. Benefici
netti nei cinque carichi UTF-8 originari; i controlli misti hanno variazioni
più contenute. Il match posizionale lungo e i tre carichi byte restano
sostanzialmente invariati. Non si attribuisce significatività statistica alle
piccole variazioni dei controlli; le distribuzioni complete restano visibili.
Le capacità iniziali più grandi possono usare qualche byte in più su soggetti
molto corti, senza promettere un risparmio di memoria su ogni programma.
La mediana RSS aumenta di 16.384 byte in `utf8_gsub_expanding` e
`sum_fields`; negli altri dodici carichi è uguale o inferiore. Sono piccole
variazioni del picco residente, non misure esatte delle allocazioni.

[UTF-8](verification-regex-search-2026-09-20/after-utf8.json),
[misti e controlli](verification-regex-search-2026-09-20/after-utf8-mixed.json),
[byte](verification-regex-search-2026-09-20/after-byte.json),
[sintesi tempi/RSS](verification-regex-search-2026-09-20/performance-summary.json).
`CARGO_INCREMENTAL=0 bash scripts/checks.sh`: exit 0, 156 test debug,
zero fallimenti o ignorati; fmt, Clippy, igiene e XML 97/12/0/0 verdi.
`CARGO_INCREMENTAL=0 cargo test --locked --release`: exit 0, 156 test,
zero ignorati. Comandi eseguiti in sequenza. La build ordinaria finale
ripristina esattamente lo SHA-256 del binario misurato dopo la build dei test.
[Gate](verification-regex-search-2026-09-20/checks.log),
[release](verification-regex-search-2026-09-20/release.log),
[esiti e impronta](verification-regex-search-2026-09-20/local-summary.json).
CI sulle quattro piattaforme ancora da completare.

## Git e pubblicazione

Branch `master`, base `bab2f7c`; pubblicazione runtime e CI da completare.
L'hash della consegna documentale verrà riportato nella risposta finale.

## Passaggio al prossimo task

Prossima priorità dopo la convalida: I/O e memoria su input grandi. Le misure
locali ARM64 non sono attribuibili a Intel o Linux. Non è una promessa di
parità generale con il C né una certificazione completa del linguaggio.

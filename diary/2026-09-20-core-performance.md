# Prestazioni: primo intervento sui campi

## Obiettivo e stato

Prima priorità della sequenza prestazioni: baseline dell'HEAD `aa0301a`,
profiling e ottimizzazione circoscritta, conservando i contratti esistenti.
Stato: verifica in corso; risultati e pubblicazione da completare.

## Modifiche e decisioni

Il campionamento nativo della somma di 20 milioni di record attribuisce
2.817 dei 6.188 campioni sotto main (45,5%) a `update_record`. Il grafo mostra
allocazioni/liberazioni nella costruzione dei campi e della seconda lista di
valori. Il campionamento include l'avvio, escluso da questo denominatore;
non è una misura strumentata del costo esatto di ogni istruzione.

`src/types.rs` riutilizza il vettore e i buffer dei campi quando FS è uno
spazio e CSV non è attivo. Rimangono identici i tre separatori byte (spazio,
tab, newline) e la classificazione tramite `from_str_num`. I campi in eccesso
sono rimossi; un campo numerico assegnato dal programma riceve un nuovo buffer.
I valori salvati hanno copie indipendenti. FS spazio è indipendente da RS:
il precedente percorso aggiungeva newline al separatore solo per altri FS.

I percorsi CSV, regex, FS vuoto e altri separatori restano quelli esistenti.
Nessuna specializzazione sul testo del programma o sui valori degli input.
La capacità dei buffer attivi può restare quella di un precedente campo più
lungo; non viene promesso un minor RSS per ogni forma di input.

`tests/data_contract.rs` aggiunge tre regressioni: tipi numerici e valori
salvati tra record di lunghezza diversa, resplit/getline/cambi FS e RS
paragrafo, conservazione deliberata dei NUL. Le prime due confrontano
stdout, stderr e stato con l'oracolo; NUL mantiene un contratto Rust esplicito.
Il file è incluso anche nel gate Linux.

`scripts/benchmark.py` aggiunge il profilo `mixed`, con 127 chiavi, decimali,
esponenti, righe vuote e numero variabile di campi; registra inoltre estremi,
deviazione standard e mediana RSS oltre ai campioni grezzi.

## Metodo e riproduzione

Ambiente, revisioni, toolchain, flag e controllo dei contenuti tracciati C:
[environment.json](verification-core-performance-2026-09-20/environment.json).
L'oracolo è `5739fd79bcfc75ba7526773d0cf634521f8aca3c`; sorgenti e fixture
originali preservati, comprese le differenze locali preesistenti di permessi.
Build: `make -C ../c_awk`, `cargo build --locked --release --bins`.
Binario precedente conservato fuori dal repository in
`/tmp/rawk-before-core-2026-09-20`, prima di modificare il runtime.

La baseline usa nove ripetizioni, riscaldamento e ordine ruotato;
[baseline.json](verification-core-performance-2026-09-20/baseline.json).
Ogni esecuzione deve terminare con successo e avere stdout identico al C;
input e binari hanno SHA-256 nei JSON. Il tempo è wall clock del processo
più wrapper/Python e alimentazione stdin; RSS proviene da `/usr/bin/time -l`.
Le misure non si sovrappongono a build, test o profiling del task.

```sh
python3 scripts/benchmark.py --scale 20 --runs 9 --output /tmp/baseline.json
python3 scripts/benchmark.py --before /tmp/rawk-before-core-2026-09-20 \
  --scale 20 --runs 9 --output /tmp/after-byte.json
python3 scripts/benchmark.py --profile mixed --before /tmp/rawk-before-core-2026-09-20 \
  --scale 5 --runs 9 --output /tmp/after-mixed.json
python3 scripts/benchmark.py --profile utf8 --before /tmp/rawk-before-core-2026-09-20 \
  --scale 5 --runs 9 --output /tmp/after-utf8.json
```

Il profiling usa `sample PID 8 1` sul binario precedente con input da file;
[grafo](verification-core-performance-2026-09-20/profile-before.log),
[metodo](verification-core-performance-2026-09-20/profile-method.json).

## Verifiche e risultati

| Carico | Prima (s) | Dopo (s) | Variazione | Dopo/C |
|---|---:|---:|---:|---:|
| sum_fields | 1.7684 | 1.1778 | -33.4% | 1.79× |
| regex_fields | 0.2428 | 0.2340 | -3.6% | 2.91× |
| array_aggregation | 2.4730 | 1.8796 | -24.0% | 2.29× |
| mixed_fields | 1.0213 | 0.7625 | -25.3% | 1.17× |
| mixed_aggregation | 0.8701 | 0.6123 | -29.6% | 0.93× |
| utf8_boolean_short | 0.7741 | 0.6418 | -17.1% | 3.13× |
| utf8_boolean_long | 0.3009 | 0.2913 | -3.2% | 0.61× |
| utf8_boolean_miss | 0.7320 | 0.5918 | -19.2% | 3.24× |
| utf8_match | 0.7988 | 0.6583 | -17.6% | 2.86× |
| utf8_gsub | 1.1712 | 0.8802 | -24.8% | 2.79× |

Mediane di nove ripetizioni su Apple M4, macOS 26.6.2, Rust 1.96.0,
release standard senza RUSTFLAGS. I due carichi fondamentali migliorano del
24–33%, quelli misti del 25–30%. Le distribuzioni della somma e aggregazione
prima/dopo non si sovrappongono. Le variazioni del controllo regex (-3,6%) e
del booleano lungo (-3,2%) non sono presentate come guadagni affidabili dovuti
alla modifica. Il vantaggio booleano lungo sul C resta presente.
Il miglioramento UTF-8 deriva dal minor costo dei campi per record, senza
interventi nel motore regex. Restano divari rilevanti rispetto al C.

La mediana RSS non aumenta in nessuno dei dieci carichi: per la somma
7.471.104 → 7.372.800 byte, aggregazione 7.520.256 → 7.405.568 byte.
Differenze piccole, senza promessa di un risparmio generale di memoria.
I buffer riutilizzati possono trattenere capacità da record precedenti.

Campioni, stdout, impronte e memoria:
[byte](verification-core-performance-2026-09-20/after-byte.json),
[misti](verification-core-performance-2026-09-20/after-mixed.json),
[UTF-8](verification-core-performance-2026-09-20/after-utf8.json),
[sintesi](verification-core-performance-2026-09-20/performance-summary.json).

Verifica mirata: nove test `data_contract` superati, comprese tre nuove
regressioni. `CARGO_INCREMENTAL=0 bash scripts/checks.sh` completa tutti i
controlli runtime: 154 test debug, fmt/Clippy e XML 97 MATCH / 12 EXPECTED /
0 UNEXPECTED / 0 SKIPPED. Il comando restituisce 1 esclusivamente per
AppleDouble; anche il primo ricontrollo ricrea il metadato del proprio log.
Pulizia con firma AppleDouble verificata e log spostato fuori repository:
`bash scripts/checks.sh check_no_macos_forks` termina con exit 0. I log iniziali
sono conservati, non riscritti. `CARGO_INCREMENTAL=0 cargo test --locked --release`
termina con exit 0: 154 test, zero ignorati. Esecuzione sequenziale.

La build finale `cargo build --locked --release --bins` ripristina esattamente
lo SHA-256 del binario misurato (Cargo test produce un artefatto distinto).
[Esiti locali](verification-core-performance-2026-09-20/local-summary.json),
[gate iniziale](verification-core-performance-2026-09-20/checks.log),
[release](verification-core-performance-2026-09-20/release.log).
CI nativa sulle quattro piattaforme ancora da completare.

## Git e pubblicazione

Branch `master`, base `aa0301a`; commit runtime e CI da registrare.
L'hash della consegna documentale verrà riportato nella risposta finale.

## Passaggio al prossimo task

Prossima priorità: regex UTF-8 brevi/senza match e match/gsub, preservando
il percorso booleano lungo. I/O/memoria e misure multipiattaforma restano
le priorità 3 e 4; la correttezza multipiattaforma è già richiesta qui.
Le misure di velocità locali non si estendono a Intel o Linux.

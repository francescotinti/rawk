# Convalida nativa macOS Intel

## Obiettivo e stato

Convalidare rawk su Darwin x86-64 nativo, mantenendo i gate ARM64 e Linux.
**Stato: completato nel profilo verificato**, CI finale tutta verde.
Convalida nativa; nessuna prova dedotta da cross-compilazione o Rosetta.
Base del task `51bdcf63ebe06db99a78d2ceb314d9cf3686e0f7`.

## Modifiche e decisioni

La [documentazione GitHub dei runner](https://docs.github.com/en/actions/reference/runners/github-hosted-runners),
consultata il 20 settembre 2026, elenca `macos-15-intel` come runner Intel.
La disponibilità è stata poi verificata eseguendo il job: Intel Core i7-8700B,
Darwin x86-64 e toolchain `x86_64-apple-darwin`, senza traduzione Rosetta.

- `.github/workflows/compatibility.yml`: matrice macOS 15 ARM64/Intel;
  gate debug completo, suite release completa, confronto esaustivo delle
  coppie Shift-JIS, contratti originali e sonde stream debug/release.
  Conservati audit grezzo, audit UTF-8 e tutti i gate Linux.
- `scripts/darwin_native_environment.sh`: controllo bloccante di sistema,
  architettura, capacità hardware, assenza di traduzione e host Rust;
  attivazione effettiva delle sei locale richieste; registrazione di OS,
  immagine runner, compilatori, Python, revisione e architettura dell'oracolo,
  collegamento a libSystem. L'oracolo è compilato sul runner nativo.

Il primo tentativo `6ee0275` ha introdotto impropriamente nel job Darwin
la sonda C progettata per glibc. Su entrambi i runner macOS 15 il preflight
fallisce soltanto sulla classificazione di `82 20` come sequenza invalida
(`invalid=unexpected errno=0`). Il controllo di ambiente e la conversione
fullwidth a/A passano. Sulla macchina locale Darwin la stessa sequenza
è invece rifiutata con EILSEQ: non è una proprietà generale dell'architettura
Intel. Le evidenze iniziali sono conservate, senza riscriverle, nelle cartelle
`initial-intel` e `initial-arm64`.

Il commit `b2b62cb` mantiene questa sonda specifica nel job Linux originario.
Darwin verifica le locale tramite attivazione esplicita, regressioni native
contro il C per conversioni/input invalidi/troncati e sonda esaustiva delle
coppie accettate dalla libc del runner. Nessun expected modificato e nessuna
riduzione della suite esistente. Il preflight glibc rimane invariato.

## Controesempio Intel e correzione runtime

Il secondo tentativo `b2b62cb` supera ambiente, fmt, Clippy e XML ma fallisce
le regressioni `unsigned_conversion_matches_native_oracle` e
`integer_flags_precision_and_signs`. Caso minimo nativo:

```awk
BEGIN { printf "%u %o %x\n", -1, -1, -1 }
```

C: `18446744073709551615 1777777777777777777777 ffffffffffffffff`;
Rust prima del fix: `0 0 0`. Stdout è l'unica differenza, exit 0 e stderr vuoto.
La sonda dell'oracolo conserva altri negativi frazionari e grandi, zero,
positivi e le due metà di uint64_t; non è un expected costruito per il fix.

`src/runner/fmt.rs` estende il percorso x86-64 glibc a macOS x86-64 soltanto
per valori nell'intervallo `[-2^63, 0)`: conversione signed e reinterpretazione
unsigned. Darwin ARM64 resta invariato. La regressione in
`tests/dynamic_format.rs` confronta ora sia printf sia sprintf e comprende
`-2^63` e il successivo valore rappresentabile. I test esistenti coprono anche
flag, larghezza, precisione e consumi degli argomenti. Non viene introdotta
una regola C portabile per conversioni fuori intervallo, NaN o infinito.

## Verifiche

Oracolo fissato: `5739fd79bcfc75ba7526773d0cf634521f8aca3c`.
Controllo shell verificato localmente su ARM64, sintassi shell e diff verificati.
Comandi locali eseguiti in sequenza: `make -C ../c_awk`, test mirato
`cargo test --locked --test dynamic_format`, `CARGO_INCREMENTAL=0 bash scripts/checks.sh`,
`CARGO_INCREMENTAL=0 cargo test --locked --release`. Il gate completo e la
suite release terminano con exit 0: 151 test debug e 151 release, zero
ignorati; fmt, Clippy e XML 97 MATCH / 12 EXPECTED / 0 UNEXPECTED / 0 SKIPPED.
Rimossi soltanto i metadati AppleDouble identificati, con elenco conservato.
I contenuti tracciati dell'oracolo locale sono stati confrontati byte per byte
con HEAD e sono invariati; preservate le differenze preesistenti di permessi.
La [CI finale `11d7313`](https://github.com/francescotinti/rawk/actions/runs/35523117465)
è tutta verde. Intel: macOS 15.7.9 (24G830), Darwin 24.6.0, immagine
20260824.0482.1, Intel Core i7-8700B, Rust/Cargo 1.96.0, Apple Clang 17.0.0
(clang-1700.0.13.5), Python 3.14.7; oracolo Mach-O x86-64 collegato a
`/usr/lib/libSystem.B.dylib` versione 1351.0.0. `uname -m` e host Rust
sono x86-64; `arch` stampa il nome storico `i386`. Il controllo hardware
x86-64 passa e non c'è traduzione Rosetta.

Locale attivate: C, en_US.UTF-8, tr_TR.UTF-8, en_US.ISO8859-1,
tr_TR.ISO8859-9, ja_JP.SJIS. Le suite verificano anche la selezione/precedenza.

| Profilo nativo | Debug | Release | Shift-JIS: coppie valide |
| --- | ---: | ---: | ---: |
| macOS 15.7.9 Intel | 151 | 151 | 15.240 |
| macOS 15.7.9 ARM64 | 151 | 151 | 15.240 |
| Linux glibc 2.39 x86-64 | 89 | 89 | 6.879 |
| Linux glibc 2.39 ARM64 | 89 | 89 | 6.879 |

Zero fallimenti o ignorati. Entrambi i gate macOS passano fmt/Clippy e XML
97 MATCH / 12 EXPECTED / 0 UNEXPECTED / 0 SKIPPED. Per ogni architettura e
profilo debug/release: 27/27 driver, 275/275 contratti e 20/20 sonde stream.
Darwin mantiene 173 casi esatti, 99 diagnostiche e tre contratti deliberati;
Linux mantiene i suoi due contratti diagnostici specifici (171/101/3).
Gli audit originali UTF-8 passano tutti i 300 casi su entrambi i macOS.

L'inventario grezzo Darwin è 27 pass, due open (T.flags/T.misc), quattro
reference-failure (T.builtin/T.errmsg/T.utf/T.utfre). Restano i problemi
storici dei grep, il ramo tedesco con LC_ALL=C e i driver Unicode lanciati
in C; i contratti individuali e il gate UTF-8 li distinguono. Nessuna
accettazione generica di differenze e nessuna modifica agli expected.

Tutte le 15.240 coppie Shift-JIS sono state scoperte e confrontate sul singolo
runner, con upper/lower entrambi riusciti. Il conteggio 11.280 documentato
in precedenza appartiene al diverso profilo locale (macOS 26.6.2/libSystem
1356.0.0), non è un invariante Darwin. Glibc mantiene il caso lower `81 f0`
con errore atteso, verificato anche in questa CI. La gestione matematica
Darwin non ha richiesto modifiche: regressioni native verdi.

[Evidenze e SHA-256](verification-macos-intel-2026-09-20/summary.json),
[stato CI completo](verification-macos-intel-2026-09-20/ci-final.json).
Le cartelle `initial-*` e `second-intel` conservano i tentativi falliti;
`final-*` gli artefatti definitivi, `local-arm64` le verifiche locali.
Il runtime finale modifica soltanto la formattazione unsigned Intel;
preservati home/autori, sorgenti C, fixture, NUL, decoder BWK/libc/RS,
stampa atomica e correzioni precedenti.

## Git e pubblicazione

Branch `master`; infrastruttura `6ee0275` e correzione harness `b2b62cb`
pubblicati. Correzione runtime e regressione `11d7313`, pubblicate e sottoposte
alla CI completa. CI verificata su `11d73134b9403d27ba00b2bf1be4ffe8b1c0f53c`, presente sul remoto.
Il commit conclusivo contiene solo documentazione/evidenze e usa `[skip ci]`;
hash effettivo, controllo remoto e working tree sono riportati nella risposta
finale, senza attribuire la CI a un commit documentale successivo.

## Passaggio al prossimo task

Il profilo verificato non certifica tutte le versioni macOS, tutte le codifiche
o cast C fuori intervallo. Altre codifiche, percorsi non UTF-8 e ottimizzazioni
restano attività distinte. Riprodurre tramite il workflow compatibility;
leggere questo rapporto e `docs/COMPATIBILITY.md`.

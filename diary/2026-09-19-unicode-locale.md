# Unicode e locale — 19 settembre 2026

**Stato: supporto UTF-8 implementato e verificato su Darwin ARM64.** Attività successiva
alla Fase 7, avviata su richiesta dell'utente. Il profilo byte-oriented chiuso
nella Fase 7 rimane un gate separato; i suoi rapporti storici non sono riscritti.

## Comportamento implementato

Il runtime seleziona `LC_CTYPE` all'avvio con precedenza `LC_ALL` non vuoto,
`LC_CTYPE` non vuoto, `LANG`, quindi `C`. Un locale inesistente conserva il
comportamento C, come l'oracolo. Viene creato un oggetto locale immutabile della
libc senza modificare la locale globale o la conversione numerica. Cambiare
ENVIRON dopo l'avvio non modifica la semantica dell'interprete.

Con una locale UTF-8 disponibile:

- `length`, `substr`, `index`, `match`, RSTART e RLENGTH usano unità carattere;
  `length(array)` continua a contare gli elementi;
- FS vuoto e `split(..., "")` separano i caratteri;
- regex letterali e dinamiche, classi/range, ancore, alternanze, ripetizioni,
  sub/gsub e RS rispettano le stesse unità e la scelta leftmost-longest;
- i match vuoti avanzano di un carattere intero;
- `%s` applica larghezza e precisione in caratteri; `%c` emette il primo carattere
  di una stringa o il codice Unicode di un numero;
- `tolower`/`toupper` usano le conversioni a un carattere della locale di sistema,
  comprese le particolarità turche. Non applicano espansioni arbitrarie come
  `ß` → `SS`, che differirebbero dal C;
- gli escape `\u` dei sorgenti producono gli stessi byte del riferimento.

I dati rimangono sequenze di byte. Il conteggio e le regex seguono il decoder
strutturale BWK, che conserva byte malformati e riconosce sequenze di 2/3/4 byte;
non viene sostituito l'input con U+FFFD. La conversione maiuscole/minuscole rifiuta
invece sequenze UTF-8 illegali, coerentemente con la conversione multibyte del C.
La conservazione dei NUL rimane l'estensione deliberata del runtime Rust.

Non viene promessa una semantica per grafemi o normalizzazione Unicode: una
lettera e un accento combinante rimangono due unità. Le classi POSIX riproducono
la tabella ctype sui valori 1–255 usata dal sorgente C; non sono sostituite con
proprietà Unicode più estese (per esempio `[:alpha:]` non include ogni alfabeto).

## Regex e lettura incrementale

`unicode_ere.rs` traduce ogni unità BWK in una chiave di quattro byte. Lo stesso
DFA Rust già usato dal profilo byte conserva la scelta del match più lungo alla
prima posizione. Gli intervalli sono compilati come intervalli di byte, senza
espandere elenchi di milioni di caratteri. Una mappa conserva gli offset nel
buffer originale; gli inizi dei match sono allineati alle chiavi.

`split` e `gsub` preparano una sola rappresentazione dell'input e la riutilizzano:
ricodificare l'intera stringa a ogni match introdurrebbe un costo quadratico.
La rappresentazione e la mappa richiedono memoria proporzionale al testo.

RS non interpreta un suffisso UTF-8 incompleto prima del successivo read. È
coperto il controesempio con un euro troncato al confine di 8192 byte, il cui
primo byte non deve diventare il separatore `â`. A EOF i byte incompleti vengono
trattati come byte isolati, come nel riferimento.

La dipendenza diretta `libc` usa la versione 0.2.186 già presente nel lockfile.
Le FFI sono limitate alla selezione/lettura del locale e alle classificazioni e
conversioni ctype; l'esecuzione AWK e le regex non sono delegate al codice C.

## Test e gate

I driver originali vengono ora eseguiti anche con `--locale en_US.UTF-8`:

| Driver | Casi originali | Risultato |
|---|---:|---|
| T.utf | 106 | Passante su C e Rust |
| T.utfre | 194 | Passante su C e Rust |

I due driver sono obbligatori in `tests/unicode_contract.rs`. In `LC_ALL=C`
restano distinti: le loro aspettative Unicode non sono quelle del profilo byte.
L'inventario dichiara entrambi i profili, senza trasformare i fallimenti delle
aspettative Unicode in successi del profilo C.

Test aggiuntivi coprono precedenza delle variabili, locale inesistente,
`en_US.UTF-8` e `tr_TR.UTF-8`, indici all'interno di sequenze multibyte,
byte malformati/troncati, NUL, conversioni case, escape, larghezza/precisione,
match vuoti, range, buffer e sostituzioni ripetute. Gli intervalli compilati
hanno sonde deterministiche ai confini e 1.000 valori generati per intervallo.

Il runner condiviso mantiene `LC_ALL=C` come default ma rispetta una variabile
LC_ALL esplicitamente impostata o rimossa dal singolo test. Senza questo
accorgimento un test etichettato UTF-8 potrebbe eseguire accidentalmente in C.

```sh
CARGO_INCREMENTAL=0 bash scripts/checks.sh
CARGO_INCREMENTAL=0 cargo test --locked --release
python3 scripts/full_driver_audit.py --rawk target/release/rawk \
  --drivers T.utf T.utfre --locale en_US.UTF-8 --check \
  --output diary/unicode-driver-results.json
```

- **124 test superati in debug e 124 in release**.
- Gate completo `checks.sh`: esito **0**, inclusi fmt, Clippy, igiene e regressioni.
- XML: **97 MATCH, 12 EXPECTED, 0 UNEXPECTED, 0 SKIPPED**, invariati.
- **106 + 194 casi originali UTF-8 passanti** in entrambi gli interpreti;
  rapporto finale release in [unicode-driver-results.json](unicode-driver-results.json).
- Il controllo di igiene era inizialmente fallito per file AppleDouble creati
  durante gli aggiornamenti. Rimossi solo i metadati identificati dalla firma
  binaria, il gate è stato rieseguito integralmente e ha esito 0.
- Log, impronte dei sorgenti e metadati:
  [verification-unicode-2026-09-19](verification-unicode-2026-09-19/summary.json).

## Misure dopo l'intervento

Nel profilo C, sette ripetizioni intercalate con il binario della copia pulita
della Fase 7, dopo la fine dei test concorrenti della sessione. Output identico
al C per tutti i carichi. Le mediane non sono una prova di significatività statistica.

| Carico byte | Prima (s) | Dopo (s) | Variazione |
|---|---:|---:|---:|
| sum_fields | 0.4611 | 0.4549 | -1.3% |
| regex_fields | 0.0654 | 0.0693 | +6.0% |
| array_aggregation | 0.6350 | 0.6280 | -1.1% |

Il carico regex mostra un aumento di circa il 6%; gli altri due sono vicini alla
base precedente. La memoria residente resta sostanzialmente stabile. Tutte le
ripetizioni, RSS, impronte e programmi sono in
[benchmark-unicode-byte-profile.json](benchmark-unicode-byte-profile.json).

Per il profilo UTF-8, cinque ripetizioni dopo riscaldamento, 100.000 record per
carico, sempre con stdout identico al C:

| Carico UTF-8 | C (s) | Rust (s) |
|---|---:|---:|
| length | 0.0402 | 0.0986 |
| regex | 0.0503 | 0.1697 |
| gsub | 0.0682 | 0.2404 |

Rust resta più lento del riferimento. Queste misure non includono una promessa
prestazionale per testi/regex arbitrari; la rappresentazione temporanea Unicode
richiede memoria lineare nel numero di unità del testo. Dati, unità di input e
programmi: [benchmark-unicode-profile.json](benchmark-unicode-profile.json).


## Confini e attività successive

Convalida su Darwin ARM64, con locale C e UTF-8 disponibili sul sistema. Altre
piattaforme e codifiche legacy non UTF-8 richiedono verifiche dedicate. La
formattazione numerica resta quella del profilo AWK già verificato: questo
intervento non introduce il separatore decimale localizzato.

Il workflow macOS include i test UTF-8 e conserva il relativo rapporto, oltre
all'audit byte-oriented. CI remota non eseguita, nessun push. La prossima attività
è la convalida su altre piattaforme; le ottimizzazioni ulteriori rimangono guidate
da misure e regressioni riproducibili.

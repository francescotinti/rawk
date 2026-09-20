# Locale legacy a byte singolo — 20 settembre 2026

Tranche: conversioni e classi POSIX per ISO-8859-1 e ISO-8859-9, verificate
contro il C originale su Darwin ARM64. Shift-JIS resta un intervento distinto;
la convalida nativa Linux rimane sospesa per assenza del runtime locale.

## Problema e correzione

Con LC_ALL=en_US.ISO8859-1, toupper del byte e9 deve produrre c9 e [:alpha:]
deve riconoscere e9. Rust usava conversioni/classi ASCII anche quando la locale
selezionata era una delle due codifiche a byte singolo.

`text.rs` riconosce il CODESET restituito dalla libc, normalizzando trattini,
underscore e maiuscole, per ISO8859-1 e ISO8859-9. Le conversioni usano
`toupper_l`/`tolower_l` sull'oggetto locale immutabile già creato; le classi nel
compilatore ERE a byte usano le corrispondenti funzioni ctype della stessa locale.
Ogni byte viene passato come valore unsigned 0–255: nessun indice negativo
nelle tabelle C. Il runtime non cambia la locale globale del processo.

Restano le unità a byte per length, substr, index, match, split e formattazione.
Le sequenze c3 a9, che in UTF-8 rappresentano un carattere, qui restano due byte.
I NUL interni vengono conservati anche durante la conversione, secondo il
contratto deliberato di Rust. Il C originale termina la stringa al primo NUL.

La selezione conserva la precedenza LC_ALL non vuoto, LC_CTYPE non vuoto,
LANG, quindi C; una locale inesistente ricade nel profilo C. Cambiare ENVIRON
nel programma non riconfigura la locale già selezionata.

## Test riproducibili

`tests/legacy_locale.rs` verifica entrambe le locale senza skip:

- precondizione osservabile: e9 diventa c9 ed è classificato alfabetico;
- conversioni upper/lower per tutti i 255 byte non nulli;
- tutti i 255 byte contro ciascuna delle 12 classi POSIX, inclusa negazione
  e risultato di match;
- posizioni, range, split, gsub e formattazione a byte;
- NUL interni come estensione Rust;
- precedenza delle variabili, locale inesistente e selezione fissa all'avvio.

I primi cinque test sono stati eseguiti prima della correzione: tutti fallivano.
Dopo la correzione tutti passano; il sesto copre la selezione della locale.
Il confronto mantiene stdout byte per byte, stderr e codice di uscita.

```sh
CARGO_INCREMENTAL=0 cargo test --locked --test legacy_locale
CARGO_INCREMENTAL=0 bash scripts/checks.sh
CARGO_INCREMENTAL=0 cargo test --locked --release
```

La CI Linux predisposta genera anche en_US.ISO8859-1 e tr_TR.ISO8859-9, e il
gate di portabilità include la nuova suite. L'esecuzione nativa Linux non è
stata effettuata: il controllo Rust per il target non convalida la libc remota.

## Confini

Il riconoscimento è intenzionalmente limitato alle due codifiche dichiarate;
essere diversi da UTF-8 non implica usare un solo byte. Non si dichiara supporto
per tutte le codifiche legacy, né una certificazione completa POSIX.

Shift-JIS è ancora aperto. Il C combina il decoder strutturale BWK nelle
operazioni di stringa/regex con conversioni multibyte della libc per il case:
sostituirlo indiscriminatamente con il percorso a byte non sarebbe corretto.
La sonda della tranche precedente rimane un caso da risolvere, non una
divergenza accettata nel nuovo gate. Le tabelle ISO possono differire tra libc:
i test le confrontano con l'oracolo compilato sulla medesima piattaforma.

## Risultati finali

- Gate completo: exit 0; 132 test in debug, fmt, Clippy, build release e igiene.
- Suite completa release: exit 0; 132 test, inclusi i 300 casi originali UTF-8.
- XML: 97 MATCH, 12 EXPECTED, 0 UNEXPECTED, 0 SKIPPED.
- Controlli Rust all-targets per Linux glibc x86-64 e ARM64: exit 0.
  Il filesystem esterno segnala il ripiego da hard link a copie nella cache
  incrementale; nessun errore di compilazione. Non è stata eseguita una build
  collegata o una prova nativa Linux.
- Workflow YAML e sintassi Bash verificati. Nessun push o esecuzione CI remota.

Log prima/dopo, gate, controlli dei target e impronte dei sorgenti:
[verification-legacy-2026-09-20](verification-legacy-2026-09-20/summary.json).

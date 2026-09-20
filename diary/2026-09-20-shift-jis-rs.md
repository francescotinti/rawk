# Chiusura della divergenza Shift-JIS nei separatori RS

## Obiettivo e comportamento

**Stato: completato nel perimetro verificato su Darwin ARM64.** La regressione
RS prima ignorata è ora attiva e passa.

Correggere la divergenza documentata nella [prima consegna Shift-JIS](2026-09-20-shift-jis.md),
senza estenderne la soluzione ai decoder delle stringhe o delle regex in memoria.
Il controesempio originale è `BEGIN{RS=".."}{print length($0),$0}` con input
`e180807a0a`. La regressione è stata riattivata senza cambiarne l'oracolo C.

Nel C, `fnematch` rende visibili al decoder gruppi di `MB_CUR_MAX` byte (due
in Shift-JIS), anche se per completare il lookahead ne manca uno solo. Quando
un candidato fallisce, riparte dal candidato successivo conservando i byte
già letti. Perciò il decoder strutturale BWK può riconoscere una sequenza di
tre byte in alcune posizioni, ma trattarne i byte separatamente in altre.
Decodificare tutto il buffer prima della ricerca perde questa informazione.

## Modifiche e decisioni

- `src/ere/stream.rs`: automa per RS Shift-JIS, con posizione del candidato,
  posizione corrente, limite dei byte visibili e ultimo match accettato.
  Lo stato sopravvive alle letture fisiche incomplete. I riavvii conservano
  il lookahead; la ricerca mantiene il match non vuoto più lungo del primo
  candidato valido.
- `src/unicode_ere.rs`: compilazione separata per il flusso. EOF è un simbolo
  fuori dall'intervallo dei rune BWK; `$` lo consuma e `.`/classi non possono
  confonderlo con dati, inclusi NUL. Questo permette di osservare l'accettazione
  del DFA a ogni confine di rune senza simulare un falso EOF.
- `src/input.rs`: usa questo percorso soltanto per RS di più byte nella locale
  Shift-JIS. Riutilizza il lettore incrementale, compattazione e cache, con
  offset relativi al record. `getline` condivide lo stesso comportamento.
- `src/text.rs`: espone la selezione Shift-JIS già fissata all'avvio.
- `tests/shift_jis.rs`: rimosso l'ignore; aggiunte 180 combinazioni di sequenze,
  prefissi e regex, ancore, alternative, ripetizioni, match vuoti, cambi di RS,
  EOF troncati, confini 8192 byte, `getline` e NUL. Test unitario per letture
  fisiche di 1, 2, 3, 7 e 8192 byte.

Le conversioni Shift-JIS, le stringhe, FS, le regex in memoria e i percorsi C,
UTF-8 e ISO-8859-1/9 conservano il comportamento precedente. Rimangono
deliberate la conservazione dei NUL e la stampa atomica in caso di errore.
Nessuna modifica alla home, all'oracolo o agli expected XML. La soluzione
non dipende da una tabella di risultati per i controesempi.

## Verifiche riproducibili

Le tre regressioni RS fallivano prima della correzione. Le evidenze storiche
della prima consegna non sono state riscritte. Nuove evidenze nella
[directory di verifica](verification-shift-jis-rs-2026-09-20/).

```sh
CARGO_INCREMENTAL=0 cargo test --locked --bin rawk ere::stream
CARGO_INCREMENTAL=0 cargo test --locked --test shift_jis
python3 diary/verification-shift-jis-rs-2026-09-20/probe.py
python3 diary/verification-shift-jis-rs-2026-09-20/audit_high_byte.py
CARGO_INCREMENTAL=0 bash scripts/checks.sh
CARGO_INCREMENTAL=0 cargo test --locked --release
```

La sonda aggiuntiva usa seed fisso, 32 regex e 89 input per regex, e registra
gli hash dei binari e dei risultati oltre agli eventuali controesempi.
Risultato: **2.440 confronti esatti** e **408 errori dell'oracolo C** dovuti a
`ungetc(*k, f)` con `char` signed FF, interpretato come EOF. La sonda grezza
restituisce exit 1 per questi errori e li conserva integralmente.

L'audit aggiuntivo dei 408 casi sostituisce FF con FE soltanto nell'input C:
i pattern della sonda trattano entrambi allo stesso modo e gli input generati
non contengono FE. Ripristinando FF nell'output, tutti i risultati coincidono
con Rust; il controllo richiede anche exit 0 e stderr vuoto del C modificato.
È una verifica ausiliaria esplicita, non 408 confronti integrali con l'input
originale. L'oracolo e gli input storici restano intatti. Rust mantiene la
conservazione dei byte binari, fissata anche da una regressione dedicata.
Ambiente: Darwin ARM64, Rust 1.96.0, `ja_JP.SJIS`; oracolo C a revisione
`5739fd79bcfc75ba7526773d0cf634521f8aca3c`. Linux nativo resta sospeso;
nessuna certificazione generale POSIX o convalida di altre codifiche/piattaforme.

## Risultati finali

- **144 test debug e 144 release passati; 0 ignorati**, inclusi gli 11 test
  Shift-JIS e i 300 casi originali UTF-8. Suite release: exit 0.
- XML: 97 MATCH, 12 EXPECTED, 0 UNEXPECTED, 0 SKIPPED. Fmt, Clippy e build
  release superati; controlli Rust all-targets Linux glibc x86-64/ARM64 passati.
- Il gate aggregato ha restituito exit 1 soltanto per metadati AppleDouble
  creati sul volume esterno. Rimossi, e `check_no_macos_forks` riverificato
  con exit 0; nessun test runtime fallito e nessuna suite già verde ripetuta
  per una modifica ai soli metadati.
- Sonda: 2.440 confronti esatti e 408 errori C verificati con l'audit ausiliario
  descritto sopra; nessuna differenza inattesa rimasta.

[Riepilogo e impronte](verification-shift-jis-rs-2026-09-20/summary.json),
[gate](verification-shift-jis-rs-2026-09-20/gate.log),
[release](verification-shift-jis-rs-2026-09-20/release.log),
[sonda grezza](verification-shift-jis-rs-2026-09-20/probe-results.json),
[audit FF](verification-shift-jis-rs-2026-09-20/high-byte-audit.json).
Le evidenze includono i fallimenti iniziali e la prima correzione intermedia;
non sono state ripulite sostituendole con log verdi.

## Git e prossimo intervento

Base locale e remota prima del lavoro: `6bd0c0f`, branch `master`. Commit e
push sono autorizzati dalla regola permanente in `AGENTS.md`; hash effettivo
ed esito della verifica remota vengono riportati nella risposta finale.
Lo stato CI va distinto dalla riuscita del push.

Le eventuali attività su Linux nativo e altre codifiche rimangono separate.
Per nuovi controesempi leggere `tests/shift_jis.rs`, `src/ere/stream.rs`,
`src/input.rs` e `../c_awk/b.c:fnematch`; usare la sonda con seed fisso per
riprodurre il perimetro qui verificato.

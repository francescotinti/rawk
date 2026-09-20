# Stato breve di rawk

Aggiornato: 20 settembre 2026. Punto di ingresso per nuove attività.

## Base e ambito verificato

- Repository: `francescotinti/rawk`, branch di coordinamento `master`.
- Commit runtime pubblicato verificato: `a118616` (correzione CI Linux, tutti i job verdi).
  Eseguire `git status` e `git log` per lo stato corrente; questo non è un puntatore dinamico.
- Fasi 1–7 chiuse nel profilo C su Darwin ARM64; successivamente aggiunti UTF-8,
  conversioni/classi ISO-8859-1/9 e ottimizzazione delle regex booleane UTF-8.
- Il residuo RS Shift-JIS è corretto nella consegna successiva a `6bd0c0f`;
  dettagli, evidenze e perimetro nel [rapporto RS](diary/2026-09-20-shift-jis-rs.md).
- Verifica corrente: **148 test debug e 148 release passati su Darwin ARM64**, nessun ignorato.
  XML 97 MATCH / 12 EXPECTED / 0 UNEXPECTED / 0 SKIPPED; fmt e Clippy passati.
  CI macOS verde, inclusi gate e audit originali/UTF-8.
- **Linux glibc x86-64 e ARM64 nativi verdi in CI**: 75 test debug + 75 release
  su ciascun runner, nessun ignorato; fmt, Clippy e build release passati.
  Corretti cast `wchar_t`, precisione dinamica negativa, unsigned negativi x86-64
  e zero-padding di `%s`, preservando Darwin e i percorsi binari/multibyte.
  [CI verificata](https://github.com/francescotinti/rawk/actions/runs/35507960469),
  [consegna Linux](diary/2026-09-20-linux-ci.md),
  [evidenze](diary/verification-linux-ci-2026-09-20/summary.json).
  Il gate locale ha segnalato solo metadati AppleDouble, rimossi con controllo
  di igiene riverificato; gli esiti originali restano conservati.
- Verifica precedente: 132 test debug/release, 300 casi originali UTF-8;
  XML 97 MATCH, 12 EXPECTED, 0 UNEXPECTED, 0 SKIPPED.
  [Evidenze storiche](diary/verification-legacy-2026-09-20/summary.json).
- Oracle: `../c_awk`, revisione `5739fd79bcfc75ba7526773d0cf634521f8aca3c`.
  Non è una certificazione POSIX/gawk completa. Conservare le differenze deliberate.

## Shift-JIS: residuo RS risolto su Darwin ARM64

Conversioni libc, errori su input invalido/troncato e unità strutturali BWK
implementati per `ja_JP.SJIS`. La sonda e9 ora restituisce errore anche in Rust;
restano deliberate la stampa atomica su errore e la conservazione dei NUL.
Verificate 11.280 coppie valide per upper/lower su Darwin ARM64.

Il percorso RS regex ora riproduce il lookahead incrementale e i riavvii di
`fnematch`, mantenendo separati i decoder delle regex in memoria. La regressione
prima ignorata è attiva e passa; verificati anche match più lunghi, ancore,
EOF, cambi di RS, letture fisiche corte, confini del buffer e `getline`.
Non è una certificazione generale di tutte le codifiche o piattaforme.
[Scheda](docs/tasks/SHIFT_JIS.md), [consegna RS](diary/2026-09-20-shift-jis-rs.md).

## Attività aperte

- Shift-JIS su Linux resta da convalidare: il gate Linux corrente non include
  `tests/shift_jis.rs` e non prepara `ja_JP.SJIS`. La CI verde non certifica
  questa codifica né l'intero inventario dei driver originali su Linux.
- Altre codifiche, macOS Intel nativo, nomi di file non UTF-8 e ulteriori
  ottimizzazioni: attività distinte da delimitare, non parte implicita di Shift-JIS.

## Regole e documenti utili

- [AGENTS.md](AGENTS.md): vincoli, autonomia, home, test e consegna.
- [Piano canonico](audit-2026-09-19/PIANO_DI_LAVORO.md).
- [Locale legacy](diary/2026-09-20-legacy-locales.md),
  [Unicode](diary/2026-09-19-unicode-locale.md),
  [compatibilità](docs/COMPATIBILITY.md).
- [Modello di consegna](docs/CONSEGNA_TASK.md).

Le priorità trasversali restano nella conversazione di coordinamento; Shift-JIS ha un task dedicato.
Non avviare altri writer sulla stessa checkout mentre quel task modifica il codice.

Commit e push a fine attività sono ora autorizzati in modo permanente;
seguire la regola di chiusura in [AGENTS.md](AGENTS.md).

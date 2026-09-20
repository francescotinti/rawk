# Stato breve di rawk

Aggiornato: 20 settembre 2026. Punto di ingresso per nuove attività.

## Base e ambito verificato

- Repository: `francescotinti/rawk`, branch di coordinamento `master`.
- Ultimo commit pubblicato verificato: `6bd0c0f` (prima consegna Shift-JIS e regola commit/push).
  Eseguire `git status` e `git log` per lo stato corrente; questo non è un puntatore dinamico.
- Fasi 1–7 chiuse nel profilo C su Darwin ARM64; successivamente aggiunti UTF-8,
  conversioni/classi ISO-8859-1/9 e ottimizzazione delle regex booleane UTF-8.
- Il residuo RS Shift-JIS è corretto nella consegna successiva a `6bd0c0f`;
  dettagli, evidenze e perimetro nel [rapporto RS](diary/2026-09-20-shift-jis-rs.md).
- Verifica corrente: **144 test debug e 144 release passati, nessun ignorato**.
  XML invariato; fmt/Clippy e controlli Rust Linux glibc dei due target superati.
  Gate: solo igiene AppleDouble inizialmente fallita, riparata e riverificata.
  [Evidenze RS](diary/verification-shift-jis-rs-2026-09-20/summary.json).
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

- Linux glibc x86-64/ARM64: CI predisposta e controlli Rust dei target passati;
  verifica nativa sospesa per assenza del runtime locale. Dopo i push la CI
  potrebbe essere partita: il suo esito remoto non è stato verificato qui.
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

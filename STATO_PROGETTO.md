# Stato breve di rawk

Aggiornato: 20 settembre 2026. Punto di ingresso per nuove attività.

## Base e ambito verificato

- Repository: `francescotinti/rawk`, branch di coordinamento `master`.
- Ultima convalida: **macOS 15.7.9 Intel x86-64 nativo**, runtime `11d7313`.
  Corretta la formattazione unsigned negativa su Intel; ARM64 invariato.
  **151 debug + 151 release** su ciascun runner macOS Intel/ARM64; XML
  97 MATCH / 12 EXPECTED / 0 UNEXPECTED / 0 SKIPPED. Per profilo: 27 driver,
  275 contratti, 20 sonde stream; audit originali UTF-8 verdi.
  **15.240 coppie Shift-JIS** verificate su ciascun runner macOS 15.7.9;
  il precedente conteggio locale 11.280 appartiene a un altro profilo libc/OS.
  Linux x86-64/ARM64: 89 debug + 89 release per runner, tutti i gate verdi.
  [CI completa](https://github.com/francescotinti/rawk/actions/runs/35523117465),
  [consegna Intel](diary/2026-09-20-macos-intel.md).
- Runtime precedente verificato: `b1adc82` (alias degli stream standard e gestione errori
  matematici glibc; audit driver Linux). Eseguire `git status` e `git log` per
  lo stato corrente; questo non è un puntatore dinamico.
- Ultima convalida Linux: **89 debug + 89 release per runner** x86-64/ARM64,
  zero ignorati; **27/27 driver ammessi**, **275/275 contratti individuali** e
  **20/20 sonde stream** in entrambi i profili. Gli errori runtime sono corretti,
  le differenze deliberate e diagnostiche hanno contratti espliciti.
  Darwin locale **151 debug + 151 release**; macOS CI **151 debug**, XML
  97 MATCH / 12 EXPECTED / 0 UNEXPECTED / 0 SKIPPED e audit verdi.
  La CI macOS non esegue l'intera suite release.
  [CI tutta verde](https://github.com/francescotinti/rawk/actions/runs/35510218505),
  [consegna e classificazione](diary/2026-09-20-linux-drivers.md).
- Convalida precedente: `3b9facf` (infrastruttura/test Shift-JIS, runtime
  allora invariato da `a118616`). Linux 87 debug + 87 release per runner;
  macOS 149 debug, XML 97 MATCH / 12 EXPECTED / 0 UNEXPECTED / 0 SKIPPED.
- Fasi 1–7 chiuse nel profilo C su Darwin ARM64; successivamente aggiunti UTF-8,
  conversioni/classi ISO-8859-1/9 e ottimizzazione delle regex booleane UTF-8.
- Il residuo RS Shift-JIS è corretto nella consegna successiva a `6bd0c0f`;
  dettagli, evidenze e perimetro nel [rapporto RS](diary/2026-09-20-shift-jis-rs.md).
- Verifica locale precedente: **148 test debug e 148 release passati su Darwin ARM64**, nessun ignorato.
  XML 97 MATCH / 12 EXPECTED / 0 UNEXPECTED / 0 SKIPPED; fmt e Clippy passati.
  CI macOS verde, inclusi gate e audit originali/UTF-8.
- **CI Linux precedente (`a118616`)**: 75 test debug + 75 release
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

## Shift-JIS: Darwin ARM64 e Linux glibc nativo

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

Linux glibc 2.39 x86-64 e ARM64: locale generata e sondata nativamente,
12 test Shift-JIS inclusi nel gate debug/release. Tutte le 6.879 coppie valide
verificate contro il C: upper 6.879 successi, lower 6.878 successi e un errore
atteso di ricodifica (`81 f0`), identico nei due interpreti. Nessuna modifica
runtime necessaria. [Consegna Linux Shift-JIS](diary/2026-09-20-shift-jis-linux.md).

## Driver originali Linux

- L'inventario grezzo dei 33 driver Linux conserva 27 pass, tre open e tre
  reference-failure. Il gate ora verifica 27 driver e tutti i 275 sottocasi
  con contratti espliciti (171 esatti, 101 diagnostiche, tre deliberate),
  oltre ai 300 casi originali UTF-8. Non è equivalenza byte per byte delle
  diagnostiche né certificazione generale di tutti i programmi AWK.

## Attività aperte

- Altre codifiche, ulteriori versioni/profili macOS, nomi di file non UTF-8 e ulteriori
  ottimizzazioni: attività distinte da delimitare, non parte implicita di Shift-JIS.

## Regole e documenti utili

- [AGENTS.md](AGENTS.md): vincoli, autonomia, home, test e consegna.
- [Piano canonico](audit-2026-09-19/PIANO_DI_LAVORO.md).
- [Locale legacy](diary/2026-09-20-legacy-locales.md),
  [Unicode](diary/2026-09-19-unicode-locale.md),
  [compatibilità](docs/COMPATIBILITY.md).
- [Modello di consegna](docs/CONSEGNA_TASK.md).

Le priorità trasversali restano nella conversazione di coordinamento.
La convalida macOS Intel è chiusa nel profilo documentato; evitare writer concorrenti sulla stessa checkout.

Commit e push a fine attività sono ora autorizzati in modo permanente;
seguire la regola di chiusura in [AGENTS.md](AGENTS.md).

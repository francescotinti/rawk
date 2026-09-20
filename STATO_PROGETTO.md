# Stato breve di rawk

Aggiornato: 20 settembre 2026. Punto di ingresso per nuove attività.

## Base e ambito verificato

- Repository: `francescotinti/rawk`, branch di coordinamento `master`.
- Ultimo commit pubblicato verificato: `b74272c` (home originale ripristinata).
  Eseguire `git status` e `git log` per lo stato corrente; questo non è un puntatore dinamico.
- Fasi 1–7 chiuse nel profilo C su Darwin ARM64; successivamente aggiunti UTF-8,
  conversioni/classi ISO-8859-1/9 e ottimizzazione delle regex booleane UTF-8.
- Ultima verifica runtime: **132 test debug e 132 release**, gate completo verde;
  300 casi originali UTF-8; XML 97 MATCH, 12 EXPECTED, 0 UNEXPECTED, 0 SKIPPED.
  [Evidenze](diary/verification-legacy-2026-09-20/summary.json).
- La documentazione organizzativa aggiunta dopo questa verifica non modifica il runtime.
- Oracle: `../c_awk`, revisione `5739fd79bcfc75ba7526773d0cf634521f8aca3c`.
  Non è una certificazione POSIX/gawk completa. Conservare le differenze deliberate.

## Prossimo obiettivo: Shift-JIS

Seguire [la scheda del task](docs/tasks/SHIFT_JIS.md). La sonda esistente mostra
che un byte e9 isolato in ja_JP.SJIS causa un errore di conversione nel C mentre
Rust lo accetta. Il C combina decoder strutturale BWK e conversioni libc: non
assumere che tutte le operazioni usino un decoder Shift-JIS uniforme.

## Attività aperte

- Shift-JIS: da implementare e verificare contro il C, preservando i profili esistenti.
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

Questa conversazione resta per il coordinamento; Shift-JIS ha un task dedicato.
Non avviare altri writer sulla stessa checkout mentre quel task modifica il codice.

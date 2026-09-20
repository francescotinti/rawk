# Stato breve di rawk

Aggiornato: 20 settembre 2026. Punto di ingresso per nuove attività.

## Base e ambito verificato

- Repository: `francescotinti/rawk`, branch di coordinamento `master`.
- Ultimo commit pubblicato verificato: `b74272c` (home originale ripristinata).
  Eseguire `git status` e `git log` per lo stato corrente; questo non è un puntatore dinamico.
- Fasi 1–7 chiuse nel profilo C su Darwin ARM64; successivamente aggiunti UTF-8,
  conversioni/classi ISO-8859-1/9 e ottimizzazione delle regex booleane UTF-8.
- Base locale organizzativa: `6cff6bc`; intervento Shift-JIS successivo nella
  commit di consegna Shift-JIS, con pubblicazione autorizzata. Hash e verifica
  remota sono riportati nella risposta di consegna. Verifiche e limiti nel
  [rapporto Shift-JIS](diary/2026-09-20-shift-jis.md).
- Verifica corrente: **139 test debug e 139 release passati, 1 ignorato**
  per il residuo RS. XML invariato; fmt/Clippy superati. Gate: solo igiene
  AppleDouble inizialmente fallita, riparata e riverificata.
  [Evidenze](diary/verification-shift-jis-2026-09-20/summary.json).
- Verifica precedente: 132 test debug/release, 300 casi originali UTF-8;
  XML 97 MATCH, 12 EXPECTED, 0 UNEXPECTED, 0 SKIPPED.
  [Evidenze storiche](diary/verification-legacy-2026-09-20/summary.json).
- Oracle: `../c_awk`, revisione `5739fd79bcfc75ba7526773d0cf634521f8aca3c`.
  Non è una certificazione POSIX/gawk completa. Conservare le differenze deliberate.

## Shift-JIS: implementazione parziale, residuo RS

Conversioni libc, errori su input invalido/troncato e unità strutturali BWK
implementati per `ja_JP.SJIS`. La sonda e9 ora restituisce errore anche in Rust;
restano deliberate la stampa atomica su errore e la conservazione dei NUL.
Verificate 11.280 coppie valide per upper/lower su Darwin ARM64.

Rimane una divergenza nei separatori RS regex con sequenze strutturali di
tre/quattro byte: il decoder C dipende dal riempimento del buffer. Regressione
esatta presente ma esplicitamente ignorata; non dichiarare supporto completo.
Prossimo passo: modellare `b.c:fnematch` separatamente dalle regex in memoria.
[Scheda](docs/tasks/SHIFT_JIS.md), [controesempio e consegna](diary/2026-09-20-shift-jis.md).

## Attività aperte

- Shift-JIS: chiudere il residuo RS descritto sopra e rimuovere il relativo ignore.
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

Commit e push a fine attività sono ora autorizzati in modo permanente;
seguire la regola di chiusura in [AGENTS.md](AGENTS.md).

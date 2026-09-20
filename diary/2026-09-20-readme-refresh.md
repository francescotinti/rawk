# Aggiornamento della home GitHub

## Obiettivo e stato

Completato l'aggiornamento puntuale richiesto del README. Base `31eaf12`,
branch `master`, working tree inizialmente pulita; il lavoro precedente era
 già pubblicato. La home riportava 132 test, Linux non convalidato e Shift-JIS
ancora da implementare. Ora riporta i profili nativi verificati, i conteggi
correnti, le quattro priorità prestazionali completate e i limiti osservati.

## Modifiche e verifiche

Modificati `README.md`, `STATO_PROGETTO.md` e questa consegna. Preservati
struttura, lingua inglese, presentazione e autori della home. Aggiornata soltanto
la sezione Latest Updates e i collegamenti pertinenti. Il piano non cambia.
Verificati diff, link locali e coerenza con la
[consegna delle misure native](2026-09-20-native-performance.md).
Runtime e oracolo C `5739fd79bcfc75ba7526773d0cf634521f8aca3c` invariati;
nessuna nuova suite runtime necessaria per queste modifiche documentali.

## Git e pubblicazione

Commit documentale con `[skip ci]`; hash effettivo, push e working tree vengono
verificati separatamente e riportati nella risposta finale. La
[CI di correttezza](https://github.com/francescotinti/rawk/actions/runs/35533876697)
e la [CI dell'harness](https://github.com/francescotinti/rawk/actions/runs/35534715831)
restano verdi sulle revisioni già convalidate. Nessuna nuova CI richiesta.

## Prossimo intervento

Restano aperti il costo delle righe corte Linux x86-64 da confermare e la
differenza RS regex EOF/`$0`, come documentato nello stato del progetto.
Per riprendere leggere AGENTS.md, STATO_PROGETTO.md e docs/PERFORMANCE.md.

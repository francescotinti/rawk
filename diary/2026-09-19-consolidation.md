# Consolidamento della Fase 7 — 19 settembre 2026

Questa tranche esegue i cinque interventi approvati: contratti del corpus,
printf dinamico e underflow, estensione ai driver originali, CI, profiling e
ottimizzazioni misurate. La Fase 7 resta aperta per le lacune emerse nel corpus
esteso; non è una dichiarazione di conformità completa.

## 1. Sei differenze deliberate, ora verificate

Il corpus piccolo contiene 225 programmi: 219 confronti integrali e sei contratti
specifici, definiti in `tests/corpus-contracts.json`. Tutti sono nel gate.

- `p.43`, `t.in2`: confronto del multinsieme delle righe, preservando le molteplicità.
- `t.intest2`: ordine ignorato solo nei blocchi delimitati da `<<<` e `>>>`;
  testo esterno e sequenza dei blocchi rimangono esatti.
- `p.48b`, `t.randk`: hash esatti delle sequenze C e Rust, con seed predefinito
  e dipendenza rand fissata dal lockfile. Verificate anche ripetibilità,
  intervallo [0,1) e restituzione del seed precedente da srand.
- `t.printf2`: hash esatti dei due flussi binari, inclusi i NUL conservati da Rust.

I contratti fissano anche gli hash di programma e fixture. Richiedono status 0,
assenza di timeout/stderr e uguaglianza dei file prodotti. Prove negative alterano
output, status, timeout, stderr, file e impronte del sorgente: il gate li rifiuta.
`corpus_audit.py --check` fallisce per ogni differenza non coperta da questi
contratti, senza una categoria EXPECTED generica.

## 2. Formattazione e conversione numerica

Printf e sprintf consumano correttamente gli argomenti di larghezza e precisione
`*`, anche combinati e con effetti collaterali. Mancanza degli argomenti e valori
dinamici non finiti o oltre ±1.000.000 producono errore. La precisione dinamica
negativa segue la sostituzione testuale del BWK locale e la sua interpretazione
Darwin, verificata contro il C, non una promessa per altre libc.

La formattazione intera ora gestisce direttamente precisione, segno, padding,
allineamento e prefissi ottali/esadecimali. Questo corregge la perdita di precisione
con allineamento a sinistra della libreria precedente. Confronti coprono d/i/u/o/x/X,
combinazioni di flag, zero e valori negativi; le conversioni unsigned fuori
intervallo sono verificate sul profilo Darwin ARM64.

Letterali e stringhe numeriche fuori intervallo non diventano infiniti o subnormali
numericamente validi. I subnormali prodotti dall'aritmetica rimangono invece
rappresentabili. Sono verificati underflow, overflow, soglia del minimo normale,
zero esatto con esponente estremo e classificazione dei campi String/StrNum.
Il controllo attuale rifiuta i risultati subnormali del parsing; non pretende di
riprodurre ogni dettaglio di errno di tutte le implementazioni strtod.

## 3. Tutti i 33 driver originali inventariati

`full_driver_audit.py` esegue tutti i driver, ciascuno in una copia temporanea
separata per interprete, con timeout di 30 secondi e pulizia dei gruppi di processi.
Copia solo fixture tracciate; controlla preventivamente i percorsi degli archivi.
Gli hash dei driver e le motivazioni sono in `tests/driver-inventory.json`.
Nuovi driver o modifiche dei sorgenti richiedono una revisione esplicita dell'inventario.

Adattamenti dichiarati:

- binario sotto test in `../a.out`, mantenendo ARGV[0] e i percorsi degli archivi;
- `/etc/passwd` e `who` sostituiti con fixture deterministiche di 20 record;
- file temporanei assoluti di T.overflow/T.split ricondotti alla directory isolata;
- helper echo originale compilato; generatori dei casi eseguiti dal C, inclusa
  la parte esterna di T.expr; i sottoprogrammi rimangono eseguiti da rawk;
- confronto non ordinato nella sola asserzione finale di T.argv;
- tre diagnostiche T.clv accettano le due formulazioni precise, imponendo status 2;
- T.beebe usa `make -ks`, preservando lo status e continuando dopo un target fallito:
  la pipeline originale poteva perdere lo status o segnalare errore senza fallimenti.

Risultato: **20 passanti, 8 aperti con C passante, 5 con fallimenti anche nel C**.
Nessun timeout nell'esecuzione finale. I 20 passanti entrano nel manifest permanente
`tests/drivers-verified.list`, eseguito da cargo test; un interprete fittizio che
restituisce successo senza output dimostra che il gate rileva il fallimento del driver.
I 13 rimanenti sono esclusioni esplicite dal gate positivo, non successi. L'audit
completo continua a eseguirli e conserva stream, status e marker in
`diary/full-driver-results.json`.

Correzioni indotte da questa estensione:

- `-f -` legge il programma da stdin; errori di parsing con più sorgenti indicano
  il file effettivamente coinvolto, anche quando non è l'ultimo;
- istruzioni vuote e separatori `;` fra regole;
- `-F t` interpreta il separatore tab come l'originale;
- stdin principale e `getline < "-"` condividono il buffer; letture alternate
  non perdono record e le letture redirette non incrementano NR/FNR;
- `getline < file > 0` confronta il risultato di getline, senza inglobare
  `> 0` nel nome del file: eliminato il ciclo infinito del driver T.split;
- rifiuto preventivo dei parametri duplicati o uguali al nome della funzione,
  evitando anche l'overflow dello stack osservato in T.misc;
- errore per indici di campo oltre il massimo intero del C, coperto da T.overflow.

### Driver ancora aperti

| Driver | Motivo principale |
|---|---|
| T.arnold | system-status fallisce anche nel C: aspettative storiche dipendenti dalla piattaforma |
| T.beebe | Escape regex numerici, graffa letterale, sorgente non UTF-8, splitwht |
| T.builtin | Unicode anche nel C; RNG deliberato, length(array), continuazioni e diagnostiche Rust |
| T.errmsg | Aspettative storiche log/exp non soddisfatte dal C corrente; ulteriori differenze Rust |
| T.expr | Incrementi nei campi, precedenze, conversioni/confronti composti |
| T.flags | Test delle formulazioni diagnostiche CLI originali |
| T.gawk | Escape regex, whitespace, graffa letterale in funstack |
| T.latin1 | Sorgenti inline/file non UTF-8, distinti dagli input dati binari già supportati |
| T.main | Opzione -d senza valore |
| T.misc | Ancore di RS e ulteriori controlli/diagnostiche del corpus esteso |
| T.re | Escape ottali e significato di `\b` nelle regex |
| T.utf, T.utfre | Unicode fuori profilo; fallimenti anche nel C con LC_ALL=C |

L'unità qui è il driver, che può contenere molti sottocasi: 20/33 non è una
percentuale di conformità del linguaggio. Un errore del C non assolve gli errori
aggiuntivi di Rust, conservati separatamente nel rapporto.

## 4. CI riproducibile

Workflow `.github/workflows/compatibility.yml`: macos-15, Rust 1.96.0, lockfile,
checkout e upload-artifact fissati per SHA. L'oracolo viene compilato dalla revisione
`5739fd79bcfc75ba7526773d0cf634521f8aca3c` di onetrueawk/awk, la stessa del riferimento
locale. Il workflow usa permessi di sola lettura e non pubblica codice.

Il gate esegue fmt, Clippy, tutti i test e build release/XML. L'audit dei 33 driver
viene conservato come artifact anche in presenza di casi aperti; i 20 ammessi hanno
invece un controllo vincolante. Il runner macos-15 ARM64 è elencato nella
[documentazione GitHub](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).
Configurazione e comandi sono verificati localmente; **nessun run remoto dichiarato**,
perché non è stato effettuato push.

## 5. Profiling e benchmark

Il campionamento nativo e i punti caldi sono descritti in `profile-consolidation.md`.
Due ottimizzazioni conservate: confronto ASCII senza allocare una stringa temporanea
nel riconoscimento inf/nan; cursore sul buffer dei record, compattato solo
quando occorre acquisire altro input. Semantica condivisa anche da CSV/getline.

Il baseline del confronto finale contiene tutte le correzioni funzionali e differisce
solo per queste due ottimizzazioni. `optimization.diff` le identifica e permette di
ricostruire il baseline con l'applicazione inversa della patch. Il rapporto JSON
`benchmark-consolidation.json` include hash dei binari e degli input, piattaforma,
tutte le misure e stdout. Comando:

```sh
python3 scripts/benchmark.py --before /tmp/rawk-before-consolidation-opt --runs 7 --scale 5
```

Un giro di riscaldamento, sette misure intercalate con ordine ruotato, confronto
byte per byte con il C a ogni esecuzione; tempi end-to-end e RSS da `/usr/bin/time -l`.

| Carico | Record | C | Rust prima | Rust dopo | Riduzione mediana |
|---|---:|---:|---:|---:|---:|
| Somma campi | 500.000 | 0,1690 s | 0,5602 s | 0,4610 s | 17,7% |
| Campi regex | 50.000 | 0,0253 s | 0,0714 s | 0,0653 s | 8,5% |
| Aggregazioni | 500.000 | 0,2120 s | 0,7397 s | 0,6510 s | 12,0% |

Memoria Rust sostanzialmente invariata: circa 7,4–8,1 MB contro 1,6–1,7 MB del C.
Rust rimane più lento del C. Sono misure locali su tre carichi, non una garanzia
universale né una soglia temporale imposta alla CI.

## Verifica finale

Risultati del gate e degli audit conclusivi in `verification-consolidation-2026-09-19/`.

| Controllo | Risultato |
|---|---|
| Build release, fmt, Clippy | OK |
| Suite Rust | 107 test passanti, zero fallimenti |
| XML | 96 MATCH, 13 EXPECTED, 0 UNEXPECTED, 0 SKIPPED |
| Audit iniziale | 29/29 stdout/status corrispondenti |
| bugs-fixed | 31/31 stdout/status corrispondenti |
| Corpus piccolo | 219 MATCH + 6 contratti, 0 differenze inattese |
| Driver completi | 33 eseguiti, 20 passanti, 13 inventariati, nessun timeout |

I numeri dei test Rust includono test contenitori: non vanno sommati ai casi
eseguiti al loro interno. Il workflow remoto dovrà essere verificato al primo run;
non è stato effettuato alcun push. I rapporti precedenti restano documenti storici.

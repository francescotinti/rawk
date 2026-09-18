# Consolidamento del porting AWK — 19 settembre 2026

Implementazione del piano approvato dall'utente dopo l'audit. Base Rust: `a198d90`; riferimento C: `5739fd7`, versione 20260426. Ambiente macOS, profilo a byte, `LC_ALL=C`. Nessun commit o push effettuato.

## Risultato

Le 29 sonde dell'audit iniziale corrispondono ora al C per stdout e codice di uscita. Le diagnostiche restano specifiche dell'implementazione. Il porting è più affidabile, ma non è ancora un sostituto completamente compatibile del C.

| Verifica | Risultato |
|---|---|
| Build release di entrambi i binari Rust | OK |
| Cargo fmt e Clippy con warning come errori | OK |
| Suite Rust | 79 test passanti, inclusi contenitori di molteplici casi |
| XML | 109 casi validati; 96 MATCH, 13 EXPECTED-DIVERGE, 0 inattesi, 0 saltati |
| Sonde audit iniziale | 29/29 stdout e status corrispondenti |
| Corpus `bugs-fixed` originale | 26/31 stdout e status corrispondenti; 5 differenze aperte |
| Gate completo | OK dopo rimozione dei metadati AppleDouble |

I conteggi non si sommano: un test Rust può contenere l'intera suite XML, numerosi casi differenziali o un ciclo property-based. Non sono percentuali di conformità AWK. La suite parallela è stata ripetuta per verificare la rimozione delle collisioni tra file temporanei.

## Interventi

1. **Harness:** processi isolati per directory e gruppo, timeout, raccolta separata di stdout/stderr, confronto a byte e status; riferimento C esplicito. Le annotazioni EXPECTED richiedono gli esatti risultati previsti per entrambi gli interpreti. Il test word-frequency ignora soltanto l'ordine delle righe, preservando duplicati e terminatori. Il gate propaga i fallimenti ed esegue l'intera suite.
2. **CLI ed errori:** `-f` conserva i file di input; assegnazioni ARGV applicate in ordine; safe mode effettivo anche per operazioni vietate in rami non eseguiti; controllo delle arità. Divisione per zero e funzione sconosciuta sono ora errori fatali con status 2. Formati malformati, indici negativi e uso incompatibile scalare/array producono diagnostiche nei casi coperti.
3. **Flusso:** propagazione uniforme di return, exit, next e degli errori attraverso espressioni, funzioni e cicli. Valutazione short-circuit; END dopo exit conserva il codice richiesto. Controlli statici dei contesti illegali.
4. **Input:** lettore condiviso tra ciclo e getline, RS modificabile senza perdere il buffer, contatori aggiornati alle letture effettive, assegnazioni di `$0` senza incremento NR/FNR, gestione EOF e errori di getline.
5. **Linguaggio:** tipi stringa/numerico più coerenti, prefissi numerici, CONVFMT, assegnazioni come espressioni, lvalue valutate una volta, range pattern, azioni vuote, escape, identificatori e precedenze. Array passati per riferimento con propagazione tra funzioni; parametri scalari locali.
6. **Regex e CSV:** selezione del match più lungo alla prima posizione tramite motore Rust a byte, cache dei separatori, split e conteggi sub/gsub corretti nei casi verificati. Pattern letterali evitano la costruzione superflua del DFA: il separatore storico di 10.000 caratteri passa anche in debug. CSV gestisce campi quotati, virgolette doppie e record multilinea. Test delle letture spezzate su diverse dimensioni del buffer.
7. **Validazione e documentazione:** corpus storico, inventario delle esclusioni, casi composti e generazione riproducibile di programmi limitati, benchmark e README aggiornato.

Non è stata delegata l'esecuzione del runtime al C. Il C è usato esclusivamente come oracolo nei test.

## Aspettative XML e differenze deliberate

Le 13 annotazioni esistenti/aggiornate coprono estensioni, byte NUL, escape, RNG, formattazione, ambiguità del parser e diagnostiche. Le snapshot del C rendono rilevabili variazioni nuove. I casi 0108 e 0109 sono passati da “warning e continua” a errore fatale sulla base dell'oracolo; i loro nomi file storici sono conservati. Gli spazi significativi negli expected sono codificati come `&#32;` e verificati senza trim. Le redirezioni usano nomi relativi nelle directory temporanee isolate.

## Corpus e limiti residui

Tutti i 31 file `.awk` di `bugs-fixed` sono eseguiti dallo script storico. Dei 26 casi corrispondenti, 22 positivi sono confrontati integralmente nel test Rust e 4 errori richiedono status 2, stdout vuoto e diagnostica senza panic.

| Caso storico | Differenza aperta |
|---|---|
| a-format | `%a` non implementato: Rust restituisce errore |
| subsep-overflow | `(i,j) in array` non ancora riconosciuto dal parser |
| inf-nan-torture | grafia dei valori Inf/NaN diversa |
| repetition-overflow | il C rifiuta `{256}`, il motore Rust lo accetta |
| fmt-overflow | OFMT ad altissima precisione: il C tronca a 255 caratteri; Rust produce la precisione completa |

Questi cinque casi non sono trasformati in successi del gate. Sono differenze nuove rilevate nell'estensione del corpus, distinte dalle 29 regressioni corrette.

`diary/c-corpus-inventory.json` elenca ogni file del corpus e il motivo dell'esclusione. I gruppi `testdir/t.*`, `p.*`, `T.*` e `tt.*` non sono stati importati integralmente: richiedono rispettivamente associazione delle fixture, adattamento dei driver shell e gestione dei carichi temporali. Il relativo risultato non è dichiarato passante. Questa resta una limitazione della copertura della fase 7.

Ulteriori limiti: formati printf dinamici `*`, completa equivalenza della sintassi regex, locale e Unicode. Il DFA ha limite di memoria di 4 MiB. I test negativi non dimostrano assenza di ogni possibile panic o esaurimento di risorse su input arbitrari. Safe mode applica restrizioni linguistiche; non è una sandbox.

La generazione in `compatibility.rs` usa seed 20260919, 64 casi, al massimo 19 record, valori e cicli limitati, e shrinking di proptest. Combina incrementi, array, funzioni e sequenze di record. Restano anche i sei property test numerici/stringa preesistenti.

## Prestazioni

Misure locali release, quattro esecuzioni per carico, mediana wall-clock con avvio del processo incluso; RSS da `/usr/bin/time -l`. Dati esatti in `benchmark-results.json`.

| Carico | C | Rust | Rapporto approssimativo |
|---|---:|---:|---:|
| Somma campi, 100.000 record | 0,041 s | 0,111 s | 2,7× |
| Campi regex, 10.000 record | 0,011 s | 0,022 s | 2,0× |
| Aggregazione array, 100.000 record | 0,048 s | 0,163 s | 3,4× |

RSS indicativamente 1,6–1,8 MB per C e 7–8,2 MB per Rust. Sono piccoli benchmark di questo ambiente, non stime universali. Il risultato non supporta le precedenti affermazioni di maggiore velocità del Rust; ottimizzare richiede un lavoro successivo guidato da profiling.

## Riproduzione ed evidenze

Dalla directory `rawk`, compilare prima `make -C ../c_awk`, poi eseguire `bash scripts/checks.sh`. Gli script `audit_regressions.py`, `historical_audit.py` e `benchmark.py` rigenerano i rispettivi JSON. Gli audit conservano anche rappresentazioni esadecimali degli stream per un confronto senza perdita di byte. I log conclusivi sono in `diary/verification-2026-09-19/`.

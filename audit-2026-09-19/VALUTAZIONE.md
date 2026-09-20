# Valutazione di rawk — 19 settembre 2026

Il progetto è un prototipo di interprete AWK con una base Rust abbastanza ordinata, ma **non è ancora un porting fedele né un sostituto affidabile dell'originale C**. Le lacune riguardano anche operazioni ordinarie, non soltanto estensioni o casi marginali. Le affermazioni del README («fully functional», «fully replicating», «fully compliant», «blazing-fast») superano le evidenze disponibili.

Audit del checkout `rawk` al commit `a198d90`, senza modificare sorgenti o test esistenti. La root contiene due repository separati. Il materiale storico attribuisce l'implementazione soprattutto a Gemini/Antigravity e l'architettura/audit a Claude; la valutazione riguarda il risultato concreto, indipendentemente dall'attribuzione.

## Verifiche eseguite

Ambiente locale macOS, Rust/Cargo 1.96.0; dipendenze vincolate dal Cargo.lock. Per i confronti aggiuntivi: `LC_ALL=C`, timeout di 3 secondi, directory temporanea isolata, confronto di stdout e codice d'uscita.

| Verifica | Risultato |
|---|---|
| `cargo build --release --locked` | OK, nessun warning Rust |
| `cargo test --locked` | **FAIL**, `diffrun_step23_target_counts`: 94 MATCH, 14 EXPECTED, 1 UNEXPECTED |
| `cargo test --locked -- --test-threads=1` | **OK: 41 test**, di cui un runner esegue 109 casi XML; 6 test sono property test |
| `cargo fmt --check` | OK |
| `cargo clippy --locked --all-targets -- -D warnings` | OK |
| `bash scripts/checks.sh` | 8 controlli OK, con limiti descritti sotto |
| `make` in `c_awk` | OK; Bison segnala 44 conflitti shift/reduce e 85 reduce/reduce; due warning C |
| `make check` in `c_awk` | Termina con status 0, **ma il log contiene BAD e differenze**: non va dichiarato interamente verde |
| Corpus XML differenziale contro il C appena compilato | 95 MATCH, 14 EXPECTED-DIVERGE, 0 UNEXPECTED, 0 SKIP |
| Sonde aggiuntive mirate | 27/27 differiscono dal C per output/status/timeout; altre 2 verificano `--safe` e CSV |

Il fallimento parallelo riguarda `test_redirect_overwrite_after_close_truncates`: tre istanze di diffrun scrivono sugli stessi file. Il risultato seriale conferma la diagnosi già annotata nel diario Step 23. Le 27 sonde sono scelte appositamente dopo l'ispezione dei punti sospetti: **non costituiscono una stima statistica della percentuale di compatibilità**.

L'AWK di sistema è versione `20200816`; l'originale fornito è `20260426`. Ho compilato quest'ultimo e ripetuto anche il corpus differenziale contro di esso tramite PATH temporaneo. La scritta `/usr/bin/awk` emessa da diffrun è hardcoded e resta tale anche quando il binario effettivo è diverso.

La suite C comprende confronti col vecchio AWK di sistema: i messaggi BAD/differ non identificano automaticamente bug nel C nuovo. Il log registra, fra gli altri, `t.printf2`, `T.arnold (system-status)`, segnalazioni di dominio matematico e differenze in test `tt.*`. I 31 script `bugs-fixed` non mostrano messaggi `failed!`. Non ho eseguito l'intera suite storica C contro rawk; ho usato il corpus del progetto e sonde isolate.

## Problemi prioritari verificati

P1 indica un problema da risolvere prima di proporre rawk come alternativa all'originale. P2 indica ulteriore incompatibilità o difetto dell'infrastruttura. Le righe si riferiscono al checkout auditato.

| Priorità | Problema e riproduzione | Punto nel codice |
|---|---|---|
| P1 | **`--safe` ignorato.** `rawk --safe 'BEGIN {system("printf SAFE_PROBE")}'` esegue il comando; il C con `-safe` lo rifiuta. Il campo viene dichiarato ma non consultato nel runtime. | `src/cli.rs:28`, `src/runner/builtins.rs:199` |
| P1 | **`getline` sull'input principale è errato e può bloccare il processo.** Con stdin `a\nb\nc\nd\n`, `{getline x; print $0,x,NR,FNR}` supera il timeout. Con un file come argomento legge invece da stdin vuoto e non consuma la riga seguente del file. | `src/runner/mod.rs:485` |
| P1 | **CLI `-f programma file` perde il primo file.** Il primo posizionale finisce in `program`, che il runtime ignora quando è presente `-f`. Con stdin vuoto non stampa nulla invece di elaborare il file. | `src/cli.rs:36`, `src/runner/mod.rs:100` |
| P1 | **`exit` salta `END` e il cleanup esplicito.** `BEGIN {exit 7} END {print "END"}` restituisce 7 ma non stampa END. I ritorni anticipati precedono anche `flush_and_close_all`. | `src/runner/mod.rs:137` e altri `return Ok(code)` in `run` |
| P1 | **Manca la valutazione corta di `&&`/`||`.** `BEGIN {x=0; print (0 && ++x),x}` produce `0 1`, il C `0 0`. Entrambi gli operandi sono valutati prima del dispatch dell'operatore. | `src/runner/mod.rs:652` |
| P1 | **La riscrittura del record altera i contatori.** `{$0=$0; print NR,FNR}` su due righe stampa `2 2` e `4 4`, invece di `1 1` e `2 2`. `update_record` mescola acquisizione del record e ricostruzione dei campi; è chiamata anche da sub/gsub. | `src/types.rs:372`, `src/types.rs:403` |
| P1 | **Tipizzazione AWK incompleta.** `"01" == "1"` restituisce 1 invece di 0; `"10" < "2"` restituisce 0 invece di 1. Il confronto converte anche stringhe esplicite in numeri, vanificando parte della distinzione String/StrNum. `"12x"+0` produce 0 invece di 12. | `src/types.rs:68`, `src/types.rs:114` |
| P1 | **Le funzioni non preservano array e controllo di flusso.** Passare `x` a `function f(a){a[1]=9}` non modifica `x[1]`; i parametri locali sono solo scalari. Un `return 7` dentro un while viene ignorato e si raggiunge il successivo `return 9`. | `src/runner/mod.rs:553`, `src/runner/mod.rs:768` |
| P1 | **Regex incompatibili con l'originale.** `match("ab",/a\|ab/)` dà RLENGTH=1 invece di 2. Una regex dinamica `"["` diventa silenziosamente la regex vuota e risulta vera invece di segnalare errore. Passare ai byte non risolve la scelta del match più lungo. | `src/types.rs:339`, `src/runner/builtins.rs:286` |
| P1 | **Separazione dei dati incompleta.** `FS="[,:]+"` è trattato letteralmente: `a,b:c` resta un campo anziché tre. Cambiare RS durante l'elaborazione non cambia il delimitatore già fissato per il file. | `src/types.rs:383`, `src/runner/mod.rs:432` |
| P2 | **`split` perde campi vuoti e conserva vecchie chiavi.** `split("a,,b",a,",")` restituisce 2 anziché 3; una precedente chiave 9 rimane. | `src/runner/builtins.rs:321` |
| P2 | **sub/gsub restituiscono un booleano di modifica anziché il numero di sostituzioni.** Su `aaa`, gsub(a,b) restituisce 1 anziché 3; sub(b,b) restituisce 0 anziché 1. | `src/runner/builtins.rs:366`, `src/runner/builtins.rs:387` |
| P2 | **Grammatica incompleta.** Falliscono stringhe con virgoletta escapata, pattern a intervallo `NR==1,NR==2`, assegnazioni nelle espressioni, `.5`, identificatori `_x`. `-2^2` produce 4 invece di -4. | `src/awk.pest:23`, `src/awk.pest:108`, ultime regole lessicali |
| P2 | **Panic su arità errata.** `BEGIN {print substr("a")}` causa index out of bounds e status 101 anziché un errore diagnostico del linguaggio. Diverse builtin indicizzano gli argomenti senza validazione. | `src/runner/builtins.rs:83` |
| P2 | **`--csv` è soltanto FS=virgola.** `a,"b,c",d` restituisce quattro campi; il C tre, con `$2` uguale a `b,c`. | `src/runner/mod.rs:63` |

Altre riproduzioni salvate: `$1++` non modifica il campo; `getline` da file inesistente restituisce 0 anziché -1; l'argomento `x=7` viene aperto come nome di file; `next` dentro una funzione è ignorato da rawk, mentre questa versione del C lo rifiuta. Quest'ultimo è un confronto fra implementazioni, non una dichiarazione di comportamento universale di tutti i dialetti AWK.

## Qualità dell'implementazione

**Aspetti positivi.** Enum per AST e valori, separazione di parser/runtime/I/O/formatter, ownership degli stream, assenza di blocchi unsafe nel codice del progetto, Cargo.lock, Clippy rigoroso e test specifici per NUL e byte alti. La migrazione da String a Vec<u8> evita corruzioni importanti sui dati binari. Sono basi utili da conservare.

**Limiti strutturali.** La gestione del flusso è divisa tra FlowControl e flag laterali (`exit_pending`, `nextfile_pending`), con propagazione non uniforme. `EvalContext` contiene insieme simboli, record, file, processi, regex, funzioni e formattazione; lettura e mutazione del record condividono effetti collaterali sbagliati. L'assenza di un'astrazione unica per l'input spiega i problemi di getline. L'assenza di riferimenti ai parametri array spiega i problemi delle funzioni.

`eval_expr` restituisce direttamente AwkValue, quindi errori e trasferimenti di controllo vengono convertiti in valori di ripiego, warning o flag. L'uso di Rust sicuro evita molte categorie di errori di memoria, ma non garantisce correttezza del linguaggio, assenza di panic o blocchi.

**Prestazioni.** La copia del corpo delle funzioni a ogni chiamata, le conversioni che allocano Vec e lo splitting immediato di tutti i campi sono costi plausibili da profilare. RS regex e modalità paragrafo leggono tutto l'input con `read_to_end`: memoria proporzionale al file e impossibilità di produrre risultati prima dell'EOF in quei percorsi.

Microbenchmark locale: 100.000 righe `1 2 3`, programma `{s += $2} END {print s}`, quattro esecuzioni per binario, output identico `200000`. Mediana C `0,0385 s`, Rust release `0,0575 s` (circa 1,49× il tempo C). È una misura indicativa su un solo carico breve, non un benchmark generale; non sostiene però l'affermazione di maggiore velocità del README.

## Perché i test verdi non bastano

1. **Il corpus copre solo una parte della semantica.** I sei property test generano soprattutto numeri e stringhe entro template semplici; non esplorano programmi composti, chiamate, scope, lvalue, lettura e controllo di flusso. Nei moduli di produzione non ci sono unit test: i target binari riportano zero test unitari.
2. **diffrun ignora stderr e status.** Confronta stdout convertito con `from_utf8_lossy` e spesso `trim()`: può nascondere alterazioni di whitespace, byte invalidi, errori o crash con output identico. Non controlla il valore atteso XML prima di classificare il confronto.
3. **Le divergenze annotate accettano qualsiasi differenza futura nello stesso caso.** Una nuova regressione può essere riclassificata EXPECTED solo perché il caso ha già un'annotazione. Non è una whitelist del comportamento esatto.
4. **Mancano isolamento e timeout.** Tre test eseguono lo stesso corpus nella medesima directory. Il runner differenziale non impone un timeout; il deadlock trovato può bloccare una verifica futura.
5. **`checks.sh` non esegue tutta la suite.** `check_tests` esegue solo il runner XML. Il diario dice che lo script usa i flag corretti per serializzare i test: in realtà evita il test problematico, non lo serializza.
6. **Il gate aggregato può dare successo dopo un fallimento.** `run_all` usa `$fn && echo OK` e non accumula lo stato. Ho sostituito le funzioni soltanto nella shell della prova: `check_fmt` falliva, gli altri passavano; risultato finale `aggregate_exit=0`. Nessuna modifica allo script su disco.

La documentazione registra bene i passaggi di lavoro ma confonde talvolta chiusura di uno step e conformità del prodotto. README cita ancora PrattParser e runner.rs, mentre la grammatica usa livelli espliciti e il runner è una directory. Rimane `src/scratch.rs`, escluso dalla build. La priorità futura indicata dal diario — ridurre clone e rendere stabile il conteggio MATCH — è secondaria rispetto agli errori funzionali emersi.

## Ordine di intervento proposto

1. Rendere affidabile la verifica: tempdir per caso, timeout, stdout binario, stderr e status, oracolo C configurabile, gate aggregato corretto; inserire le sonde confermate come regressioni.
2. Correggere safe mode e CLI; introdurre un lettore condiviso tra ciclo principale e getline; separare lettura del record da assegnazione/risplitting.
3. Uniformare errori e controllo di flusso, con propagazione anche dentro espressioni e funzioni; eseguire END e chiusura stream nel percorso appropriato.
4. Correggere coercioni, array per riferimento, grammatica, splitting e builtin; scegliere esplicitamente la semantica regex compatibile con il C.
5. Ampliare i test usando il patrimonio di `c_awk/testdir` e `bugs-fixed`; solo dopo, ottimizzare con benchmark rappresentativi.

**Giudizio finale:** buona base sperimentale e discreta organizzazione Rust; correttezza e validazione ancora insufficienti per parlare di porting completato. Il problema principale non è lo stile del Rust: è la semantica AWK che i test attuali non esercitano.

## Evidenze riproducibili

Nella cartella di questo rapporto:

- `probes.py`: riproduce le 27 sonde; `probes.json`: output, status ed errori osservati.
- `options.json`: verifica safe e CSV.
- `tests-serial.log`, `clippy.log`, `gates.log`: verifiche del progetto Rust.
- `diffrun-original-c.log`: corpus XML confrontato col C fornito.
- `c-regress.log`: risultato completo di make check sul C, inclusi BAD e differenze.
- `gate-failure-probe.log`: dimostrazione del falso successo aggregato.
- `benchmark.json`: tempi delle quattro esecuzioni e output.

Per ripetere le sonde dalla root: `python3 audit-2026-09-19/probes.py`, dopo aver compilato entrambi i binari. Build e suite C hanno generato i normali artefatti nelle rispettive directory; nessun commit o cambiamento ai sorgenti è stato effettuato.

# rawk

Porting sperimentale di AWK da C a Rust. Il riferimento di compatibilità è il sorgente nella cartella adiacente `c_awk`; il runtime Rust esegue autonomamente i programmi.

Il profilo verificato è orientato ai byte, con `LC_ALL=C`. La suite comprende il nucleo AWK e alcune estensioni (RT, BEGINFILE/ENDFILE, builtin temporali e bitwise). Non costituisce una certificazione di conformità POSIX o gawk. Stato e risultati aggiornati sono nel [rapporto di estensione del corpus](diary/2026-09-19-corpus-expansion.md); il [primo rapporto](diary/2026-09-19-remediation.md) conserva la situazione iniziale.

## Build e utilizzo

```bash
cargo build --locked --release --bins
printf 'foo,bar\n' | target/release/rawk -F ',' '{ print $2 }'
target/release/rawk -f programma.awk input.txt
target/release/rawk --csv '{ print $2 }' dati.csv
target/release/rawk --safe 'BEGIN { print "hello" }'
```

Safe mode blocca comandi di sistema, pipe e redirezioni di output e non espone ENVIRON. Non è una sandbox generale.

## Verifica

Compilare prima il riferimento C, dalla radice di questo repository:

```bash
make -C ../c_awk
cargo test --locked
bash scripts/checks.sh
python3 scripts/audit_regressions.py
python3 scripts/historical_audit.py
python3 scripts/corpus_audit.py
python3 scripts/driver_audit.py
python3 scripts/benchmark.py
```

`cargo test` e `diffrun` usano per default `../c_awk/a.out`; `RAWK_REFERENCE` permette di scegliere il riferimento dei test Rust. Il corpus storico richiede anche i sorgenti adiacenti in `c_awk/bugs-fixed`. Le annotazioni XML sono relative alla versione del C verificata, non a qualsiasi AWK installato.

```bash
target/release/diffrun tests/testsuite.xml --awk ../c_awk/a.out --rawk target/release/rawk
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Il confronto controlla byte di stdout, stderr e codice di uscita, in directory temporanee separate, con timeout. Le differenze deliberate hanno aspettative esplicite; un riferimento mancante è un errore. L'ordine delle righe è ignorato soltanto nei casi che lo dichiarano. Gli script Python di audit riportano separatamente il confronto stdout/status e conservano le diagnostiche.

## Architettura

- `awk.pest`, `parser.rs`, `ast.rs`: grammatica PEG, parsing e AST.
- `validation.rs`: vincoli statici, arità, safe mode e parametri array.
- `runner/`: interpretazione, builtin, formattazione e I/O.
- `types.rs`: valori, conversioni, scope e contesto di esecuzione.
- `input.rs`: lettura condivisa tra ciclo principale e getline, RS dinamico e CSV.
- `ere.rs`: ricerca a byte con scelta del match più lungo alla prima posizione; DFA Rust per i pattern non letterali.
- `test_support.rs`: infrastruttura condivisa di verifica.

## Limiti noti

Le cinque differenze inizialmente trovate in `bugs-fixed` sono corrette: 31/31 casi corrispondono per stdout e status. L'estensione a `testdir/p.*` e `t.*` dà 214 corrispondenze integrali su 225 programmi; gli 11 casi rimanenti sono elencati nel rapporto, distinguendo ordine degli array e RNG dalle lacune funzionali. I tre driver adattati T.argv, T.clv e T.delete passano con entrambi gli interpreti.

`cargo test` include i 214 casi verificati (stream, status e file prodotti), i 31 casi storici e i tre driver. Questi ultimi richiedono Python 3 e un compilatore C per l'helper echo originale. `corpus_audit.py --check` restituisce errore se incontra qualunque differenza, comprese quelle ancora aperte: non è presentato come un gate verde. I driver hanno adattamenti dichiarati a percorsi, fixture, ordine degli array e testo diagnostico; i sorgenti C originali non vengono modificati.

Il profilo adotta il limite C di 255 per le ripetizioni regex e di 255 byte per la conversione numerica tramite OFMT/CONVFMT. Il formatter `%a` segue le particolarità del riferimento Darwin; altri sistemi richiedono verifica dedicata. Formati printf dinamici `*`, locale/Unicode complete e l'intera sintassi regex del C richiedono ulteriore lavoro. Il limite di memoria del DFA è 4 MiB.

I benchmark locali mostrano Rust più lento e con maggiore memoria rispetto al C sui tre carichi misurati. L'ottimizzazione rimane una fase successiva alla compatibilità.

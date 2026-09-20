# rawk

Porting sperimentale di AWK da C a Rust. Il riferimento di compatibilità è il sorgente nella cartella adiacente `c_awk`; il runtime Rust esegue autonomamente i programmi.

Sono verificati il profilo orientato ai byte (`LC_ALL=C`) e il profilo UTF-8 su Darwin ARM64. La selezione segue `LC_ALL`, `LC_CTYPE`, `LANG`; dettagli nel [rapporto Unicode e locale](../diary/2026-09-19-unicode-locale.md). La suite comprende il nucleo AWK e alcune estensioni (RT, BEGINFILE/ENDFILE, builtin temporali e bitwise). Non costituisce una certificazione di conformità POSIX o gawk. Stato e risultati aggiornati sono nel [rapporto di chiusura della Fase 7](../diary/2026-09-19-phase7-closure.md); il [primo rapporto](../diary/2026-09-19-remediation.md) conserva la situazione iniziale.

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

Il corpus storico `bugs-fixed` corrisponde in 31/31 casi per stdout e status. Tutti i 225 programmi `testdir/p.*` e `t.*` sono nel gate: 219 confronti integrali e sei contratti specifici per ordine degli array, RNG e conservazione dei NUL. `corpus_audit.py --check` rifiuta ogni differenza non coperta da tali contratti, che fissano sorgenti, fixture e risultati ammessi e hanno prove negative.

Dei 33 driver shell, **27 sono verificati integralmente**. T.flags, T.misc, T.builtin e T.errmsg hanno contratti per le singole invocazioni; T.utf e T.utfre sono verificati separatamente in `en_US.UTF-8`, con tutti i loro 300 casi originali. I 275 sottocasi comprendono 173 confronti esatti, 99 differenze della sola diagnostica e tre differenze deliberate, tutte fissate con esiti precisi. Inventario, impronte degli input e prove negative impediscono che una nuova divergenza venga accettata automaticamente. Il rapporto completo conserva anche i fallimenti delle asserzioni storiche: `diary/full-driver-closure-results.json`.

Printf/sprintf supportano larghezza e precisione dinamiche `*`. Escape regex, ancore RS, sorgenti non UTF-8, precedenze, keyword e CLI hanno regressioni dedicate. Il profilo usa il limite C di 255 per le ripetizioni regex e di 255 byte per OFMT/CONVFMT, con cache DFA limitata a 4 MiB. Conversioni e classi POSIX ISO-8859-1/9 sono verificate su Darwin ARM64; le altre locale legacy e piattaforme richiedono una convalida separata; la CLI continua a richiedere nomi di file UTF-8.

I test richiedono Python 3, un compilatore C e l'oracolo originale. La CI è predisposta su macOS (gate completo) e Linux glibc x86-64/ARM64 (gate di portabilità), con revisioni fissate; lo stato delle esecuzioni remote va verificato su GitHub Actions. Entrambi i gate includono UTF-8 e verificano esplicitamente che le locale richieste siano attive. Il gate Linux non certifica i contratti storici registrati su Darwin. Stato e comandi nel [rapporto di portabilità](../diary/2026-09-19-portability.md). Risultati della Fase 7, copia pulita e benchmark sono nel rapporto di chiusura; il rapporto di consolidamento conserva le misure precedenti.

## Aggiornamento prestazioni — 20 settembre 2026

Ottimizzata la ricerca booleana UTF-8 (`~`, `!~`, pattern di regola): evita la
mappa degli offset e il calcolo del match più lungo. Nei tre carichi misurati
riduce i tempi del 12–41%; il profilo byte mostra variazioni di +1–3%.
Verifica: 126 test debug e release, gate completo verde. Misure, comandi e limiti
nel [rapporto prestazioni](../diary/2026-09-20-regex-performance.md).
Le altre codifiche legacy rimangono aperte; la verifica nativa Linux è sospesa in
assenza di runtime locale.

## Locale ISO-8859-1 e ISO-8859-9

Conversioni maiuscole/minuscole e classi regex POSIX seguono le tabelle della
locale selezionata, conservando unità a byte e NUL interni. I test confrontano
con il C tutti i 255 byte non nulli nelle due locale e richiedono che siano
installate. Dettagli nel [rapporto locale legacy](../diary/2026-09-20-legacy-locales.md).
Shift-JIS e le altre codifiche non sono incluse in questa convalida.

## Shift-JIS — profilo delimitato

In `ja_JP.SJIS` le conversioni upper/lower usano la libc della locale e
rifiutano sequenze invalide o troncate. Stringhe, regex in memoria, campi e
formattazione mantengono le unità strutturali BWK del C; `%c` numerico emette
UTF-8 anche qui. NUL interni e stampa atomica su errore rimangono estensioni
deliberate Rust. Verificato su Darwin ARM64, incluse 11.280 coppie valide per
entrambe le conversioni.

Il residuo nei separatori RS regex con sequenze strutturali di tre/quattro
byte è corretto: un automa dedicato riproduce i gruppi di lettura e i riavvii
di `fnematch`, senza cambiare le regex in memoria. La regressione è attiva;
non rimangono ignore per Shift-JIS. Rust conserva i byte FF anche nei casi
in cui l'oracolo Darwin fallisce passando un `char` negativo a `ungetc`.
Linux glibc 2.39 x86-64/ARM64 è ora verificato sui runner nativi: 12 test
Shift-JIS nel gate debug/release e 6.879 coppie valide confrontate con il C.
Upper riesce per tutte; lower riesce per 6.878 e restituisce l'errore previsto
`illegal wide character` per `81 f0`, come il C. La sonda controlla anche
questo esito; non scarta il caso. [Consegna Linux](../diary/2026-09-20-shift-jis-linux.md).
La convalida non si estende automaticamente ad altre piattaforme. [Correzione RS e verifiche](../diary/2026-09-20-shift-jis-rs.md).

Audit iniziale e piano di lavoro: [valutazione](../audit-2026-09-19/VALUTAZIONE.md) e [piano aggiornato](../audit-2026-09-19/PIANO_DI_LAVORO.md).

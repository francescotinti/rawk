# Estensione del corpus e chiusura delle differenze storiche

19 settembre 2026. Eseguiti i primi tre passi approvati: consolidamento Git, correzione delle cinque differenze storiche, ampliamento del confronto con il C. Il riferimento resta `c_awk` revisione `5739fd7`, profilo `LC_ALL=C`, macOS. Non sono stati eseguiti push.

## Stato salvato

Lo stato del primo audit è conservato in tre commit:

- `6d2fd04`: runtime e infrastruttura differenziale.
- `2147bc9`: test e script di regressione.
- `5fb2978`: documentazione, evidenze e benchmark iniziali.

Il rapporto precedente conserva i risultati di quella fase; il presente documento lo aggiorna senza riscriverne la storia.

## Cinque differenze risolte

| Area | Comportamento verificato |
|---|---|
| `%a` / `%A` | Formattazione esadecimale in Rust, segno, larghezza, precisione, padding, arrotondamento e valori subnormali prodotti aritmeticamente |
| Tuple `(i,j) in array` | Parsing, appartenenza e passaggio dei parametri array; stesso ordine di valutazione di SUBSEP usato per accesso e delete |
| Inf/NaN | Segno esplicito nelle conversioni numeriche, come il C |
| Ripetizioni regex | Rifiuto dei conteggi superiori a 255, senza confondere parentesi graffe letterali o classi con quantificatori |
| OFMT/CONVFMT | Limite di 255 byte per i normali formati numerici ASCII, corrispondente al buffer C di 256 byte incluso NUL |

La scelta è mantenere la compatibilità con il C anche per i limiti numerici. Le particolarità Darwin di `%a` (normalizzazione dei subnormali e arrotondamento dalla prima cifra esadecimale omessa) sono coperte dal confronto; gli altri sistemi operativi non sono stati verificati. Non viene usato il C per eseguire il runtime Rust.

Il corpus `bugs-fixed` passa da 26/31 a **31/31 per stdout e status**. Nei test permanenti, 26 programmi positivi richiedono uguaglianza degli stream e dello status; cinque programmi invalidi richiedono status 2, stdout vuoto e diagnostica senza panic. Il testo delle diagnostiche rimane specifico dell'interprete.

## Programmi piccoli originali

Importati tutti i **225 programmi `p.*` e `t.*`** per l'audit. Le convenzioni di input riproducono Compare.p e Compare.t: due passaggi di test.countries per p.*, uno di test.data per t.*. Ogni interprete lavora in una directory temporanea distinta. Si confrontano byte degli stream, status, timeout e file prodotti dalle redirezioni, senza trim o ordinamento generale.

Risultato: **214 corrispondenze integrali e 11 differenze**. I 214 casi sono fissati in `tests/corpus-verified.list` e rieseguiti da `cargo test`. Il manifest non si aggiorna automaticamente durante i test. L'audit completo continua a eseguire anche gli 11 casi aperti; `--check` restituisce errore finché resta una differenza.

Questo lavoro ha corretto anche:

- chiamate alle builtin con spazi prima della parentesi e `length` senza argomento;
- continuazioni di riga, inclusi CRLF, senza rendere valido un backslash isolato a fine file;
- assegnazione `**=` e corpo vuoto dei cicli;
- separazione dei pattern regex su righe consecutive;
- valore numerico restituito dal post-incremento di una stringa;
- `ARGV[0]`, ora derivato dal nome effettivo dell'eseguibile.

## Tre driver shell

**T.argv, T.clv e T.delete passano con entrambi gli interpreti** e sono inclusi nella suite Rust. Il runner compila l'helper echo.c originale, limita i processi e controlla sia status sia marcatori BAD/FAIL: il solo status del driver non basta.

Adattamenti applicati esclusivamente alla copia temporanea del driver:

- percorso dell'eseguibile nel valore atteso di ARGV[0];
- ordinamento soltanto nel caso esplicitamente dedicato all'iterazione di ARGV dopo delete;
- fixture locale al posto di /etc/passwd;
- accettazione delle due precise formulazioni diagnostiche per tre errori CLI, con aggiunta del controllo status 2.

Gli originali in c_awk non sono modificati. Gli altri 30 driver T.* e i 21 carichi tt.* restano da adattare; non sono conteggiati come passanti.

## Undici differenze ancora presenti nei programmi piccoli

| Casi | Evidenza e classificazione |
|---|---|
| p.43, t.in2, t.intest2 | Ordine dell'iterazione degli array differente. Non viene imposto l'ordine interno del C; nei test con output misto servirà un confronto per sezioni, senza ordinare indiscriminatamente tutto lo stream. |
| p.48b, t.randk | RNG diverso, già dichiarato nel profilo; campionamenti differenti. |
| t.printf2 | Byte NUL conservati da Rust in printf; differenza deliberata del profilo binario. |
| t.arith | Arrotondamento/formattazione numerica differente: esempio 1301.12 contro 1301.13. Da correggere. |
| t.gsub4, t.split3 | Classi regex con intervallo discendente accettate dal C ma rifiutate dal motore Rust. Da definire e implementare la compatibilità. |
| t.null0 | Confronto numerico di campi vuoti diverso. Da correggere. |
| t.sub0 | Escape della stringa di sostituzione dopo sostituzioni ripetute diverso. Da correggere. |

Sono quindi cinque programmi con lacune funzionali, cinque con ordine/RNG diversi e uno legato alla conservazione NUL. Il conteggio riguarda questi input, non una classificazione esaustiva del linguaggio.

Ulteriore caso osservato durante le prove `%a`: il C tratta il letterale subnormale `1e-308` come zero, mentre Rust lo mantiene. I test del formatter esercitano separatamente il subnormale prodotto da `1e-300/1e8`. La conversione dei letterali ai limiti di underflow resta da approfondire. Restano inoltre printf dinamico con `*`, locale/Unicode complete e il resto della sintassi regex.

## Verifica finale e riproduzione

Build release, fmt, Clippy con warning come errori, suite Rust e gate completi verificati. La suite contiene **85 test Rust**, alcuni dei quali eseguono molti casi: questi conteggi non vanno sommati ai 109 XML, ai 31 storici o ai 214 programmi importati.

- Audit iniziale: **29/29** stdout/status corrispondenti.
- XML: **96 MATCH, 13 EXPECTED-DIVERGE, zero inattese, zero saltati**.
- Corpus storico bugs-fixed: **31/31** stdout/status corrispondenti.
- Programmi piccoli: **214/225** confronti integrali.
- Driver shell adattati: **3/3**, con riferimento C anch'esso passante.

Dalla directory rawk:

```sh
bash scripts/checks.sh
python3 scripts/audit_regressions.py
python3 scripts/historical_audit.py
python3 scripts/corpus_audit.py
python3 scripts/driver_audit.py
```

`corpus_audit.py --full-output` conserva gli stream completi; normalmente il JSON registra lunghezza, SHA-256 e un'anteprima dopo aver confrontato integralmente i byte. `--check` rende bloccante ogni differenza, incluse le 11 ancora aperte. Nessuna differenza viene convertita in EXPECTED in base al solo fatto di essere già presente.

Evidenze aggiornate nei JSON di diary e nei log di `verification-corpus-2026-09-19/`. Il file `c-corpus-inventory.json` distingue casi importati, differenze, driver adattati e gruppi ancora non importati.

## Prossimo intervento consigliato

Correggere i cinque casi funzionali appena emersi, con priorità a campi vuoti, escape delle sostituzioni e classi regex; verificare poi formattazione e underflow. Successivamente estendere i driver T.* e aggiungere CI. I benchmark precedenti restano indicativi della necessità di profiling: in questa fase non sono state introdotte ottimizzazioni prestazionali né dichiarati miglioramenti di velocità.

# Chiusura della Fase 7 — 19 settembre 2026

**Stato: Fase 7 chiusa nel perimetro concordato.** Profilo: Darwin ARM64, `LC_ALL=C`, semantica
orientata ai byte. Il riferimento è il C originale, revisione
`5739fd79bcfc75ba7526773d0cf634521f8aca3c`, versione 20260426.
Questo rapporto aggiorna il [consolidamento precedente](2026-09-19-consolidation.md),
che rimane come evidenza storica. I conteggi non sono percentuali di conformità AWK.

## Interventi realizzati

- Regex: escape ottali, backspace `\b`, escape esadecimali, graffe e parentesi
  letterali. `split(s,a,/ /)` mantiene la semantica regex, distinta da `" "`;
  `//` conserva la divisione per byte. Le ancore di RS non vengono riapplicate
  dopo ogni record o dopo la compattazione del buffer.
- Grammatica: incrementi negli indici dei campi, precedenze con regex letterali,
  continuazioni di stringhe, confini delle parole chiave (`break_after` è un
  identificatore). Tutti i 114 sottocasi di T.expr coincidono con il C.
- Sorgenti: programmi inline, da file e da stdin accettano byte non UTF-8.
  Una rappresentazione interna reversibile evita di cambiare la parità dei
  backslash durante il passaggio al parser UTF-8.
- Builtin e valori: `length(array)`, valutazione degli argomenti eccedenti di
  length e funzioni utente, warning di atan2 con un argomento; controllo dei
  conflitti funzione/array e delle assegnazioni al contatore di for-in.
  I numeri integrali grandi seguono la precisione `%.30g` del C.
- CLI e runtime: `-d` senza valore, status 1 senza argomenti, warning e prosecuzione
  per opzioni sconosciute, errore per nextfile in BEGIN/END; le assegnazioni CLI
  con newline riproducono output e status del riferimento.
- Due vecchie aspettative XML corrette: l'uso di una funzione come valore causa
  errore; `print 1e21` coincide ora con il C. La stampa Rust resta atomica in caso
  di errore durante la valutazione degli argomenti, con contratto esplicito.

## Matrice dei driver e contratti

| Gruppo | Trattamento nel gate |
|---|---|
| 27 driver completi | Eseguiti integralmente da `tests/shell_drivers.rs` |
| T.expr | Anche 114 singoli confronti esatti, per localizzare le regressioni |
| T.flags, T.misc, T.builtin, T.errmsg | Invocazioni isolate e contratti individuali |
| T.utf, T.utfre | Fuori dal profilo Unicode; rimangono nell'audit informativo |

I 275 sottocasi individuali comprendono 173 confronti esatti, 99 differenze della
sola diagnostica e tre differenze deliberate già previste: nextfile nelle funzioni,
NUL nelle stringhe e algoritmo/seed iniziale del generatore casuale. Ogni differenza
fissa **entrambi** i risultati esatti: stdout, stderr, status, timeout e file cambiati.
Viene normalizzato soltanto il percorso/nome dell'eseguibile nelle diagnostiche.
Non basta che entrambi i programmi terminino con un errore.

`tests/closure-contracts.json` fissa anche l'inventario e le impronte degli input.
Le prove negative alterano ogni componente degli esiti, aggiungono/rimuovono casi
e cambiano le impronte: il gate deve rifiutarle. Nessun aggiornamento automatico
delle aspettative è previsto durante i test.

`extract_driver_cases.py` registra le invocazioni dei driver misti tramite il C;
`driver-subcases.json` conserva argomenti binari, stdin e fixture revisionati.
La riproduzione individuale non sostituisce la semantica delle pipeline shell:
la pipe chiusa di T.misc ha perciò un test separato che chiude il lettore prima
che il processo scriva, verificando errore 2 sia nel C sia in Rust.
Le aspettative Unicode di T.builtin sono esterne al profilo; le sue invocazioni
sono comunque confrontate nel locale C. I grep obsoleti di T.errmsg (compreso
il caso volutamente falso finale) non determinano l'esito dei contratti individuali.

L'adattamento aggiuntivo di T.arnold cambia esclusivamente l'aspettativa storica
`system-status` da 518 a 262 su Darwin, verificandola su entrambi gli interpreti.
Gli altri adattamenti restano quelli documentati nel rapporto precedente.
I 316 file originali di testdir, inclusi gli archivi, hanno impronte SHA-256
in `tests/reference-fixtures.json`. I sorgenti originali C non sono modificati.

## Verifiche e riproduzione

```sh
CARGO_INCREMENTAL=0 bash scripts/checks.sh
CARGO_INCREMENTAL=0 cargo test --locked --release
python3 scripts/closure_cases.py --rawk target/release/rawk --check --output /tmp/closure.json
python3 scripts/full_driver_audit.py --rawk target/release/rawk --output /tmp/drivers.json
```

L'ultimo comando è informativo: conserva i fallimenti delle asserzioni storiche
e dei driver Unicode, senza trasformarli in successi. Il gate usa il manifest dei
27 driver completi e i contratti individuali dei quattro driver misti.

- Build release, fmt, Clippy e gate completo: esito 0.
- Suite debug: **114 test superati**; suite release: **114 test superati**.
- XML: **97 MATCH, 12 EXPECTED, 0 UNEXPECTED, 0 SKIPPED**.
- Audit iniziale: **29/29**; bugs-fixed: **31/31** per stdout e status.
- Corpus piccolo: **219 confronti integrali + 6 contratti**, tutti verificati.
- Driver completi: **27/27 ammessi passanti**; sottocasi: **275/275 contratti soddisfatti**.
- Audit informativo dei 33 driver: 27 pass, 2 `open` (T.flags/T.misc, grep storici
  delle diagnostiche e NUL), 4 `reference-failure` (T.builtin/T.errmsg e i due
  driver Unicode). Queste etichette descrivono le asserzioni originali: le
  invocazioni dei quattro driver misti hanno il gate separato descritto sopra.
  Nessun timeout nell'audit finale.
- Copia pulita: esportati i file di lavoro senza `.git`/`target`, con manifest
  SHA-256 di **187 file di sorgente/configurazione/test**; oracolo C ottenuto
  da un clone e ricompilato. `checks.sh` termina con esito 0, **114 test**.
  Le impronte coincidono con i file finali. È una verifica della fotografia
  di lavoro, non di un commit pubblicato.

Log e metadati sono in [verification-phase7-closure-2026-09-19](verification-phase7-closure-2026-09-19/summary.json).
Il [manifest dei sorgenti](verification-phase7-closure-2026-09-19/source-snapshot.json)
esclude diario e README, aggiornati al termine delle misure.

## Prestazioni finali

Sette ripetizioni intercalate dopo riscaldamento, stesso input e stdout identico
al C. Il binario precedente ha lo stesso SHA-256 registrato nel benchmark di
consolidamento. Le misure vengono eseguite dopo la fine delle suite, senza altre
verifiche concorrenti avviate da questa sessione.

| Carico | C (s) | Rust precedente (s) | Rust finale (s) | Variazione | RSS finale (MiB) |
|---|---:|---:|---:|---:|---:|
| sum_fields (500000 record) | 0.1684 | 0.4452 | 0.4500 | +1.1% | 7.05 |
| regex_fields (50000 record) | 0.0253 | 0.0642 | 0.0645 | +0.4% | 7.80 |
| array_aggregation (500000 record) | 0.2101 | 0.6172 | 0.6410 | +3.9% | 7.09 |

Il costo delle correzioni è un aumento delle mediane fra 0,4% e 3,9% rispetto al
precedente binario ottimizzato; RSS sostanzialmente stabile. Sono misure locali,
non una stima della significatività statistica né una garanzia su altri carichi.
Rust rimane più lento del C. Nessuna ulteriore ottimizzazione è stata introdotta
per mascherare questo costo. Dati grezzi, impronte e tutte le ripetizioni:
[benchmark-phase7-closure.json](benchmark-phase7-closure.json).


## Limiti e attività successive

La chiusura riguarda il perimetro e i corpus concordati; non dimostra conformità
completa POSIX/gawk. Restano fuori locale/Unicode completi e altre piattaforme.
Il supporto ai byte nei programmi non estende la CLI a nomi di file non UTF-8.
Restano i limiti documentati delle regex, dei buffer numerici e le estensioni
esplicite della suite XML e del corpus piccolo. Non sono stati nascosti nuovi
difetti dietro esclusioni generiche.

Il workflow CI è predisposto e fissato per revisione/toolchain. Non è stata
eseguita CI remota e non è stato effettuato alcun push. Il suo esito andrà
verificato al primo avvio sul servizio remoto.

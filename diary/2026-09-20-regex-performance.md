# Regex UTF-8: ricerca booleana — 20 settembre 2026

Intervento sulle prestazioni eseguibile su Darwin ARM64. La convalida nativa
Linux resta sospesa per assenza di un runtime locale, come concordato.

## Modifica

Gli operatori `~`, `!~` e i pattern delle regole richiedono soltanto l'esistenza
di un match. Prima invocavano il percorso di ricerca posizionale UTF-8, con
costruzione della mappa degli offset e passaggio DFA per il match più lungo.
Ora codificano solo le chiavi dei caratteri e usano il localizzatore, condividendo
con la ricerca posizionale il controllo dell'allineamento a quattro byte.

`match`, `sub`, `gsub`, separatori e indici mantengono il percorso posizionale.
Il decoder è condiviso: non viene introdotta una seconda interpretazione dei
byte invalidi. La ricerca booleana richiede ancora memoria lineare per le chiavi;
non è una ricerca Unicode senza allocazioni.

## Regressioni

Il nuovo test confronta con il C operatori booleani, ricerca posizionale e regole
per alternanze, match vuoti, ancore, byte invalidi e corrispondenze non allineate.
In particolare le chiavi di `Āa` contengono la chiave di `𐀀` a un offset non
allineato: deve essere scartata, senza perdere un successivo match valido.
La conservazione dei NUL è verificata separatamente come estensione Rust.

## Benchmark riproducibile

`scripts/benchmark.py` mantiene il profilo byte predefinito e aggiunge
`--profile utf8`: ricerche booleane brevi/lunghe/senza match, più `match` e `gsub`
come controlli. Misure in release, riscaldamento, sette ripetizioni intercalate,
stdout uguale al C a ogni esecuzione. Input e binari hanno impronte nel JSON.
I benchmark sono eseguiti dopo la fine dei test, senza carichi di verifica concorrenti.

```sh
python3 scripts/benchmark.py --profile utf8 --scale 1 --runs 7 \
  --before /tmp/rawk-before-2026-09-20-regex \
  --output diary/benchmark-regex-utf8-2026-09-20.json
python3 scripts/benchmark.py --profile byte --scale 5 --runs 7 \
  --before /tmp/rawk-before-2026-09-20-regex \
  --output diary/benchmark-regex-byte-2026-09-20.json
```

Il binario `before` è stato copiato dalla release verificata prima delle modifiche.
Le mediane misurano questi carichi, non garantiscono un miglioramento generale.

## Codifiche legacy: ricognizione

Una sonda su en_US.ISO8859-1 e tr_TR.ISO8859-9 conferma differenze ancora aperte:
per il byte é (e9) il C produce É (c9) con toupper e lo riconosce in [:alpha:];
Rust conserva e9 e non lo riconosce nella classe. Un byte e9 isolato sotto
ja_JP.SJIS produce inoltre un errore di conversione nel C, non in Rust.
Questi risultati non sono test accettati come compatibili: delimitano il
successivo intervento funzionale. I byte, i programmi e gli esiti esatti sono
conservati in `verification-regex-performance-2026-09-20/legacy-probe.json`.

## Esiti finali

- Gate completo: exit 0, 126 test debug, fmt, Clippy, build release e igiene.
- Suite completa release: exit 0, 126 test; inclusi i 300 casi originali UTF-8.
- XML: 97 MATCH, 12 EXPECTED, 0 UNEXPECTED, 0 SKIPPED.
- `cargo check --locked --all-targets --target x86_64-unknown-linux-gnu`: exit 0.
  È un controllo dei tipi per il target, non una prova di linking o esecuzione Linux.
- Nessun push o lancio CI remoto. Nessuna modifica delle baseline di compatibilità.

| Carico | Prima (s) | Dopo (s) | Variazione |
|---|---:|---:|---:|
| utf8_boolean_short | 0.1946 | 0.1593 | -18.1% |
| utf8_boolean_long | 0.1169 | 0.0693 | -40.7% |
| utf8_boolean_miss | 0.1757 | 0.1540 | -12.3% |
| utf8_match | 0.1643 | 0.1662 | +1.2% |
| utf8_gsub | 0.2373 | 0.2368 | -0.2% |
| sum_fields | 0.4518 | 0.4588 | +1.5% |
| regex_fields | 0.0653 | 0.0662 | +1.4% |
| array_aggregation | 0.6220 | 0.6393 | +2.8% |

Il miglioramento riguarda i tre carichi booleani UTF-8 (12–41% di tempo in meno).
I controlli match/gsub variano di +1,2% e -0,2%; i carichi byte di +1,4%–2,8%.
Queste piccole variazioni non sono presentate come equivalenza statistica.
La mediana RSS non aumenta in nessun carico; nel caso booleano lungo scende
da 8.994.816 a 8.814.592 byte. Il runtime resta più lento del C in diversi carichi.

Dati completi: [UTF-8](benchmark-regex-utf8-2026-09-20.json),
[byte](benchmark-regex-byte-2026-09-20.json). Log, impronte e sonda legacy:
[verifica](verification-regex-performance-2026-09-20/summary.json).

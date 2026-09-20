# Audit dei driver originali su Linux nativo

## Obiettivo e stato

Audit dell'inventario originale su Linux glibc x86-64/ARM64, partendo dalle
[evidenze Shift-JIS](verification-shift-jis-linux-2026-09-20/). La CI precedente
eseguiva tutti i 33 driver senza `--check`: il suo successo non certificava
l'intero inventario. **Stato: completato nel perimetro verificato**, CI finale tutta verde.

## Diagnosi e classificazione

La raccolta iniziale nativa sul commit `78bd646` conferma, su entrambe le
architetture, 26 driver `pass`, quattro `open`, tre `reference-failure`.
Le etichette rappresentano le asserzioni dei driver, non una diagnosi.

| Driver/gruppo | Evidenza e classificazione | Verifica applicata |
| --- | --- | --- |
| 26 driver già passanti | Asserzioni complete superate sui due interpreti | Manifest originale, impronte e `--verified --check` |
| T.beebe | Bug runtime: riapertura di `/dev/stdout` come file, troncamento/offset indipendente su Linux; il sottocaso `messages` perde output | Driver completo e controesempi con stdout/stderr su file e pipe |
| T.builtin | RNG deliberatamente diverso, con due sequenze Rust e sequenze C fissate dal contratto | Singola invocazione, inclusi i file prodotti |
| T.flags | Quattro grep rifiutano il testo delle diagnostiche Rust | Contratti esatti di stdout, stderr, exit e file |
| T.misc | Diagnostiche diverse e conservazione deliberata del NUL; il driver invoca anche una funzione shell `error` inesistente quando un grep fallisce | Contratti delle singole invocazioni, senza accettare il solo exit della shell |
| T.errmsg | Diagnostiche diverse, estensione `nextfile` nelle funzioni e un controllo negativo intenzionale che fallisce anche in C; emersi inoltre due bug matematici Linux | Tutti i sottocasi, con correzione di `log(-1)` ed `exp(1000)` |
| T.utf / T.utfre in C | Aspettative Unicode incompatibili con `LC_ALL=C`; output, diagnostiche, exit e marcatori coincidono fra C e Rust | Conservazione dell'inventario grezzo; 300 casi verificati separatamente in UTF-8 |

Su macOS `T.builtin` attiva il ramo tedesco perché la locale è disponibile,
ma `LC_ALL=C` prevale sul solo `LANG` impostato dal driver: fallisce anche il C.
Su Linux quel ramo condizionale non si attiva. Non è una divergenza del runtime
introdotta dal task. I sottocasi estratti rimangono fissati, indipendentemente
dalle condizioni della shell. Nessun sorgente C o expected originale modificato.

La sonda iniziale esegue tutti i **275 sottocasi**: 171 coincidono esattamente,
99 differiscono soltanto nella diagnostica, tre sono differenze deliberate
(RNG, NUL, nextfile) e due sono bug matematici. Il confronto con i contratti
Darwin rifiuta precisamente `T.errmsg/49` e `/50` su entrambi i runner.
L'unica differenza fra architetture nelle firme dei sottocasi è il segno del NaN
Rust nel primo caso, prima della correzione; non viene codificato come expected.

## Modifiche e decisioni

- `src/runner/io.rs`, `src/runner/builtins.rs`, `src/types.rs`: i nomi esatti
  `/dev/stdout` e `/dev/stderr` usano gli stream già aperti, per `>` e `>>`.
  Anche `printf` senza newline mantiene l'ordine. `close` esegue flush e
  ricollega il descrittore a `/dev/null`, come `closefile` dell'oracolo;
  la successiva riapertura e i processi figli vedono il descrittore corretto.
  `fflush` riconosce gli alias iniziali e ne rispetta la chiusura. I comandi
  pipe e gli altri nomi di file mantengono il percorso ordinario.
- Le sonde minime rivelano ordine errato con `printf`, close e fflush anche
  su Darwin, benché il driver `T.beebe` a newline complete fosse verde lì.
  La regressione prima della modifica fallisce ed è conservata.
- Su Linux glibc `log`, `exp`, `sqrt` chiamano la libm nativa e controllano
  `errno`: EDOM/ERANGE producono avviso e valore `1`, come `errcheck` del C.
  Darwin mantiene il percorso precedente. La matrice mirata copre anche zero,
  valori negativi, underflow/subnormali e valori finiti, verificando il reset
  dell'errore con una chiamata valida successiva. Non generalizza cast numerici
  fuori intervallo né altri operatori matematici.
- `tests/driver_regressions.rs`: regressioni degli stream, incluso NUL/FF e
  processi figli, e matrice matematica contro l'oracolo nativo.
- `scripts/closure_cases.py` e `tests/closure-contracts-linux-glibc.json`:
  due contratti Linux per le sole differenze del contesto diagnostico di
  log/exp **dopo** il fix. Stdout `1`, status 0, avvisi e file sono fissati
  esplicitamente per entrambi gli interpreti. Gli altri 273 contratti non
  cambiano; profili Linux non convalidati vengono rifiutati.
- `scripts/test_closure_contracts.py`: prove negative sui contratti di entrambe
  le piattaforme; mutazioni di stdout/stderr/status/file, timeout, inventario,
  impronte e duplicazione di casi devono fallire.
- `scripts/linux_driver_probe.py`: conserva risultati dei 275 sottocasi e
  sonde degli stream, revisioni, piattaforma, libc e SHA-256 dei binari.
  `--check` impone i contratti nativi e l'uguaglianza delle sonde.
- Workflow: controlli bloccanti in **debug e release** per i 27 driver del
  manifest, i 275 sottocasi e le sonde degli stream. L'inventario grezzo dei
  33 driver rimane disponibile senza riscrivere i suoi fallimenti storici.

Preservati home/autori, byte alti/NUL, stampa atomica, percorsi BWK/libc/RS,
correzioni printf di glibc e contratti deliberati. Nessun expected originale
modificato per ottenere verde. Altre codifiche, macOS Intel, ottimizzazioni e
compatibilità generale POSIX/gawk restano fuori perimetro.

## Verifiche

Oracolo `5739fd79bcfc75ba7526773d0cf634521f8aca3c`; sorgenti invariati.
La checkout C conserva differenze di permessi e artefatti di compilazione.

Comandi principali locali (Darwin ARM64):

```sh
make -C ../c_awk
CARGO_INCREMENTAL=0 cargo test --locked --test driver_regressions
CARGO_INCREMENTAL=0 cargo test --locked --release --test driver_regressions
python3 scripts/test_closure_contracts.py
CARGO_INCREMENTAL=0 bash scripts/checks.sh
CARGO_INCREMENTAL=0 cargo test --locked --release
```

La [raccolta iniziale nativa](https://github.com/francescotinti/rawk/actions/runs/35509952804)
è verde pur conservando i difetti, perché la nuova sonda iniziale è diagnostica.
La [CI delle correzioni](https://github.com/francescotinti/rawk/actions/runs/35510218505)
esegue invece i nuovi controlli bloccanti: tutti i job sono verdi.

Evidenze in [verification-linux-drivers-2026-09-20](verification-linux-drivers-2026-09-20/).
I gate locali aggregati conservano exit 1 dovuto soltanto al controllo
AppleDouble; i log originali non sono stati alterati. I metadati identificati
sono stati rimossi, con elenco, e il controllo di igiene riverificato
separatamente. Le suite finali sono state eseguite in sequenza, senza
ricompilazioni sovrapposte alle suite che usano gli stessi binari.

Risultati Linux finali sul commit `b1adc82`: **89 debug + 89 release per
runner**, nessun fallimento o ignorato; fmt, Clippy e build release passati.
In ogni profilo: **27/27 driver**, **275/275 contratti** (171 esatti,
101 diagnostiche, tre deliberate), **20/20 sonde stream** uguali al C.
Shift-JIS mantiene tutte le 6.879 coppie verificate. L'inventario grezzo
riporta 27 pass / 3 open / 3 reference-failure, coerenti con la classificazione.

Darwin ARM64 locale: **151 test debug + 151 release**, nessun fallimento o
ignorato; release exit 0. Gate funzionale, fmt, Clippy e XML **97 MATCH /
12 EXPECTED / 0 UNEXPECTED / 0 SKIPPED** passati. Sonda finale locale:
275 contratti e 20 sonde stream superati.

macOS CI: **151 test debug**, gate completo e audit originali/UTF-8 verdi;
XML invariato. Il job macOS non esegue l'intera suite release, verificata
localmente. Non dedurre una suite release remota dalla build release.

[Riepilogo e impronte](verification-linux-drivers-2026-09-20/summary.json),
[classificazione dei 33 driver](verification-linux-drivers-2026-09-20/classification.json),
[esito CI finale](verification-linux-drivers-2026-09-20/ci-final.json).

## Git e pubblicazione

Base `8e1c2af`, branch `master`. Raccolta diagnostica `78bd646`, correzioni e
contratti `b1adc82`, pubblicati e verificati sul branch remoto.
CI finale verificata sul runtime `b1adc82`. Il commit conclusivo contiene
soltanto documentazione/evidenze e usa `[skip ci]`; hash effettivo e verifica
remota sono riportati nella risposta di consegna. Non sono necessarie altre
suite runtime per queste sole modifiche documentali.

## Passaggio al prossimo task

La copertura riguarda il manifest e i contratti espliciti dell'oracolo fissato,
non ogni possibile programma AWK. Per riprodurre: workflow compatibility,
`full_driver_audit.py`, `closure_cases.py`, `linux_driver_probe.py` e
`tests/driver_regressions.rs`. I log storici restano immutati.

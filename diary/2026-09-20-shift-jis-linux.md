# Shift-JIS sui runner Linux nativi

## Obiettivo e stato

Convalidare `ja_JP.SJIS` su Linux glibc x86-64 e ARM64 con l'oracolo nativo
fissato a `5739fd79bcfc75ba7526773d0cf634521f8aca3c`.
**Stato: completato nel perimetro verificato**, CI finale tutta verde.

Prima di questo intervento il gate Linux non selezionava `tests/shift_jis.rs`
e il workflow non generava la locale; la CI verde precedente non ne certificava
il comportamento.

## Modifiche e decisioni

- `.github/workflows/compatibility.yml`: genera `ja_JP.SJIS` con il charmap
  `SHIFT_JIS`. `--no-warnings=ascii` disabilita soltanto l'avviso previsto per
  questa codifica non compatibile ASCII; gli errori di `localedef` restano fatali.
- `scripts/shift_jis_locale_probe.c`: preflight nativo di `setlocale`, codeset,
  `MB_CUR_MAX=2`, decodifica, conversione maiuscola, ricodifica, input incompleto
  e input invalido/EILSEQ. Non assume che il valore di `wchar_t` su Darwin
  sia uno scalare Unicode; verifica i byte prodotti dalla conversione.
- `scripts/check_portability.sh`: include esplicitamente la suite Shift-JIS (ora 12 test)
  sia in debug sia in release. Nessun test esistente rimosso o ignorato.
- `scripts/shift_jis_platform_probe.py`: scopre sul runner tutte le coppie di
  due byte riconosciute dalla libc e confronta upper/lower con il C locale.
  Registra piattaforma, libc, codeset, revisioni e impronte; fallisce su divergenze.
  Non impone il numero di coppie osservato su Darwin alla libc Linux.
- Le nuove evidenze sono conservate negli artefatti CI e nella
  [directory del task](verification-shift-jis-linux-2026-09-20/).

Nessuna modifica al runtime, ai sorgenti dell'oracolo,
agli expected o alla home. Preservati stampa atomica, NUL, byte alti e percorsi
BWK/libc/RS distinti. L'inventario completo dei driver Linux resta diagnostico,
senza `--check`; non viene presentato come equivalenza dell'intero corpus.

## Diagnosi della sonda estesa

La prima attivazione della suite passa su entrambi i runner: 86 test debug e
86 release, nessun fallimento o ignorato. La sonda estesa iniziale invece
fallisce su entrambi: glibc 2.39 riconosce 6.879 coppie, ma la conversione
minuscola di `81 f0` produce un carattere non ricodificabile in Shift-JIS.
C e rawk restituiscono entrambi exit 2 e `illegal wide character`, con lo stesso
stdout. Il problema era l'assunzione della sonda che ogni coppia decodificabile
avesse una conversione ricodificabile; non una divergenza del runtime.

La sonda finale separa le conversioni ricodificabili e confronta individualmente
anche quelle che devono fallire, controllando exit, stdout e classe diagnostica.
Nessuna coppia viene scartata per ottenere verde. La nuova regressione
`decoded_character_with_unencodable_case_mapping_matches_c` confronta upper e
lower con il C nativo; l'helper riconosce distintamente gli errori di decodifica
e ricodifica. Il comportamento Darwin rimane ricavato dal proprio oracolo.
Le evidenze iniziali sono conservate in `initial-arm64` e `initial-x86_64`.

## Verifiche

Comandi locali eseguiti su Darwin ARM64:

```sh
cc -std=c11 -Wall -Wextra -Werror scripts/shift_jis_locale_probe.c -o /tmp/rawk-sjis-locale-probe
/tmp/rawk-sjis-locale-probe
bash -n scripts/check_portability.sh
CARGO_INCREMENTAL=0 cargo test --locked --test shift_jis
CARGO_INCREMENTAL=0 cargo test --locked --release --test shift_jis
python3 scripts/shift_jis_platform_probe.py --output diary/verification-shift-jis-linux-2026-09-20/darwin-pairs.json
```

Debug e release locali: 11/11 iniziali, poi nuova regressione 1/1 in entrambi i profili, nessun ignorato. Sonda locale: 11.280 coppie valide,
upper/lower coincidenti con l'oracolo. Verifiche complete native eseguite in CI; nessuna convalida Linux dedotta da
cross-compilazione.

CI finale sul commit `3b9facfebb6e29f4399f330be3ca7fbbfe591663`:
[run 35508846617](https://github.com/francescotinti/rawk/actions/runs/35508846617).
Linux glibc 2.39 x86-64 e ARM64: **87 debug + 87 release per runner**, zero
fallimenti e ignorati; fmt, Clippy e build release passati. Tutte le **6.879**
coppie valide confrontate: upper 6.879 successi, lower 6.878 successi più
l'errore di ricodifica atteso e verificato su `81 f0`. Nessuna divergenza inattesa.

macOS nella stessa CI: **149 test debug**, zero fallimenti e ignorati; gate
completo, fmt/Clippy, build release, XML **97 MATCH / 12 EXPECTED / 0 UNEXPECTED /
0 SKIPPED** e audit originali/UTF-8 passati. La CI macOS non esegue l'intera
suite release: localmente sono stati riverificati i test Shift-JIS release.
[Riepilogo e impronte](verification-shift-jis-linux-2026-09-20/summary.json),
[esito CI](verification-shift-jis-linux-2026-09-20/ci-final.json),
[log CI](verification-shift-jis-linux-2026-09-20/ci-final.log).

Il runtime non è cambiato: le suite complete locali già verdi non sono state
ripetute; sono stati eseguiti i test Shift-JIS pertinenti e i gate remoti.
I log delle verifiche iniziali restano intatti, compreso il fallimento della
sonda alla run 35508673531. I log grezzi conservano anche spazi finali e righe
vuote: il controllo whitespace è applicato ai documenti, senza riscrivere le
evidenze. Rimossi 50 metadati AppleDouble generati sul volume esterno;
`check_no_macos_forks` riverificato con exit 0.

## Git e pubblicazione

Base `4071216`, branch `master`. Commit infrastruttura `4ed62b2`, aggiunta
sonda esaustiva `2c21463` e verifica degli errori di ricodifica `3b9facf`, tutti pubblicati. Nessuna modifica runtime.
CI finale verificata tutta verde su `3b9facf`. Il commit conclusivo contiene
soltanto documentazione/evidenze e usa `[skip ci]`; hash effettivo, verifica
remota e stato della working tree sono riportati nella risposta di consegna.
Nessun nuovo test runtime richiesto dalle sole modifiche documentali.

## Passaggio al prossimo task

Altre codifiche, macOS Intel nativo e generalizzazione dei cast numerici fuori
intervallo restano esclusi. Per riprodurre il perimetro: workflow compatibility,
`scripts/check_portability.sh`, le due nuove sonde e `tests/shift_jis.rs`.

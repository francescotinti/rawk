# Priorità 4 — misure prestazionali native

## Obiettivo e stato

Tranche autorizzata sulla consegna `6068e1f`: misure Linux x86-64/ARM64 e
macOS Intel/ARM64, corpus più ampio e monitoraggio riproducibile. Il runtime
resta `3649297`; nessuna modifica semantica o degli expected. Misure e stato
finale sono riportati nelle sezioni successive.

## Modifiche e decisioni

- `scripts/prepare_native_benchmark.py`: esportazione Git dei due commit,
  build release isolate e sequenziali con Rust/Cargo 1.96.0, registrazione
  ambiente, compilatore, flag, runner e hardware.
- `scripts/benchmark_native.py`: 26 carichi deterministici, oracolo C esplicito,
  confronto esatto stdout/status/stderr, warmup, nove ripetizioni intercalate
  per due sessioni, distribuzioni wall/RSS, hash di binari/input/output,
  intervalli bootstrap appaiati e conservazione dei rapporti parziali su errore.
- `scripts/record_benchmark_oracle.py`: provenienza del C compilato in checkout
  separata, fissato a `5739fd79bcfc75ba7526773d0cf634521f8aca3c`.
- `scripts/summarize_native_benchmark.py`: tabelle riproducibili, rifiuto di
  rapporti incompleti, nessun confronto di tempi assoluti tra host.
- `.github/workflows/performance.yml`: dispatch manuale sulle quattro
  piattaforme native; architettura verificata, build prima delle misure,
  artefatti anche su errore. Nessuna automazione periodica dell'app.
- `scripts/test_benchmark_native.py`: test del confronto, RSS reale file/pipe,
  exit nonzero, stderr, timeout, output divergente e rapporto incompleto.
- [Guida operativa](../docs/PERFORMANCE.md), stato e piano canonico.

Il corpus riusa gli strumenti storici senza cambiarli. Aggiunge aggregazione
con 50.000 chiavi, CSV misto con quoting/CRLF/newline e output formattato;
conserva righe corte da 128 MiB e casi RSS sfavorevoli. Include campi misti,
UTF-8, regex dinamiche, sostituzioni, miss, file/pipe/getline, EOF, record lunghi,
RS paragrafi/regex. È un corpus sintetico più vario, non una stima della frequenza
dei carichi degli utenti.

Confronto principale: `61009df` → `3649297`, ricostruiti entrambi con 1.96.0
sullo stesso runner. Quantifica la priorità 3 e misura i carichi precedenti
come controlli; non pretende di rimisurare l'effetto isolato delle priorità 1–2.
Secondo dispatch: `3649297` → `3649297`, controllo del rumore su altri runner.
Le misure locali storiche con 1.98.1 non entrano nel calcolo delle variazioni.
Cargo.toml e Cargo.lock sono identici tra i due commit principali.

Gli intervalli sono esplorativi, con pochi campioni e molti confronti. Un segnale
concorde nelle due sessioni è candidato a verifica, non una regressione certificata.
Sessioni consecutive non sono repliche indipendenti su host diversi. Non si
introduce una soglia percentuale rigida. Il controllo della stessa revisione
mostra quanto il metodo possa segnalare effetti anche senza cambiamenti al codice.
RSS è il picco residente per processo, non conteggio delle allocazioni.

Preservati README/autori, contratti RS/getline/pipe/stream e differenze deliberate
byte/NUL/locale. La differenza preesistente RS regex EOF/`$0` in END resta aperta;
il carico di prestazione somma le lunghezze durante le azioni senza normalizzarla.
L'oracolo locale, con le sue modifiche preesistenti, non viene ricompilato né modificato.

## Verifiche dell'harness

Ambiente locale confermato: Rust/Cargo 1.98.1, stable ARM64 predefinita senza
override. Lo smoke locale usa binari già presenti e controlla soltanto i tre
nuovi carichi contro il C: non è una nuova misura prima/dopo né una baseline.
Cinque test locali passano, inclusi percorsi di errore reali, timeout del gruppo
di processi, output intenzionalmente errato con JSON incompleto, locale assente,
rotazione/warmup e rendering. La modalità `test_only` li esegue senza build o
benchmark: [CI harness finale](https://github.com/francescotinti/rawk/actions/runs/35534715831)
verde, **5 test su ciascuna delle quattro piattaforme** sul commit `a9659e1`.

Le misure principali e il controllo della stessa revisione usano l'harness
`0ed769d`. Il seguito `a9659e1` aggiunge il controllo preventivo della locale,
i test di errore/completamento e il renderer, senza cambiare il metodo cronometrato.
Le locale delle misure erano già predisposte dai runner (Linux: localedef
esplicito); l'hash degli output Unicode è conservato per tutti i profili.
I test runtime locali completi non sono ripetuti: il runtime non è cambiato.
La CI di correttezza scattata al primo push resta separata dalle misure.

[CI di correttezza](https://github.com/francescotinti/rawk/actions/runs/35533876697)
completamente verde sul commit infrastrutturale `0ed769d`, runtime invariato
rispetto a `3649297`: 160 debug + 160 release per runner macOS, 98 + 98 per
runner Linux, zero fallimenti o ignorati. I conteggi sono estratti dai log;
verdi anche gli step driver, stream e codifiche. Il seguito dell'harness usa
il dispatch `test_only` per verificare i cambi pertinenti senza ripetere i gate
runtime già verdi. La consegna documentale finale usa `[skip ci]`.

## Risultati nativi ed evidenze

Stato: **priorità 4 completata**. [Confronto principale](https://github.com/francescotinti/rawk/actions/runs/35533876766)
e [controllo della stessa revisione](https://github.com/francescotinti/rawk/actions/runs/35533894243)
verdi su tutte e quattro le piattaforme. Ogni report contiene 26 carichi ×
2 sessioni × 9 ripetizioni × 3 binari: 1.404 campioni conservati, **11.232**
nei due dispatch. Warmup esclusi. Input e output hanno hash identici per lo
stesso carico in tutti gli otto report; nessuna divergenza è normalizzata.

Rust/Cargo 1.96.0 in tutte le build; C GCC 13.3.0 su Ubuntu 24.04/glibc 2.39,
Apple clang 17.0.0 su macOS 15.7.9. macOS ARM64: Apple M1 virtuale, 7 GiB;
Intel: i7-8700B, 14 GiB. CPU, memoria e immagini Linux sono registrate nei
metadata originali. Queste macchine non coincidono col M4 locale storico.

Variazioni percentuali del tempo appaiato, **sessione 1 / sessione 2**:

| Piattaforma | Record 1 MiB | Record 8 MiB | CSV lungo | 128 MiB, righe corte | 128 MiB, record 64 KiB |
|---|---:|---:|---:|---:|---:|
| Linux x86-64 | −93,8 / −93,7 | −98,9 / −98,9 | −90,5 / −90,6 | **+5,5 / +4,1** | −44,3 / −44,6 |
| Linux ARM64 | −92,6 / −92,6 | −98,5 / −98,5 | −93,7 / −93,7 | −0,2 / −0,3 | −45,1 / −45,6 |
| macOS ARM64 | −92,5 / −93,3 | −98,7 / −98,6 | −94,2 / −93,3 | −1,0 / −2,4 | −53,4 / −49,0 |
| macOS Intel | −88,9 / −88,4 | −97,4 / −97,6 | −87,1 / −86,7 | −3,1 / +0,6 | −35,5 / −34,6 |

Il beneficio dei record lunghi è ampio in entrambe le sessioni di ogni
piattaforma. Non si calcola una media fra architetture. Su righe corte Linux
x86-64 emerge un costo concorde: +4,4/+5,5% nel caso da 16 MiB e +5,5/+4,1%
nel caso da 128 MiB (circa 80–97 ms in più rispetto a una mediana precedente
di 1,76 s). È il candidato prioritario per diagnosi e conferma su un'altra
esecuzione prima di attribuirgli una portata generale. I controlli della
stessa revisione x86-64 non mostrano un segnale concorde di rallentamento.
Sempre su Linux x86-64, `regex_fields` +2,2/+1,7% e `utf8_dynamic_boolean`
+0,5/+1,2% sono effetti assoluti di circa 0,2–0,3 ms, da trattare con prudenza.
Tutti gli altri casi, compresi quelli inconcludenti, sono nelle tabelle complete.

RSS non universalmente inferiore. I dati Linux restano generalmente vicini;
sul M1 virtuale diminuisce nei grandi record/CSV, ma aumentano i picchi mediani
nei controlli paragrafi (+16 KiB in entrambe le sessioni) e output formattato
(+80/+112 KiB). Sul Mac Intel il CSV varia fra −2,91 e −0,35 MiB di differenza
mediana. Nessuna di queste osservazioni cancella gli aumenti storici locali
M4 di +1,94 MiB (record 1 MiB) e +1,38 MiB (CSV), ottenuti con un altro
ambiente/toolchain. Distribuzioni residenti e conteggi delle allocazioni
restano grandezze distinte.

Il controllo della stessa revisione produce **binari byte-identici** su
ciascuna piattaforma. Nessun segnale wall concorde sui due Linux o su Intel;
un falso segnale `repeatable_faster` per `long_getline` su M1 (−13/−19% nei
rapporti appaiati, differenza fra mediane circa 3 ms su 25–30 ms). Nessun
segnale RSS concorde nei quattro controlli. Questo caso dimostra perché
anche un intervallo che esclude 1 in entrambe le sessioni non prova da solo
una variazione causata dal codice. Non si impone una soglia percentuale né
si trasforma il segnale esplorativo in un gate automatico.

- [Tutti i risultati principali, tempi assoluti e RSS](verification-native-performance-2026-09-20/main-summary.md).
- [Tutti i controlli della stessa revisione](verification-native-performance-2026-09-20/control-summary.md).
- [Integrità, conteggi e segnali degli otto report](verification-native-performance-2026-09-20/integrity-summary.json).
- JSON e log di build/misura originali: sottocartelle `main/` e `control/`
  nella [cartella delle evidenze](verification-native-performance-2026-09-20/).
- [CI correttezza](verification-native-performance-2026-09-20/correctness-ci.json),
  [conteggi estratti](verification-native-performance-2026-09-20/correctness-counts.json),
  [CI harness](verification-native-performance-2026-09-20/harness-ci.json),
  [test locali](verification-native-performance-2026-09-20/harness-tests.log),
  [preservazione dei sorgenti](verification-native-performance-2026-09-20/preservation-check.json).

## Git e pubblicazione

Branch `master`, base `6068e1f`, inizialmente pulita. Infrastruttura pubblicata
in `0ed769d1b18e0b076d52cb8e926ce30d136e5ff6`; protezioni, test finali e renderer
in `a9659e1355d07abf9fe07c0991d0135cb3a33ed2`. I relativi push sono riusciti.
Questa consegna aggiunge soltanto documentazione ed evidenze, con `[skip ci]`;
hash del commit finale, conferma remota e working tree sono verificati dopo
il commit e riportati nella risposta conclusiva. Nessuna modifica al runtime
rispetto alla base, nessuna modifica ai 424 file tracciati dell'oracolo locale.
Link locali verificati e diff dei documenti pulito. Gli spazi finali presenti
nei log originali della CI sono conservati, senza riscrivere le evidenze.
AppleDouble esclusi dai commit; rimossi soltanto dopo controllo della firma
00051607, senza attraversare `.git` o `target`.

## Passaggio al prossimo task

Le priorità 1–4 sono completate nel perimetro documentato. Rimangono candidati
separati: confermare e diagnosticare il costo delle righe corte su Linux x86-64;
diagnosticare la differenza RS regex EOF/`$0` già documentata. Nessun fix runtime
implicito in questa consegna. Per monitorare un nuovo candidato, leggere
AGENTS.md, STATO_PROGETTO.md e [PERFORMANCE.md](../docs/PERFORMANCE.md), poi
eseguire il dispatch con due commit espliciti, stessa toolchain e controllo
della stessa revisione quando serve. I risultati non promettono prestazioni
su altri hardware, versioni OS, compilatori o carichi reali non misurati.

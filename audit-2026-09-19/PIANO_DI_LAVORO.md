# Piano di lavoro rawk

Stato: **Fasi 1–7 completate nel profilo Darwin ARM64, orientato ai byte, `LC_ALL=C`**. Evidenze e limiti nel [rapporto di chiusura](../diary/2026-09-19-phase7-closure.md). Il testo seguente conserva il piano approvato e gli aggiornamenti storici.

## Obiettivo e confini

Correggere i problemi confermati dall'audit, rendere affidabile la verifica automatica e ottenere un interprete con un perimetro di compatibilità dichiarato e misurabile.

Riferimento principale: il sorgente presente in `c_awk`, compilato e identificato per versione e revisione. L'AWK installato nel sistema è un riferimento secondario. Non promettiamo conformità completa sulla sola base di un numero di test verdi.

Proposte da approvare insieme al piano:

- Conservare Rust come implementazione del runtime, senza delegare l'esecuzione al binario C.
- Dare priorità al comportamento dell'originale per il nucleo comune. Conservare le estensioni già presenti, come RT e BEGINFILE/ENDFILE, in un gruppo di test distinto con contratto esplicito.
- Conservare la gestione binaria di byte alti e NUL: eventuali differenze deliberate dal C vengono documentate e verificate, non cancellate per aumentare il conteggio MATCH.
- Validare inizialmente il profilo byte-oriented in `LC_ALL=C`; supporto completo delle locale e comportamento Unicode restano fuori da questa tranche.
- Implementare realmente CSV e safe mode, poiché la CLI li espone già. Safe mode significa le restrizioni concrete previste dal C, non una sandbox generale.
- Ottimizzazioni prestazionali, nuove estensioni gawk e supporto a ulteriori piattaforme vengono dopo il consolidamento funzionale.

L'approvazione del piano autorizzerà l'esecuzione delle fasi in sequenza. Le normali scelte interne non richiederanno nuove approvazioni. Se la verifica delle regex dimostra necessario cambiare questi vincoli o ridurre il perimetro, presenterò il cambiamento e le evidenze prima di procedere su quella parte.

## Fase 1 — Rendere attendibili i test

Interventi:

- Unificare gli helper condivisi tra runner XML, test d'integrazione e diffrun.
- Directory temporanea distinta per ogni caso e per ciascun interprete; fixture di input replicate, nessun output condiviso.
- Timeout e chiusura dei processi avviati dal test, incluse le pipe, per evitare blocchi e processi residui.
- Confrontare stdout come byte, status e stderr. Normalizzare soltanto elementi dichiarati, come prefissi dei percorsi diagnostici; eliminare trim e conversioni lossy indiscriminate.
- Rendere configurabili binario di riferimento e manifest; stampare il percorso effettivamente usato. Segnalare come verifica non eseguita o errore l'assenza dell'oracolo.
- Rendere le divergenze attese specifiche: output/status accettati per rawk e motivo della differenza. Una qualsiasi differenza nuova non deve diventare automaticamente EXPECTED.
- Correggere `checks.sh`: eseguire la suite completa e propagare tutti i fallimenti.
- Importare le 27 sonde e i due casi safe/CSV in un inventario tracciabile. Attivare le regressioni con i rispettivi fix; i difetti ancora aperti rimangono visibili nel rapporto, senza falsi conteggi di successo.

Accettazione: `cargo test` passa nella modalità parallela standard; un test del gate dimostra exit nonzero quando un controllo fallisce; timeout, errori, byte e isolamento sono coperti. Ripetere una volta la suite parallela per verificare la correzione della collisione nota.

## Fase 2 — CLI, safe mode e confini degli errori

Interventi:

- Risolvere la distinzione fra programma inline, uno o più `-f` e file di input.
- Gestire `-v`, `--`, stdin esplicito e assegnazioni `nome=valore` negli argomenti rispettandone l'ordine di applicazione.
- Ricavare dal C le restrizioni di safe mode e applicarle a comandi, pipe, redirezioni e ambiente dove previsto, senza affidarsi al solo flag della CLI.
- Introdurre errori del runtime espliciti e validazione degli argomenti delle builtin; un errore di programma deve produrre una diagnosi, non un panic Rust.

Accettazione: i casi `-f programma file`, più file, assegnazioni intercalate e arità errata hanno comportamento verificato. Safe mode rifiuta le operazioni vietate e consente le operazioni lecite. Il programma di prova non viene eseguito quando vietato.

## Fase 3 — Controllo di flusso uniforme

Interventi:

- Propagare errori e trasferimenti di controllo anche attraverso la valutazione delle espressioni e delle chiamate, distinguendoli dai normali valori.
- Eliminare progressivamente `exit_pending` e `nextfile_pending` a favore della propagazione esplicita.
- Correggere la valutazione corta di `&&` e `||`.
- Gestire `return`, break/continue, next/nextfile ed exit nei cicli annidati e nelle funzioni; diagnosticare i contesti vietati dal profilo scelto.
- Centralizzare la terminazione: END nei casi previsti, conservazione/sovrascrittura del codice exit secondo il riferimento, chiusura degli stream e attesa dei processi figli. Distinguere exit volontario da errore fatale.

Accettazione: le regressioni su exit/END, short circuit e return nel while passano; nessuna istruzione successiva a un trasferimento viene eseguita accidentalmente, neppure nella stessa espressione. Test dedicati per exit in BEGIN, azione, funzione ed END.

## Fase 4 — Input, record e getline

Interventi:

- Introdurre un gestore dell'input principale condiviso fra ciclo dei record e getline, con stato di file corrente, stdin e argomenti ancora da elaborare.
- Separare acquisizione del record, assegnazione a `$0`, splitting e ricostruzione dopo modifiche ai campi/NF.
- Aggiornare NR/FNR soltanto nelle operazioni di lettura appropriate; verificare separatamente getline principale e rediretto, con e senza variabile.
- Applicare cambiamenti di RS ai record successivi; gestire EOF, file successivo, nextfile, FILENAME e hook delle estensioni conservate.
- Introdurre lettura incrementale per delimitatore singolo e paragrafi. Preparare l'interfaccia per RS regex, completata con la fase 6, evitando letture dell'intero file come comportamento ordinario.

Accettazione: nessun timeout nelle sonde getline, lettura corretta su stdin e più file; `$0=$0` e sub/gsub non alterano NR/FNR; errori di apertura restituiti correttamente; RS modificato durante lo script ha effetto.

## Fase 5 — Valori, grammatica e funzioni

Tre sottopassi separati, ognuno verificabile:

1. **Valori:** coercizione numerica con prefisso, distinzione String/StrNum, confronti e uso coerente di CONVFMT/OFMT.
2. **Grammatica e destinazioni delle assegnazioni:** escape di stringhe e regex, identificatori con underscore, numeri `.5`, precedenza della potenza, assegnazioni come espressioni, pattern a intervallo. Unificare lettura/scrittura di variabili, campi e elementi array per assegnazioni, incrementi e decrementi; valutare gli indici con effetti collaterali una sola volta.
3. **Funzioni:** scalari per valore, array per riferimento, scope dei parametri, parametri omessi, ricorsione e diagnosi delle incompatibilità scalar/array.

Accettazione: casi dell'audit corretti più test di composizione, per esempio incremento di un elemento con indice mutabile, funzione che modifica un array ricevuto da un'altra funzione, return da cicli annidati e pattern a intervallo su più file.

## Fase 6 — Regex, separatori, builtin e CSV

Questa è la fase con maggiore incertezza tecnica. Il primo sottopasso è una verifica della soluzione regex, prima di estenderla a tutti i chiamanti.

- Definire un'interfaccia comune per compilazione e ricerca, con errori espliciti.
- Verificare su un corpus mirato semantica leftmost-longest, alternanze, classi, ancore, match vuoti, byte alti e NUL. Valutare una soluzione Rust sulla base di queste prove; non assumere che sostituire una crate garantisca compatibilità.
- Usare la stessa semantica in match, operatori regex, FS, RS e sub/gsub. Cache con limiti e politica documentata.
- Correggere split: campi vuoti, separatore spazio, cancellazione dell'array precedente e differenze previste fra separatore letterale e regex.
- Correggere numero e avanzamento delle sostituzioni sub/gsub, compresi match vuoti e sostituzioni che non cambiano il testo.
- Completare RS regex incrementale, con test sulle corrispondenze che attraversano i confini dei buffer. Il buffering di un singolo record lungo può comunque crescere: non promettere memoria costante per input arbitrario.
- Implementare CSV secondo il comportamento verificato del C: campi quotati, virgolette raddoppiate, separatori e newline nei campi, terminazioni di riga e interazione con FS/RS.

Accettazione: nessun fallback silenzioso per regex invalide; RLENGTH corretto su `a|ab`; regressioni split/gsub/FS/CSV risolte; stessi risultati variando la dimensione dei buffer.

## Fase 7 — Convalida estesa e documentazione

Interventi:

- Inventariare `c_awk/testdir` e `bugs-fixed`; adattare i test applicabili al nuovo harness. Ogni esclusione deve avere un motivo, distinguendo estensioni, dipendenze dall'ambiente e lacune ancora aperte.
- Aggiungere generazione di piccoli programmi validi e limitati: espressioni con effetti collaterali, funzioni, array e sequenze di record. Seed riproducibili e minimizzazione dei controesempi.
- Verificare debug e release, stdin/file/pipe, errori e casi binari.
- Aggiornare README, diario e matrice di compatibilità. Preservare la storia degli audit precedenti; rimuovere soltanto artefatti confermati inutilizzati.
- Ripetere benchmark rappresentativi per lettura, campi, regex e aggregazioni, registrando tempi e memoria. Eventuali ottimizzazioni diventano un intervento successivo motivato dai dati.

Accettazione finale:

- Build release, fmt, Clippy, suite completa e gate tutti verdi.
- Tutte le 29 verifiche dell'audit hanno esito risolto o una differenza deliberata esplicitamente concordata; nessun difetto viene chiuso mediante skip generico.
- Nessuna divergenza inattesa nel corpus incluso; nessun panic o timeout nei test previsti.
- Rapporto finale con copertura funzionale, esclusioni, differenze deliberate e rischi residui. Il numero dei test non viene presentato come percentuale di conformità AWK.

## Modalità di avanzamento

Sequenza: **1 → 2 → 3 → 4 → 5 → 6 → 7**. All'inizio della fase 1 fissiamo anche la matrice dei comportamenti; la scelta del motore regex resta un sottopasso tecnico esplicito della fase 6.

Per ogni sottopasso: caso riproducibile che fallisce → modifica circoscritta → verifica pertinente → suite completa prima della chiusura della fase → breve aggiornamento con risultati e problemi rimasti. Nessuna modifica degli expected soltanto per rendere verde la suite: ogni aggiornamento deve essere giustificato dall'oracolo o da una differenza deliberata del profilo.

Lavorare nel repository `rawk`, mantenendo `c_awk` come riferimento. Separare i cambiamenti per problema/fase, evitando un'unica riscrittura indistinta. Nessun push o pubblicazione previsto dal piano.

Non fisso una durata attendibile prima di risolvere l'incertezza sulle regex e misurare l'ampiezza dei casi storici applicabili. Le fasi 3, 4 e 6 sono i blocchi più impegnativi; la fase 1 è il prerequisito per valutarli senza falsi positivi.


## Aggiornamento — consolidamento del 19 settembre 2026

Eseguiti i cinque interventi aggiuntivi approvati: contratti delle sei differenze
residue, printf dinamico/underflow, inventario completo dei 33 driver, CI con
oracolo fissato, profiling e due ottimizzazioni misurate. Sono attività di
consolidamento della Fase 7, non un ritorno alla Fase 1.

Il corpus piccolo ha 219 confronti integrali e sei contratti verificati. Dei driver,
20 passano e sono nel gate; 13 mantengono motivazioni individuali e risultati
consultabili. La Fase 7 resta aperta per queste lacune e per la convalida delle
piattaforme oltre il profilo Darwin ARM64. Dettagli e misure:
`rawk/diary/2026-09-19-consolidation.md`.


## Aggiornamento finale — chiusura della Fase 7, 19 settembre 2026

Completati i cinque blocchi approvati per la chiusura: scomposizione dei driver,
regex/separatori, grammatica/builtin/sorgenti binari, CLI/diagnostiche e convalida
finale. Le lacune funzionali individuate nei driver del profilo sono corrette;
le differenze deliberate hanno contratti individuali esatti e prove negative.

- 114 test superati in debug e release; fmt, Clippy, build e gate verdi.
- 27 driver completi nel gate, 275 sottocasi con contratto verificato.
- XML: 97 MATCH, 12 EXPECTED, nessuna divergenza inattesa e nessuno skip.
- Audit 29/29, bugs-fixed 31/31; tutti i 225 programmi del corpus piccolo verificati.
- Gate superato anche in una copia pulita senza target, con C ricompilato.
- Benchmark finale: mediane +0,4%–3,9% rispetto al precedente binario ottimizzato;
  memoria stabile, stdout identico al C. Il costo è riportato, non nascosto.

**Fase 7 chiusa per il profilo concordato.** Unicode/locale completi e piattaforme
ulteriori rimangono lavori successivi, non criteri pendenti di questa tranche.
CI remota non eseguita; nessun push. Rapporto, matrice e log:
[chiusura della Fase 7](../diary/2026-09-19-phase7-closure.md).


## Attività successiva — Unicode e locale, 19 settembre 2026

Su richiesta dell'utente è stato implementato il profilo UTF-8 pilotato da
LC_CTYPE, preservando il gate `LC_ALL=C`. Sono coperti stringhe, indici, regex,
separatori, sostituzioni, formattazione, conversioni case e input ai confini dei
buffer; i byte invalidi seguono i comportamenti verificati del C e i NUL restano
un'estensione esplicita.

Convalida: 124 test in debug e release, gate completo verde, tutti i 300 casi
originali di T.utf/T.utfre passanti in en_US.UTF-8. Rapporti storici della Fase 7
preservati. Misure e limiti nel [rapporto Unicode e locale](../diary/2026-09-19-unicode-locale.md).

Restano la convalida su altre piattaforme, le codifiche legacy non UTF-8 e le
ottimizzazioni motivate dalle misure. Workflow CI aggiornato; nessun push e
nessuna esecuzione CI remota.


## Portabilità — infrastruttura e controlli, 19 settembre 2026

Predisposta CI nativa Linux glibc x86-64/ARM64 con oracolo C fissato e locale
UTF-8 esplicitamente generate. Aggiunto gate riproducibile di portabilità e test
che impedisce falsi positivi quando le locale non sono disponibili. Superati i
controlli Rust di tutti i target per Linux x86-64/ARM64 e Darwin x86-64.

L'esecuzione Linux rimane da svolgere: nessun runtime Linux locale disponibile,
nessun push o lancio CI remoto. Non si dichiara ancora compatibilità Linux.
Dettagli nel [rapporto di portabilità](../diary/2026-09-19-portability.md).


## Attività locali — prestazioni, 20 settembre 2026

Su indicazione dell'utente, sospesa la convalida nativa Linux e proseguito con
le attività eseguibili su Darwin. Ottimizzata la ricerca booleana UTF-8 e reso
riproducibile il benchmark a cinque carichi Unicode. Gate completo e 126 test
in debug/release verdi. Tempi dei tre carichi booleani ridotti del 12–41%;
variazioni del profilo byte +1–3%, documentate senza generalizzare il risultato.

Una sonda conferma difformità ancora aperte nelle conversioni e classi delle
locale ISO-8859-1/9 e nel trattamento di byte invalidi Shift-JIS. Il prossimo
intervento funzionale riguarda queste codifiche, con profili delimitati e
confronti esatti; Linux rimane pendente. [Rapporto e risultati](../diary/2026-09-20-regex-performance.md).


## Locale legacy — ISO-8859-1/9, 20 settembre 2026

Corrette le conversioni upper/lower e le classi POSIX per le due codifiche a
byte singolo, usando l'oggetto locale immutabile della libc. Tutti i 255 byte
non nulli sono confrontati con il C per conversioni e 12 classi; verificati
anche operazioni a byte, NUL e precedenza delle variabili di locale.

Gate completo e 132 test debug/release passanti; XML invariato e 300 casi
originali UTF-8 ancora verdi. CI predisposta per generare le due locale Linux;
controlli Rust dei target x86-64/ARM64 superati, esecuzione nativa sospesa.
Shift-JIS e le altre codifiche rimangono aperte, senza estendere implicitamente
il supporto a tutte le locale non UTF-8. [Rapporto](../diary/2026-09-20-legacy-locales.md).


## Modalità operative — 20 settembre 2026

Adottate regole persistenti in [AGENTS.md](../AGENTS.md), punto di ingresso
[STATO_PROGETTO.md](../STATO_PROGETTO.md), una conversazione per obiettivo e
[modello di consegna](../docs/CONSEGNA_TASK.md). Il prossimo obiettivo ha una
[scheda Shift-JIS](../docs/tasks/SHIFT_JIS.md). La home mantiene struttura e
attribuzioni originali; riscritture da concordare. Il piano canonico è questa
copia versionata nel repository, non quella esterna.

## Shift-JIS — 20 settembre 2026

Implementate conversioni libc e unità strutturali BWK per `ja_JP.SJIS`, con
regressioni su dati validi, invalidi, troncati e NUL. La sonda completa delle
coppie valide della libc Darwin verifica 11.280 coppie per upper/lower.
Preservati i contratti di NUL e stampa atomica, senza modificare gli expected
XML o la home. [Rapporto e risultati](../diary/2026-09-20-shift-jis.md).

Rimane aperto il decoder del percorso streaming `fnematch` per RS regex:
sequenze strutturali di tre/quattro byte possono essere interpretate dal C
in funzione del riempimento a gruppi di due byte. Test differenziale presente
ma ignorato esplicitamente; prossimo passo modellare tale comportamento
prima di estendere il perimetro dichiarato. Linux nativo ancora sospeso.
139 test debug/release passati con 1 ignore esplicito; XML 97 MATCH,
12 EXPECTED, 0 UNEXPECTED. Fmt/Clippy e controlli Rust Linux glibc dei due
target passati; igiene AppleDouble ripristinata dopo il gate.

Su istruzione dell’utente, `AGENTS.md` richiede ora commit e push su GitHub
al termine di ogni attività, dopo le verifiche pertinenti e con verifica
del commit remoto, salvo diversa istruzione esplicita.


## Chiusura del residuo RS Shift-JIS — 20 settembre 2026

Ricostruito `fnematch` con visibilità incrementale di due byte e riavvio dei
candidati sul buffer già letto. Percorso separato dalle regex in memoria;
regressione RS riattivata, senza cambiare gli expected. Verificati anche
ancore, alternative, match più lungo, EOF, `getline`, cambi di RS, buffer e
letture corte. NUL e byte FF restano preservati. La sonda estesa distingue
2.440 confronti esatti da 408 errori C `ungetc(0xff)`, verificati separatamente
senza introdurre l'errore nel runtime Rust. [Consegna](../diary/2026-09-20-shift-jis-rs.md).
Linux nativo e altre piattaforme restano attività distinte e sospese.

Verifica finale RS: 144 test debug e release, nessun ignorato; XML invariato,
fmt/Clippy e controlli Rust Linux dei due target passati. Solo l'igiene dei
metadati AppleDouble ha richiesto pulizia e riverifica; Linux nativo sospeso.


## CI Linux nativa — 20 settembre 2026

Superata la sospensione storica per assenza di runtime Linux locale grazie ai
runner GitHub nativi x86-64/ARM64. Runtime `a118616`: tutti i job della
[CI](https://github.com/francescotinti/rawk/actions/runs/35507960469) verdi.
Corretti Clippy su `wchar_t` ARM64, precisione dinamica negativa glibc,
unsigned negativi x86-64 e padding di `%s`. Conservati Darwin, NUL, byte alti,
stampa atomica, home, sorgenti C ed expected esistenti.

Verificati 75 test debug + 75 release per runner Linux; Darwin locale 148 debug
+ 148 release, nessun ignorato. XML 97 MATCH / 12 EXPECTED / 0 UNEXPECTED /
0 SKIPPED. Il gate macOS remoto comprende anche gli audit originali e UTF-8.
Metadati AppleDouble locali rimossi e controllo di igiene riverificato;
i fallimenti intermedi sono conservati nelle evidenze.

Il gate Linux mantiene le suite selezionate: non include Shift-JIS e non
prepara `ja_JP.SJIS`; l'inventario completo dei driver è diagnostico, senza
`--check`. Restano da delimitare convalida Shift-JIS Linux, altre codifiche,
macOS Intel nativo e ulteriori domini numerici. Nessuna certificazione generale.
[Consegna ed evidenze](../diary/2026-09-20-linux-ci.md).


## Shift-JIS Linux nativo — 20 settembre 2026

Attivata la suite Shift-JIS nei runner glibc 2.39 x86-64/ARM64 con locale
`ja_JP.SJIS` generata e sonda libc obbligatoria. Entrambi superano 87 test debug
+ 87 release, inclusi 12 Shift-JIS, senza ignorati. Confrontate tutte le 6.879
coppie decodificabili native: upper riesce per tutte, lower per 6.878; il caso
`81 f0` verifica l'errore di ricodifica condiviso da C e Rust. La sonda iniziale
si interrompeva su questo errore perché assumeva che ogni mapping fosse
ricodificabile; evidenze conservate e controllo corretto, senza scartare casi.

Nessuna modifica al runtime: preservati contratti binari, decoder BWK,
conversioni libc, RS e correzioni precedenti. Altre codifiche e macOS Intel
rimangono attività separate; inventario driver Linux ancora diagnostico.
CI finale `3b9facf` tutta verde, compreso macOS: 149 test debug, XML invariato,
audit originali/UTF-8 passati. [Consegna ed evidenze](../diary/2026-09-20-shift-jis-linux.md).


## Audit dei driver Linux nativi — 20 settembre 2026

Esaminati tutti i 33 driver e i 275 sottocasi sui runner glibc x86-64/ARM64.
Corretti gli alias stdout/stderr (anche ciclo close/fflush e ordine senza
newline) e gli errori libm di log/exp/sqrt su Linux. T.beebe passa ora su
entrambe le architetture; i fallimenti storici restanti sono classificati
come diagnostiche, contratti deliberati o problemi del profilo/driver.

Il workflow controlla in debug e release 27 driver ammessi e 275 sottocasi
(171 esatti, 101 diagnostiche fissate, tre differenze deliberate), oltre a
20 sonde degli stream. Due nuovi contratti Linux accettano soltanto il
contesto diagnostico differente dopo la correzione matematica; le prove
negative respingono mutazioni dei risultati, input e inventario.
L'inventario grezzo dei 33 driver resta disponibile senza nasconderne i
fallimenti. Nessuna modifica all'oracolo, ai suoi expected o alla home.

I gate nativi Linux passano 89 test debug e 89 release per runner.
Darwin locale: 151 debug e 151 release; macOS CI: 151 debug, XML invariato
e audit verdi. CI `b1adc82` tutta verde; la suite release macOS è locale.
Dettagli Git, evidenze e limiti nella
[consegna del task](../diary/2026-09-20-linux-drivers.md).
Altre codifiche, macOS Intel, ottimizzazioni e operatori/cast numerici fuori
dal perimetro verificato rimangono separati.


## macOS Intel nativo — completato il 20 settembre 2026

Convalida reale sul runner `macos-15-intel`, macOS 15.7.9 x86-64, senza
Rosetta. Corretto un bug di printf/sprintf unsigned negativo nel solo profilo
Intel, con controesempio nativo, regressioni e confini verificati. Nessun
contratto o expected originale modificato; preservati ARM64 e Linux.

CI `11d7313` tutta verde: 151 debug + 151 release su ciascun runner macOS,
XML 97 MATCH / 12 EXPECTED / 0 UNEXPECTED / 0 SKIPPED; per profilo 27 driver,
275 contratti e 20 sonde stream. Audit UTF-8 verde e 15.240 coppie Shift-JIS
verificate nativamente su entrambi i runner macOS 15.7.9, senza ereditare
il conteggio del diverso profilo locale. Linux: 89 debug + 89 release per
runner, gate integri. Verifica locale ARM64: 151 debug + 151 release.

Dettagli dell'harness, ambiente, tentativi iniziali e pubblicazione nella
[consegna Intel](../diary/2026-09-20-macos-intel.md).
Restano separati altre codifiche, nomi di file non UTF-8, ulteriori versioni
macOS, ottimizzazioni e domini numerici fuori dal perimetro verificato.

## Priorità prestazioni — 20 settembre 2026

Sequenza autorizzata; questa tranche esegue soltanto la prima priorità.

1. **Lettura, campi, valori e aggregazione**: baseline riproducibile su `aa0301a`,
   profiling locale e primo intervento circoscritto sul costo misurato. Confronto
   intercalato prima/dopo/C con output controllato, distribuzioni e RSS; gate
   locali e CI nativa sulle quattro piattaforme prima della consegna runtime.
   Nessuna promessa di parità col C; conservare anche eventuali risultati negativi.
2. **Regex e sostituzioni UTF-8**: match/gsub, ricerche brevi e senza match;
   preservare il vantaggio del percorso booleano su righe lunghe.
3. **I/O e memoria su input grandi**: copie, allocazioni, buffering e picco RSS,
   preservando pipe, getline, RS e stream condivisi.
4. **Misure multipiattaforma e regressioni**: Linux e macOS Intel/ARM64,
   corpus più rappresentativo e monitoraggio riproducibile. La correttezza
   multipiattaforma resta obbligatoria già nella prima priorità.

Le misure storiche non sono una baseline dell'HEAD corrente. Il punteggio
indicativo 35/100 non è una metrica di accettazione. Commit e push seguono
l'autorizzazione permanente in AGENTS.md, che supera le note storiche del piano.

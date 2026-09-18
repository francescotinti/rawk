# Correzione dei cinque casi funzionali residui

19 settembre 2026. Seguito dell'intervento approvato dopo l'estensione del corpus. Riferimento C: revisione `5739fd7`; profilo a byte, `LC_ALL=C`, macOS. I rapporti precedenti sono conservati come evidenza delle fasi precedenti.

## Esito

I cinque casi `t.null0`, `t.sub0`, `t.gsub4`, `t.split3` e `t.arith` ora corrispondono integralmente al C. Sono aggiunti al manifest fisso della suite permanente: **219/225** programmi piccoli, contro i precedenti 214/225.

Le sei differenze rimaste nel corpus sono:

- `p.43`, `t.in2`, `t.intest2`: ordine di iterazione degli array;
- `p.48b`, `t.randk`: sequenza RNG differente;
- `t.printf2`: conservazione dei byte NUL nel profilo Rust.

Nessuna nuova divergenza è stata trasformata in EXPECTED o rimossa dal confronto. Questi sei casi continuano a essere eseguiti dall'audit completo; `corpus_audit.py --check` continua a restituire errore finché ci sono differenze esatte.

## Correzioni

### Campi vuoti

Un campo oltre NF restituisce una stringa vuota, mentre una variabile mai inizializzata mantiene il valore duale zero/stringa vuota. Questo evita che `$5 == 0` risulti vero quando il record contiene meno di cinque campi. Copia del campo in una variabile, estensione dei campi e riduzione di NF sono verificate contro il C.

### Sostituzioni ed escape

Distinti i due passaggi: decodifica del letterale nel sorgente e interpretazione della stringa di sostituzione. `\&` nel sorgente viene decodificato come nel C; una barra effettivamente presente prima di `&` nella stringa a runtime produce invece un ampersand letterale.

La gestione delle combinazioni di barre e ampersand segue `backsub` del C, inclusa la distinzione tra modalità predefinita e `POSIXLY_CORRECT`. I test coprono da zero a otto barre, stringhe letterali e sostituzioni lette dall'input, sia per sub sia per gsub. L'estensione preesistente che conserva altri escape sconosciuti nei letterali, verificata dal caso XML 0066, rimane esplicita.

### Classi regex

Le classi vengono normalizzate prima della compilazione con il motore Rust. Gli intervalli sono espansi nell'ordine del C; un intervallo discendente elimina la propria componente invece di causare un errore. Se la classe positiva diventa vuota, il C la interpreta come un match di lunghezza zero: anche questo caso è riprodotto.

Coperti intervalli concatenati, trattini letterali, negazione, parentesi quadra iniziale, escape numerici e classi POSIX ASCII. La normalizzazione mantiene il profilo binario: NUL e byte alti restano rappresentabili, compresi gli intervalli che partono da zero. Le differenze deliberate rispetto alle stringhe terminate da NUL del C non vengono eliminate. Questo non introduce supporto completo delle locale o delle classi Unicode.

### Formattazione numerica

Aggiunto `src/number_format.rs`, condiviso tra printf/sprintf e conversioni OFMT/CONVFMT, per e/E/f/g/G. La conversione decimale di Rust arrotonda il valore binario effettivo; la scelta della notazione g/G considera l'esponente dopo l'arrotondamento, evitando errori quando si attraversa una potenza di dieci.

L'esempio che falliva, 10409/8 = 1301.125 con sei cifre significative, produce ora **1301.12**, come il C, anziché 1301.13. Verificati precisione, segno, larghezza, padding, forma alternativa, notazione esponenziale e distinzione tra zero numerico normalizzato e conversione della stringa "-0". Il formatter esadecimale resta separato.

La verifica delle differenze residue ha inoltre scoperto che `%c` trattava i campi numerici StrNum come stringhe: 17379 produceva "1" invece del byte 0xE3. Corretto il tipo di conversione e il padding, con prove su stringhe esplicite, valori negativi e byte alti. La precedente classificazione di t.printf2 come differenza solo NUL era quindi incompleta; dopo questa correzione è stata verificata l'uguaglianza esatta eliminando dal solo output Rust i byte NUL. Questa eliminazione è una verifica diagnostica, non una normalizzazione applicata al corpus.

## Verifica

Le nuove prove mirate hanno inizialmente riprodotto i quattro gruppi di errore, poi sono passate dopo le correzioni. La suite ora contiene **92 test Rust**, inclusi sette nuovi test contenitori: confronti puntuali, conservazione dei byte e 64 casi razionali/formati generati con seed 20260920 e shrinking. Il numero dei test non va sommato ai casi eseguiti al loro interno.

| Verifica | Esito |
|---|---|
| Build release, fmt, Clippy e gate completo | OK |
| Suite Rust | 92 test passanti |
| XML | 96 MATCH, 13 EXPECTED-DIVERGE, zero inattesi e zero saltati |
| Audit iniziale | 29/29 stdout/status corrispondenti |
| bugs-fixed | 31/31 stdout/status corrispondenti |
| p.* e t.* | 219/225 confronti integrali |
| Driver shell adattati | 3/3 passanti con entrambi gli interpreti |

Evidenze in `verification-functional-2026-09-19/` e nei JSON aggiornati di diary. I 219 casi permanenti confrontano stream, status e file prodotti. Nessun sorgente del riferimento C modificato, nessun push.

## Lavoro ancora aperto

Le sei differenze del corpus richiedono criteri specifici o decisioni di compatibilità, non correzioni indiscriminate agli expected. Fuori da questo corpus restano il comportamento osservato dei letterali ai limiti di underflow, printf con larghezza/precisione dinamiche `*`, gli altri driver T.*, locale/Unicode complete e l'intera sintassi regex originale. Questa tranche non dichiara risolti tali punti e non contiene ottimizzazioni prestazionali o CI.

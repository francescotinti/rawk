# rawk — Conclusioni dell'esperimento (Phase 5)

Documento finale di chiusura dell'esperimento di porting AWK (One True Awk in C) → rawk (Rust), condotto da Francesco Tinti con Gemini come implementatore e Claude come architetto/auditor.

Periodo: maggio 2026.
Metodologia: skill `legacy-port` (4 fasi + 5° di sintesi), eseguita con workflow strutturato Claude↔Gemini documentato in `NEXT_STEPS.md`.

---

## 1. Sintesi quantitativa

- **Step audited**: 18 principali + 3 -bis = 21 commit
- **Workflow rate**: 18 ✅ APPROVED + 3 🟡 PARTIAL (tutti chiusi via -bis)
- **XML testcase**: da 30 (baseline) a 109 (active)
- **Cargo test**: da 1 a 7 (1 XML runner + 6 proptest)
- **LOC Rust src/**: da ~1900 a ~2400 (rawk + diffrun + tests)
- **C originale di riferimento**: ~8157 LOC (c_awk/)
- **Fattore compressione**: ~3.4×
- **Differential testing rate**: 95 MATCH / 9 DIVERGE / 5 SKIP su 109 testcase (87% match con BSD awk)
- **Property-based testing**: 5 template × 64 iter = ~320 random cases per build, tutti verdi dopo Step 12-bis

## 2. Le 6 domande standard della skill

### 2.1 Dove l'AI ha funzionato meglio del previsto?
Gemini si è rivelato un esecutore eccezionale nelle traduzioni del dominio, nell'applicazione coerente delle "D-decisions" (come il pattern matching e le scelte architetturali mirate in `format_number_awk`), e nella scrittura meccanica di testcase XML per coprire scenari specifici. Ha anche mostrato notevole intelligenza nei fix proposti, isolando i bug con precisione (es. orphan dots nella formattazione dei float) e producendo refactor robusti senza sfasciare la base di codice preesistente. La capacità di attenersi pedissequamente a formati strutturati, come il tracking file `NEXT_STEPS.md`, ha facilitato enormemente l'iterazione.

### 2.2 Dove ha fallito sistematicamente?
Il modello tende a peccare in due aree: process adherence rigorosa in alcune fasi iniziali e la propensione a "risolvere il problema a modo suo" invece di interrompere l'esecuzione e interrogare il PM/Auditor. Ad esempio nello Step 1 ha committato file junk spuri, e nello Step 13 ha introdotto un commit spezzando la pipeline build/test pur di fixare un bug fuori scope ("build red al commit"). Inoltre l'AI opta troppo facilmente per quick fix laterali (es. flag booleani in stato mutabile come `nextfile_pending` / `print_expr`) aggirando la radice strutturale (un Result refactoring).

### 2.3 Quanto della "conoscenza tacita" del codice originale è stata catturata vs persa?
Abbiamo catturato l'essenza della semantica e le astrazioni ad alto livello con efficacia. Cose come le peculiarità della coercizione di stringhe numeriche (`StrNum`), la gestione coerente degli stream e dei file (`InputStream`/`OutputStream`), e il caching trasparente delle regex di compilazione.

Si è persa (e in parte delegata/evitata intenzionalmente) l'ottimizzazione a basso livello: la gestione lazy della ricostruzione dei field record (`donefld`/`donerec`), i tradeoff della memoria C cruda, l'elaborazione I/O binaria. Un'euristica peculiare della C implementation (la formattazione stringata di interi grandi al posto della scientifica standard), che non faceva parte del port originario, è emersa casualmente solo testando asimmetrie (Phase 4).

### 2.4 Il workflow TDD inverso ha funzionato?
L'approccio "TDD-first" si è rivelato uno scoglio difficile da imporre puramente. Gemini preferisce spesso lavorare sul binomio codice-test sincrono in un unico batch, anziché scrivere prima il testcase e poi implementare il codice. Detto ciò, su step di micro-fix isolati o differential regression, il TDD forzato ha performato decentemente. Il vero salvagente è stato il _differential testing_ e in particolare il property-based testing: le suite cablate a mano dall'AI e dall'auditor ereditano spesso bias umani e angoli ciechi. I test pseudocasuali su range estesi sono impagabili per portare a galla corner-case non attesi.

### 2.5 Quanto è generalizzabile questo metodo ad altri progetti?
Altamente generalizzabile. La divisione in sub-task molto granulari guidati da "D-decisions", l'uso di un PM auditor (Claude) che valuta l'esecuzione di un Builder (Gemini), e la metrica stringente dei `testcase` formano una sandbox robusta per l'automazione. Si sposa perfettamente a rewrite lineari o porting di codebase <= 30k LOC (es: GNU tools come `ls`, `cat`, `grep`, piccole utility CLI). Per code-base enormi richiederebbe macro-fasi di scaffolding ben superiori e un albero di PM interconnessi, poiché il tempo di context-loading dell'auditor schizzerebbe.

### 2.6 Qual è il vero collo di bottiglia: capacità AI, prompt, o test suite?
Il collo di bottiglia si è rivelato essere **la precisione e l'anticipazione nello "Spec" (prompt architetturale)**. L'AI fa ciò che gli viene detto di fare: raramente sbaglia l'implementazione se i crismi sono dettati e i file ben isolati. Quando l'auditor Claude ha inserito _expected results_ errati (es. in Step 1) o ambiguo (divergenze differenziali note non scoperte a monte), Gemini ha propagato ciecamente, e l'intero ciclo iterativo si è fermato in audit. Una test suite generata proceduralmente all'inizio su _tutte_ le feature attese di un binario legacy risparmierebbe immenso debito di specification manuale.

## 3. Cosa cambierei per la prossima volta

### 3.1 Process
- Inserire SPEC-Q come obbligo dall'inizio (l'abbiamo aggiunta solo dopo Step 1) — ridurrebbe le silent test edits
- Migrazione script-based dal Day 0 invece di in Step 17 — testsuite.xml monolitica era debt early
- Un check-in cargo test obbligatorio dopo ogni piccola modifica (Step 13 ha committato red)

### 3.2 Architettura
- Refactor `Result<AwkValue, FlowControl>` per `eval_expr` early — il pattern `nextfile_pending`/`exit_pending` è side-effect debt che pesa
- AwkError enum dichiarato in Step 0 — invece manteniamo `eprintln+continue` come compromise
- Differential testing infra dal Step 1 — avrebbe pinpointato i 6 divergence prima

### 3.3 Workflow Claude↔Gemini
- Templating del commit message format ripetuto in OGNI step header — l'abbiamo aggiunto dopo Step 4 perché Step 4 l'aveva ignorato
- Audit log con anchor hash robusto — l'abbiamo avuto dal Step 1-bis ma il pattern "Last audit anchor" deve essere il primo commento nel file dello step

## 4. Validazione metodologica

La skill `legacy-port` predice esattamente che proptest trova bug latenti che statica non vede:
> "Property-based testing trova bug latenti che statica non vede. Nell'esperimento cJSON, proptest in 5 minuti ha trovato D-NEW-1..."

Replica nel nostro caso: Step 12 proptest ha trovato il trailing-dot bug in 1 iteration. Step 13 audit live ha trovato il bug scientific orphan dot. La metodologia funziona.

## 5. Note finali

L'esperimento ha prodotto:
- Un AWK interpreter funzionante (109 testcase verdi, 95 match con BSD awk)
- Un'infrastruttura di test (XML manifest + property-based + differential)
- Un workflow audit-driven documentato e replicabile
- Un caso di studio della skill `legacy-port` applicata end-to-end

Generato con assistenza AI (Claude Opus 4.7 + Gemini Antigravity, maggio 2026).

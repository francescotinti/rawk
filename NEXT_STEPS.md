# NEXT_STEPS — Spec operative per Gemini

Documento di handoff scritto dopo audit Claude Code dei commit `ed791b9`, `52c1645`, `453987f`.

## Stato attuale (verificato)

✅ **Già fatto**:
- AST tipato (`Expr::NumberLiteral`, `Expr::UnaryMinus`, `Expr::UnaryPlus`)
- `AwkValue::StrNum(String, f64)` con dualità POSIX, applicato a campi/ARGV/ENVIRON/getline/split
- Magic vars (NF/NR/FS) deduplicate (singola fonte di verità)
- Operatori `^` `**` `%` con precedenze giuste
- Cache regex globale su `EvalContext`
- 31 testcase XML verdi

🎯 **Da fare ora**: i due Step sotto, in ordine. **Un commit per Step**, non mischiare.

🚫 **Fuori scope**: gawk extensions (`gensub`, `mktime`), ANSI C99 grouping flag, `nextfile`, `system()`, `close()`, `fflush()`. Saranno step successivi.

---

# Workflow Claude ↔ Gemini

## Ruoli

| Ruolo | Responsabilità |
|---|---|
| **Claude** | Architetto + auditor. Scrive spec di ogni Step, audita commit, decide next step. NON scrive codice. |
| **Gemini** | Implementatore. Legge spec, scrive codice, fa commit. NON decide architettura né riapre decisioni di design. |
| **Francesco** | Relay minimale + decisore finale su trade-off ambigui. |

## Le 5 fasi del ciclo

### Fase A — SPEC (Claude)
Aggiunge un blocco `# Step N` con:
- Goal (1 paragrafo)
- Decisioni di design `D N.1 ... D N.k` (chiuse, non riapribili)
- Testcase XML obbligatori (TDD-first)
- File modificati attesi
- Acceptance criteria
- Tag `🚧 PRONTO PER GEMINI` nell'header

Solo UN step alla volta è 🚧. I successivi sono 🔒 LOCKED finché lo step precedente non è ✅.

### Fase B — IMPLEMENTAZIONE (Gemini)
1. Legge questo file, identifica lo step `🚧 PRONTO`.
2. Aggiunge **prima** i testcase XML in `tests/testsuite.xml`. Verifica che `cargo test` fallisca per il motivo atteso (TDD-first).
3. Implementa il codice secondo le decisioni `D N.k`. Niente improvvisazioni.
4. Verifica `cargo build` clean (0 warning) + `cargo test` verde.
5. **Un singolo commit** con il message format obbligatorio (sotto).
6. Aggiorna l'header dello step in questo file: `🚧 PRONTO` → `🟢 FATTO — AUDIT PENDING`.

Se incontra un'ambiguità non coperta dalle decisioni `D N.k`, **NON improvvisa**: lascia un commento `// DESIGN-Q: <domanda>` nel codice, committa l'avanzamento parziale taggato `🟡 BLOCKED — DESIGN-Q`, e Francesco lo segnala a Claude.

**Variante SPEC-Q** (aggiunta dopo iter 1): se un testcase con `expected_stdout` definito da Claude sembra contraddire le decisioni `D N.k` o il comportamento POSIX corretto, **NON cambiare silenziosamente l'expected**. Aggiungi nel testcase un commento XML `<!-- SPEC-Q: <motivo> -->`, taggalo come `🟡 BLOCKED — SPEC-Q`, e segnalalo a Francesco. Lo spec di Claude può avere bug — sono da chiarire, non da correggere a mano.

### Format commit message obbligatorio

```
feat(stepN): <titolo breve>

IN-SCOPE:
- <bullet list di cosa è stato implementato>

OUT-OF-SCOPE (debito esplicito):
- <bullet list di cosa è stato deliberatamente lasciato fuori>

Testcase aggiunti: N. Totali: M.
```

### Fase C — HANDOFF (Francesco)
Manda a Claude UN solo trigger:
- **`audita`** → Claude esegue Fase D (caso 90%).
- **`Gemini chiede: <domanda>`** → Claude risponde, modifica spec se serve.
- **`Gemini è bloccato: <motivo>`** → Claude debugga.

Mai copiare diff o messaggi. Il git log e i commit message contengono tutto.

### Fase D — AUDIT (Claude)
Sequenza fissa:
1. `git log --oneline -10` → identifica commit dal *last audited hash* in `## Audit Log`.
2. `cargo build 2>&1` + `cargo test 2>&1` → conferma green.
3. `git diff <last_hash> HEAD -- <file>` per ogni file modificato.
4. Verifica ogni decisione `D N.k` applicata letteralmente, non "in spirito".
5. Verifica conteggio testcase + commit message format.
6. Esecuzione live di 1-2 testcase chiave per scovare bug del compilatore di test.
7. Output strutturato:

```markdown
## Audit Step N — commit <hash>
**Verdetto**: ✅ APPROVED | 🟡 PARTIAL | ❌ REJECTED
**Build/Test**: green / N/M test
**Decisioni applicate**: D N.1 ✅, D N.2 🟡 (motivo), ...
**Leftovers** (se PARTIAL): - [ ] task 1
**Anchor**: auditato fino a `<hash>`
```

8. Se ✅: aggiorna `## Audit Log`, promuove il top del `## Backlog ordinato` a nuovo `# Step N+1` (Fase A), tagga 🚧.
9. Se 🟡: scrive `# Step N-bis` con i leftover come decisioni esplicite, tagga 🚧.
10. Se ❌: descrive il blocco in `## Audit Log`, lascia step com'era e ritorna a Gemini.

### Fase E — LOOP
Gemini riceve il nuovo step (o fix) e si ricomincia da Fase B.

## Anti-pattern del workflow

- ❌ Francesco copia messaggi di Gemini al posto del trigger `audita`. Costa context.
- ❌ Gemini implementa "in spirito" senza seguire le `D N.k`. Le D sono contratto.
- ❌ Claude fa audit "a memoria" senza diff reale. Sempre `git diff` + `cargo test`.
- ❌ Più step in un commit. Un commit per step, sempre.
- ❌ Gemini avvia uno step 🔒 LOCKED prima dell'audit ✅ del precedente.
- ❌ Claude promuove backlog senza prima aggiornare `## Audit Log`.

---

# Step 1 — Concatenazione invisibile + CONVFMT/OFMT

✅ **DONE — commit `3082b1a` (code) + 640462d (cleanup) — 42 test verdi**

Vedi `Step 1-bis` sotto per i task obbligatori prima di Step 2.

## Goal
Implementare l'operatore di concatenazione AWK (giustapposizione: `print "a" "b"` → `ab`) con la giusta precedenza POSIX, e introdurre CONVFMT/OFMT per evitare bug di rappresentazione float.

## Decisioni di design (NON riaprire)

### D1.1 — Precedenza POSIX
Concat sta **sopra** `==/!=/<=/>=/</>` e **sotto** `+/-`. Quindi:
- `1 + 2 "x" 3 + 4` → `Concat([Add(1,2), "x", Add(3,4)])` → `"3x7"` ✓
- `"ab" "c" == "abc"` → `Eq(Concat(["ab","c"]), "abc")` → `1` ✓

### D1.2 — Disambiguazione `f(x)` vs `f (x)` (LESSICALE, non semantica)
`func_call` richiede zero whitespace tra ident e `(`. Con spazio cade nel ramo concat.

Implementazione in `awk.pest` con compound atomic rule:
```pest
func_call = ${ ident ~ "(" ~ ws_or_nl* ~ expr_list? ~ ws_or_nl* ~ ")" }
ws_or_nl  = _{ " " | "\t" | "\r" | "\n" }
```
Il `${...}` (compound atomic) sopprime l'auto-insert di WHITESPACE solo al boundary `ident(`. Le sotto-rule funzionano normali.

**Test di accettazione D1.2**: `function add(x){return x+1} BEGIN{print add(5), add (5)}` → `6 6` (entrambi chiamano la funzione).

### D1.3 — Struttura grammatica: ABBANDONARE Pratt monolitico
Stratificare la grammatica in livelli espliciti. Mantenere `PrattParser` solo per `^` (right-assoc) o sostituirlo con regola right-recursive. Nuova grammatica completa per `awk.pest` (sostituisce le righe attuali da `expr` a `infix_op`):

```pest
expr           = { ternary_expr }
ternary_expr   = { logical_or ~ ("?" ~ expr ~ ":" ~ expr)? }
logical_or     = { logical_and ~ (op_or ~ logical_and)* }
logical_and    = { in_expr ~ (op_and ~ in_expr)* }
in_expr        = { match_expr ~ (op_in ~ ident)? }
match_expr     = { rel_expr ~ ((op_match | op_not_match) ~ rel_expr)* }
rel_expr       = { concat_expr ~ (rel_op ~ concat_expr)* }
rel_op         = _{ op_eq | op_neq | op_ge | op_le | op_gt | op_lt }
concat_expr    = { add_expr ~ add_expr* }
add_expr       = { mul_expr ~ ((op_add | op_sub) ~ mul_expr)* }
mul_expr       = { pow_expr ~ ((op_mul | op_div | op_mod) ~ pow_expr)* }
pow_expr       = { term ~ (op_pow ~ pow_expr)? }
term           = { prefix_op? ~ primary ~ postfix_op? }
```

`infix_op` come gruppo unico **non esiste più**. Ogni livello matcha solo i suoi operatori.

### D1.4 — AST
Aggiungere a `ast.rs`:
```rust
Expr::Concat(Vec<Expr>),
```
Variadic, non binario. In `parser.rs::parse_concat`:
- Se `add_expr` è uno solo → ritorna direttamente l'expr (no wrapping).
- Se `add_expr+` ≥ 2 → wrappa in `Concat`.

### D1.5 — CONVFMT e OFMT
Implementare in QUESTO Step (non rimandare).
- Default: `"%.6g"` per entrambi.
- Storage: nuovi campi `convfmt: String` e `ofmt: String` su `EvalContext`.
- `set_var("CONVFMT", v)` / `set_var("OFMT", v)` aggiornano i campi della struct (NON inserire in `self.vars`).

Aggiungere a `AwkValue`:
```rust
pub fn as_string_convfmt(&self, fmt: &str) -> String {
    match self {
        AwkValue::Uninitialized => String::new(),
        AwkValue::String(s) => s.clone(),
        AwkValue::StrNum(s, _) => s.clone(),  // preserva la stringa originale
        AwkValue::Number(n) => format_number_awk(*n, fmt),
    }
}
```

`format_number_awk(n, fmt)`:
- Se `n.is_finite() && n == n.trunc() && n.abs() < 1e16`: emetti `format!("{}", n as i64)`.
- Altrimenti: usa la crate `sprintf` con il `fmt` fornito.

Aggiungere `sprintf = "0.4"` (verifica versione corrente su crates.io) a `Cargo.toml`.

### D1.6 — Refactor `parser.rs`
Rimuovere completamente `parse_logical_expr` e `pratt()` (o limitarlo a `pow`). Aggiungere una funzione `parse_X` per ogni livello della grammatica D1.3:

```rust
fn parse_concat(pair: Pair<Rule>) -> Expr {
    let parts: Vec<Expr> = pair.into_inner().map(parse_add).collect();
    if parts.len() == 1 {
        parts.into_iter().next().unwrap()
    } else {
        Expr::Concat(parts)
    }
}

fn parse_add(pair: Pair<Rule>) -> Expr {
    let mut iter = pair.into_inner();
    let mut lhs = parse_mul(iter.next().unwrap());
    while let Some(op) = iter.next() {
        let rhs = parse_mul(iter.next().unwrap());
        let bop = match op.as_rule() {
            Rule::op_add => BinaryOperator::Add,
            Rule::op_sub => BinaryOperator::Sub,
            _ => unreachable!(),
        };
        lhs = Expr::BinaryOp(Box::new(lhs), bop, Box::new(rhs));
    }
    lhs
}
// ... idem per parse_mul, parse_pow, parse_rel, parse_match, parse_in, parse_logical_and, parse_logical_or, parse_ternary
```

### D1.7 — Modifiche `runner.rs::eval_expr`
- Nuovo case:
```rust
Expr::Concat(parts) => {
    let convfmt = context.convfmt.clone();
    let s: String = parts.iter()
        .map(|e| eval_expr(e, context).as_string_convfmt(&convfmt))
        .collect();
    AwkValue::String(s)
}
```
- In `Statement::Print`: rimpiazzare `as_string()` con `as_string_convfmt(&context.ofmt)` per tutti gli args (concat di `print a, b, c` con OFS).

## Testcase obbligatori (aggiungere a `tests/testsuite.xml` PRIMA del codice)

```xml
<testcase name="test_concat_basic">
    <awk><![CDATA[BEGIN { print "a" "b" "c" }]]></awk>
    <expected_stdout match="exact"><![CDATA[abc
]]></expected_stdout>
</testcase>
<testcase name="test_concat_field">
    <awk><![CDATA[{ print $1 $2 "-" $3 }]]></awk>
    <stdin><![CDATA[foo bar baz]]></stdin>
    <expected_stdout match="exact"><![CDATA[foobar-baz
]]></expected_stdout>
</testcase>
<testcase name="test_concat_paren">
    <awk><![CDATA[BEGIN { print "a" ("b" "c") "d" }]]></awk>
    <expected_stdout match="exact"><![CDATA[abcd
]]></expected_stdout>
</testcase>
<testcase name="test_concat_unary_precedence">
    <awk><![CDATA[BEGIN { print -1 "x" }]]></awk>
    <expected_stdout match="exact"><![CDATA[-1x
]]></expected_stdout>
</testcase>
<testcase name="test_concat_arith_precedence">
    <awk><![CDATA[BEGIN { print 1 + 2 "x" 3 + 4 }]]></awk>
    <expected_stdout match="exact"><![CDATA[3x7
]]></expected_stdout>
</testcase>
<testcase name="test_concat_func_call_disambig">
    <awk><![CDATA[
    function add(x) { return x + 1 }
    BEGIN { print add(5), add (5) }
    ]]></awk>
    <expected_stdout match="exact"><![CDATA[6 6
]]></expected_stdout>
</testcase>
<testcase name="test_concat_var_with_paren_expr">
    <awk><![CDATA[BEGIN { x = "hello"; print x (1 + 2) }]]></awk>
    <expected_stdout match="exact"><![CDATA[hello3
]]></expected_stdout>
</testcase>
<testcase name="test_concat_with_relational">
    <awk><![CDATA[BEGIN { if ("ab" "c" == "abc") print "ok" }]]></awk>
    <expected_stdout match="exact"><![CDATA[ok
]]></expected_stdout>
</testcase>
<testcase name="test_convfmt_default_float">
    <awk><![CDATA[BEGIN { x = 0.1 + 0.2; print "x=" x }]]></awk>
    <expected_stdout match="exact"><![CDATA[x=0.3
]]></expected_stdout>
</testcase>
<testcase name="test_convfmt_integer_no_decimal">
    <awk><![CDATA[BEGIN { print "n=" 42 ", m=" 3.0 }]]></awk>
    <expected_stdout match="exact"><![CDATA[n=42, m=3
]]></expected_stdout>
</testcase>
<testcase name="test_convfmt_custom">
    <awk><![CDATA[BEGIN { CONVFMT="%.2f"; x = 1/3; print "x=" x }]]></awk>
    <expected_stdout match="exact"><![CDATA[x=0.33
]]></expected_stdout>
</testcase>
```

## File modificati attesi

- `Cargo.toml` (+1 dep)
- `src/awk.pest` (~30 righe modificate)
- `src/ast.rs` (+1 variant)
- `src/parser.rs` (refactor consistente, ~50-80 righe nette)
- `src/types.rs` (+15 righe)
- `src/runner.rs` (+15 righe)
- `tests/testsuite.xml` (+11 testcase)

## Acceptance criteria

- [ ] `cargo build` clean (0 warning)
- [ ] `cargo test` verde, 31 + 11 = **42 testcase passano**
- [ ] Commit message dichiara IN-SCOPE: concat + CONVFMT/OFMT + disambiguazione lessicale `f(x)/f (x)`
- [ ] Commit message dichiara OUT-OF-SCOPE: format specifiers reali in printf (Step 2), CONVFMT con flag esotici (`'`, `_`)

---

# Step 1-bis — Cleanup process di Step 1

✅ DONE — commit 640462d

## Goal
Sanare le 4 process violations rilevate nell'audit di `3082b1a` senza toccare il codice funzionale (che è OK). Output: un commit pulito che porta Step 1 da 🟡 a ✅.

## Tasks (eseguire in ordine, UN solo commit finale)

### T1 — Estendi `.gitignore`
Aggiungi:
```
/target
.DS_Store
*.txt
src/scratch.rs
src/debug.rs
pest_test.rs
out.txt
dummy.txt
f1.txt
f2.txt
```
(Le `*.txt` sono inclusivi: nessun file `.txt` deve stare nel repo. Eccezione possibile in futuro: `LICENSE.txt`, ma adesso non c'è.)

### T2 — Rimuovi i file junk dal repo
```bash
git rm --cached .DS_Store f1.txt f2.txt src/scratch.rs pest_test.rs
# debug.rs, dummy.txt, out.txt non sono tracciati ma esistono su disco — il .gitignore li copre
```
Verifica con `git status` che siano gone dal tracking.

### T3 — Aggiorna header Step 1 in `NEXT_STEPS.md`
Cambia:
```
🟡 **PARTIAL — commit `3082b1a`, code OK ma cleanup pending**
```
in:
```
✅ **DONE — commit `3082b1a` (code) + <hash> (cleanup) — 42 test verdi**
```
(metti l'hash del commit di cleanup quando lo crei).

### T4 — Aggiorna header Step 2 e Step 1-bis
- Cambia Step 2 da `🔒 LOCKED` a `🚧 PRONTO PER GEMINI — attivato <data>`.
- Cambia Step 1-bis (questa sezione) da `🚧 PRONTO PER GEMINI` a `✅ DONE — commit <hash>`.

### T5 — Aggiorna `## Audit Log`
Aggiungi riga:
```
| 2026-05-03 | Step 1-bis (cleanup) | ✅ APPROVED | <hash> | 42/42 | Junk files rimossi, .gitignore esteso. Step 1 ora ✅. Step 2 sbloccato. |
```
**Nota**: questa è auto-attestazione di Gemini. Claude la verificherà al prossimo `audita`.

## Format commit message obbligatorio

```
chore(step1-bis): cleanup process violations

IN-SCOPE:
- Remove .DS_Store, f1.txt, f2.txt, src/scratch.rs, pest_test.rs from tracking
- Extend .gitignore to prevent re-adding
- Update NEXT_STEPS.md status tags
- Update audit log

OUT-OF-SCOPE:
- Functional code changes (Step 1 code is correct, no edits needed)

Testcase aggiunti: 0. Totali: 42.
```

## Acceptance criteria

- [ ] `git status` clean dopo il commit
- [ ] `cargo build` e `cargo test` ancora green (nessuna regressione: questo step non tocca `src/*.rs`)
- [ ] `git ls-files | grep -E "\.DS_Store|^f[12]\.txt$|scratch\.rs|pest_test\.rs"` deve essere vuoto
- [ ] `NEXT_STEPS.md` ha 3 header aggiornati e 1 nuova riga in Audit Log

## Anti-pattern da evitare

- ❌ Toccare `src/*.rs` (eccetto `src/scratch.rs` che va rimosso). Se senti il bisogno di "fixare anche X", non farlo: apri un task in backlog.
- ❌ Usare `git filter-branch` o `git rebase` per "cancellare" il commit `3082b1a`. Lasciamolo nella storia. Solo cleanup forward.
- ❌ Combinare questo cleanup con l'inizio di Step 2. Un commit per step.

---

# Step 2 — Printf / sprintf con format specifiers reali

✅ **DONE — commit `510d2c3` — 59 test verdi**

## ⚠️ T0 — Pre-cleanup obbligatorio (1 minuto)
Prima di iniziare il lavoro funzionale, rimuovi 4 file zombie ancora tracciati (residuo di Step 1-bis per inaccuratezza dello spec Claude):
```bash
git rm --cached debug.rs dummy.txt out.txt scratch.rs
```
Aggiungi a `.gitignore`: `/debug.rs` e `/scratch.rs` (root paths, distinti dai pattern `src/...` esistenti).

T0 va nello **stesso** commit di Step 2 (un solo commit per tutto), riportato nel IN-SCOPE come bullet finale.

## Format commit message obbligatorio (ripetuto qui per evitare oblio)

```
feat(step2): Printf/sprintf con format specifiers reali

IN-SCOPE:
- awk_sprintf con full POSIX format parsing (D2.1-D2.7)
- decode_string_escapes a parse-time
- Pre-cleanup T0: rimossi debug.rs/dummy.txt/out.txt/scratch.rs dal tracking

OUT-OF-SCOPE (debito esplicito):
- Position arguments POSIX (%2$d)
- Locale grouping flag (', _)
- Long modifier (l, ll, h)
- %a / %A (hex float)

Testcase aggiunti: 17. Totali: 59.
```

## Goal
Sostituire lo scanner ingenuo attuale di `sprintf` (che ignora width/precision/conversion e usa solo `as_string()`) con un'implementazione POSIX-compliant.

**Stato attuale buggato** (`runner.rs` ~410-440):
```rust
while format_char.is_ascii_digit() || format_char == '.' || format_char == '-' {
    format_char = chars.next().unwrap_or(' ');
}
let val = if arg_idx < args.len() { eval_expr(&args[arg_idx], context) } else { ... };
result.push_str(&val.as_string());  // ← ignora completamente lo spec!
```

## Decisioni di design (NON riaprire)

### D2.1 — Backend
Usare la crate `sprintf` (già aggiunta in Step 1). Verifica API su crates.io PRIMA dell'implementazione: `sprintf::sprintf(fmt: &str, args: &[&dyn Printf]) -> Result<String, _>`.

**Se l'API è dynamic-args e crea problemi di lifetime**, fallback: format scanner manuale che chiama `sprintf!` macro per UN solo arg alla volta su ogni chunk del format string. Vedi D2.5 per il pattern.

### D2.2 — Conversion mapping AWK → C printf
Tabella obbligatoria. Per ogni conversion char, applica questo cast prima di passare a sprintf:

| Conversion | AwkValue → cast | Note |
|---|---|---|
| `%d %i` | `as_number() as i64` | |
| `%o %x %X %u` | `as_number() as u64` | |
| `%c` | Vedi D2.3 | speciale |
| `%e %E %f %g %G` | `as_number() as f64` | |
| `%s` | `as_string()` | non-convfmt: stringa esplicita |
| `%%` | nessun arg, emit `%` | |

### D2.3 — `%c` speciale POSIX
- Se l'arg è `String` o `StrNum` con stringa non-vuota: prendi il **primo carattere** della stringa (UTF-8 char, non byte).
- Altrimenti: tratta `as_number() as u32` come codepoint Unicode, costruisci con `char::from_u32().unwrap_or('\0')`.

### D2.4 — Escape sequences nel parsing del literal stringa
Spostare la decodifica da runtime a parse-time. Aggiungere a `parser.rs`:

```rust
fn decode_string_escapes(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' { out.push(c); continue; }
        match chars.next() {
            Some('n')  => out.push('\n'),
            Some('t')  => out.push('\t'),
            Some('r')  => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some('"')  => out.push('"'),
            Some('/')  => out.push('/'),
            Some('a')  => out.push('\x07'),
            Some('b')  => out.push('\x08'),
            Some('f')  => out.push('\x0c'),
            Some('v')  => out.push('\x0b'),
            Some('0') => out.push('\0'),
            Some(other) => { out.push('\\'); out.push(other); }  // unknown: preserve
            None => out.push('\\'),
        }
    }
    out
}
```

Applicare in `parse_primary` quando matcha `Rule::string_literal`. Rimuovere il decode runtime in `sprintf`/`printf`.

### D2.5 — Format scanner manuale (pattern raccomandato)

Implementare in `runner.rs` (sostituendo l'attuale logica `"sprintf" => {...}`):

```rust
fn awk_sprintf(fmt: &str, args: &[AwkValue]) -> String {
    let mut out = String::new();
    let mut chars = fmt.chars().peekable();
    let mut arg_idx = 0;
    while let Some(c) = chars.next() {
        if c != '%' { out.push(c); continue; }
        // Caso speciale %% senza arg
        if chars.peek() == Some(&'%') {
            chars.next();
            out.push('%');
            continue;
        }
        // Accumula spec: flags + width + .precision + conversion
        let mut spec = String::from('%');
        loop {
            match chars.next() {
                None => { out.push_str(&spec); return out; }
                Some(ch) => {
                    spec.push(ch);
                    if "diouxXeEfgGcs".contains(ch) {
                        let arg = args.get(arg_idx).cloned()
                            .unwrap_or(AwkValue::Uninitialized);
                        arg_idx += 1;
                        out.push_str(&format_one(&spec, &arg));
                        break;
                    }
                }
            }
        }
    }
    out
}

fn format_one(spec: &str, arg: &AwkValue) -> String {
    let conv = spec.chars().last().unwrap();
    match conv {
        'd' | 'i' => sprintf::sprintf!(spec, arg.as_number() as i64).unwrap_or_default(),
        'o' | 'x' | 'X' | 'u' => sprintf::sprintf!(spec, arg.as_number() as u64).unwrap_or_default(),
        'c' => {
            let ch: char = match arg {
                AwkValue::String(s) | AwkValue::StrNum(s, _) if !s.is_empty() =>
                    s.chars().next().unwrap(),
                _ => char::from_u32(arg.as_number() as u32).unwrap_or('\0'),
            };
            // %c con sprintf: passa come stringa di lunghezza 1
            let one_char = ch.to_string();
            // Sostituisci %c con %s nello spec per usare sprintf string formatter
            let spec_s = spec.replacen('c', "s", 1);
            sprintf::sprintf!(&spec_s, one_char).unwrap_or_default()
        }
        'e' | 'E' | 'f' | 'g' | 'G' => sprintf::sprintf!(spec, arg.as_number()).unwrap_or_default(),
        's' => sprintf::sprintf!(spec, arg.as_string()).unwrap_or_default(),
        _ => spec.to_string(),
    }
}
```

**Nota su API sprintf crate**: la sintassi esatta della macro/funzione può variare. Verifica empiricamente con un piccolo prototipo prima di committare. Se la crate scelta non gestisce dynamic format strings, valuta `printf-compat` o implementa manualmente i casi più comuni (cost ~100 LOC, fattibile).

### D2.6 — Errori
Mai `panic!` o `std::process::exit(1)` da `awk_sprintf`/`format_one`. Comportamenti:
- Spec malformato (`%`+conversion sconosciuta): emit lo spec literale.
- Args insufficienti: usare `Uninitialized` (→ stringa vuota o 0 numerico).
- Args in sovrappiù: ignorare silently.

### D2.7 — Integration con `Statement::Printf`
Sostituire l'attuale:
```rust
Statement::Printf(exprs, redirect) => {
    let formatted = eval_expr(&Expr::FunctionCall("sprintf".to_string(), exprs.clone()), context).as_string();
    handle_output(&formatted, redirect, context);
}
```
Con:
```rust
Statement::Printf(exprs, redirect) => {
    let fmt = eval_expr(&exprs[0], context).as_string();
    let args: Vec<AwkValue> = exprs[1..].iter().map(|e| eval_expr(e, context)).collect();
    let formatted = awk_sprintf(&fmt, &args);
    handle_output(&formatted, redirect, context);
}
```

E `sprintf` come funzione builtin (in `eval_expr`):
```rust
"sprintf" => {
    if args.is_empty() { return AwkValue::String(String::new()); }
    let fmt = eval_expr(&args[0], context).as_string();
    let vals: Vec<AwkValue> = args[1..].iter().map(|e| eval_expr(e, context)).collect();
    AwkValue::String(awk_sprintf(&fmt, &vals))
}
```

## Testcase obbligatori (aggiungere a `tests/testsuite.xml` PRIMA del codice)

```xml
<testcase name="test_printf_int">
    <awk><![CDATA[BEGIN { printf "%d\n", 42 }]]></awk>
    <expected_stdout match="exact"><![CDATA[42
]]></expected_stdout>
</testcase>
<testcase name="test_printf_int_width_pad">
    <awk><![CDATA[BEGIN { printf "[%5d]\n", 42 }]]></awk>
    <expected_stdout match="exact"><![CDATA[[   42]
]]></expected_stdout>
</testcase>
<testcase name="test_printf_int_zeropad">
    <awk><![CDATA[BEGIN { printf "[%05d]\n", 42 }]]></awk>
    <expected_stdout match="exact"><![CDATA[[00042]
]]></expected_stdout>
</testcase>
<testcase name="test_printf_int_left_align">
    <awk><![CDATA[BEGIN { printf "[%-5d]\n", 42 }]]></awk>
    <expected_stdout match="exact"><![CDATA[[42   ]
]]></expected_stdout>
</testcase>
<testcase name="test_printf_int_negative">
    <awk><![CDATA[BEGIN { printf "%+d %+d\n", 5, -5 }]]></awk>
    <expected_stdout match="exact"><![CDATA[+5 -5
]]></expected_stdout>
</testcase>
<testcase name="test_printf_float_default">
    <awk><![CDATA[BEGIN { printf "%f\n", 3.14159 }]]></awk>
    <expected_stdout match="exact"><![CDATA[3.141590
]]></expected_stdout>
</testcase>
<testcase name="test_printf_float_precision">
    <awk><![CDATA[BEGIN { printf "%.2f\n", 3.14159 }]]></awk>
    <expected_stdout match="exact"><![CDATA[3.14
]]></expected_stdout>
</testcase>
<testcase name="test_printf_float_width_precision">
    <awk><![CDATA[BEGIN { printf "[%8.2f]\n", 3.14159 }]]></awk>
    <expected_stdout match="exact"><![CDATA[[    3.14]
]]></expected_stdout>
</testcase>
<testcase name="test_printf_string_align">
    <awk><![CDATA[BEGIN { printf "[%-10s|%10s]\n", "hi", "hi" }]]></awk>
    <expected_stdout match="exact"><![CDATA[[hi        |        hi]
]]></expected_stdout>
</testcase>
<testcase name="test_printf_string_truncate">
    <awk><![CDATA[BEGIN { printf "[%.3s]\n", "abcdef" }]]></awk>
    <expected_stdout match="exact"><![CDATA[[abc]
]]></expected_stdout>
</testcase>
<testcase name="test_printf_hex_oct">
    <awk><![CDATA[BEGIN { printf "%x %X %o\n", 255, 255, 8 }]]></awk>
    <expected_stdout match="exact"><![CDATA[ff FF 10
]]></expected_stdout>
</testcase>
<testcase name="test_printf_percent_literal">
    <awk><![CDATA[BEGIN { printf "100%%\n" }]]></awk>
    <expected_stdout match="exact"><![CDATA[100%
]]></expected_stdout>
</testcase>
<testcase name="test_printf_multiple_args">
    <awk><![CDATA[BEGIN { printf "%s=%d (%.2f%%)\n", "rate", 75, 75.456 }]]></awk>
    <expected_stdout match="exact"><![CDATA[rate=75 (75.46%)
]]></expected_stdout>
</testcase>
<testcase name="test_printf_string_escapes_in_literal">
    <awk><![CDATA[BEGIN { printf "a\tb\nc" }]]></awk>
    <expected_stdout match="exact"><![CDATA[a	b
c]]></expected_stdout>
</testcase>
<testcase name="test_sprintf_returns_value">
    <awk><![CDATA[BEGIN { s = sprintf("[%5d]", 7); print s }]]></awk>
    <expected_stdout match="exact"><![CDATA[[    7]
]]></expected_stdout>
</testcase>
<testcase name="test_printf_char_conversion">
    <awk><![CDATA[BEGIN { printf "%c%c%c\n", 65, 66, 67 }]]></awk>
    <expected_stdout match="exact"><![CDATA[ABC
]]></expected_stdout>
</testcase>
<testcase name="test_printf_char_from_string">
    <awk><![CDATA[BEGIN { printf "%c\n", "hello" }]]></awk>
    <expected_stdout match="exact"><![CDATA[h
]]></expected_stdout>
</testcase>
```

## File modificati attesi

- `src/parser.rs` (+30 righe per `decode_string_escapes` e suo uso)
- `src/runner.rs` (+80 righe nette: `awk_sprintf`, `format_one`, refactor di `Statement::Printf` e builtin `sprintf`)
- `tests/testsuite.xml` (+17 testcase)

## Acceptance criteria

- [ ] `cargo build` clean (0 warning)
- [ ] `cargo test` verde, 42 + 17 = **59 testcase passano**
- [ ] Commit message IN-SCOPE: format specifiers `%d %i %o %x %X %u %c %s %e %E %f %g %G %%` + flag `- + 0 #` + width + precision; escape sequences a parse time
- [ ] Commit message OUT-OF-SCOPE:
  - Position arguments POSIX (`%2$d`)
  - Locale grouping flag (`'`, `_`)
  - Long modifier (`l`, `ll`, `h`)
  - `%a` / `%A` (hex float, raro in awk)

---

# Step 3 — String literal escape `\xHH` e `\NNN` (octal)

✅ **DONE — commit `cec7a9d` — 66 test verdi**

## Format commit message obbligatorio (ripetuto qui per evitare oblio)

```
feat(step3): String literal escapes \xHH and \NNN (octal)

IN-SCOPE:
- decode_string_escapes esteso con \xHH (1-2 hex digits) e \NNN (1-3 octal digits)
- Integrazione del case \0 nel ramo octal generico

OUT-OF-SCOPE (debito esplicito):
- \uHHHH Unicode escape (4 hex) — gawk extension non POSIX
- \UHHHHHHHH Unicode (8 hex) — non POSIX
- Byte non-UTF8 puri (\xFF + others) — limitazione strutturale di Rust String

Testcase aggiunti: 7. Totali: 66.
```

## Goal
Completare `decode_string_escapes` (introdotto in Step 2) con il supporto per:
- `\xHH` — escape esadecimale (1 o 2 hex digits, case-insensitive)
- `\NNN` — escape ottale (1, 2, o 3 octal digits 0-7)

POSIX awk supporta entrambi. Attualmente `decode_string_escapes` gestisce solo `\n \t \r \\ \" \/ \a \b \f \v \0` e fallisce silenziosamente sui multi-char escape.

## Decisioni di design (NON riaprire)

### D3.1 — Sintassi accettata
- **Hex**: `\x` seguito da 1 o 2 hex digit (`0-9a-fA-F`). Greedy: prendi sempre il massimo possibile.
- **Octal**: `\` seguito da 1, 2, o 3 octal digit (`0-7`). Greedy: prendi sempre il massimo possibile.
- Se `\x` non è seguito da nessun hex digit → preserva `\x` literali (no panic).

### D3.2 — Range e overflow
- Hex: 0x00-0xFF → 1 byte. `\xHH` con due digit dà al massimo 0xFF, OK.
- Octal: `\0` (0) a `\777` (511 dec). Valore `> 0xFF` → applicare modulo 256 (POSIX-compliant).
- `\0` resta supportato come case zero del ramo octal generico (rimuovere il vecchio case speciale).

### D3.3 — Encoding nel target Rust
Il byte risultante è inserito nella stringa Rust come `char::from_u32(byte as u32).unwrap()`. I byte 0x00-0xFF sono code-point validi U+0000-U+00FF. Quindi `\xFF` diventa `'\u{ff}'` (UTF-8 a 2 byte). Questo diverge dal C awk che lavora su byte. Limitazione strutturale di Rust String (UTF-8) — documentare nel commit message come OUT-OF-SCOPE.

### D3.4 — Implementazione
Riscrivere `decode_string_escapes` in `parser.rs`. Il pattern (lookahead-based con `chars.peek()` invece di `chars.next()` come oggi):

```rust
fn decode_string_escapes(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' { out.push(c); continue; }
        match chars.peek().copied() {
            Some('n')  => { chars.next(); out.push('\n'); }
            Some('t')  => { chars.next(); out.push('\t'); }
            Some('r')  => { chars.next(); out.push('\r'); }
            Some('\\') => { chars.next(); out.push('\\'); }
            Some('"')  => { chars.next(); out.push('"'); }
            Some('/')  => { chars.next(); out.push('/'); }
            Some('a')  => { chars.next(); out.push('\x07'); }
            Some('b')  => { chars.next(); out.push('\x08'); }
            Some('f')  => { chars.next(); out.push('\x0c'); }
            Some('v')  => { chars.next(); out.push('\x0b'); }
            Some('x') => {
                chars.next();
                let mut hex = String::new();
                for _ in 0..2 {
                    if let Some(&h) = chars.peek() {
                        if h.is_ascii_hexdigit() { hex.push(h); chars.next(); } else { break; }
                    }
                }
                if !hex.is_empty() {
                    let val = u32::from_str_radix(&hex, 16).unwrap_or(0);
                    out.push(char::from_u32(val).unwrap_or('\0'));
                } else {
                    out.push('\\'); out.push('x');
                }
            }
            Some(d) if d.is_digit(8) => {
                let mut oct = String::new();
                for _ in 0..3 {
                    if let Some(&o) = chars.peek() {
                        if o.is_digit(8) { oct.push(o); chars.next(); } else { break; }
                    }
                }
                let val = u32::from_str_radix(&oct, 8).unwrap_or(0) % 256;
                out.push(char::from_u32(val).unwrap_or('\0'));
            }
            Some(other) => { chars.next(); out.push('\\'); out.push(other); }
            None => out.push('\\'),
        }
    }
    out
}
```

NOTA: il vecchio case `Some('0') => out.push('\0')` va RIMOSSO. È coperto dal nuovo ramo octal generico (`\0` matcha `is_digit(8)` su '0' e produce byte 0).

## Testcase obbligatori (aggiungere a `tests/testsuite.xml` PRIMA del codice)

```xml
<testcase name="test_escape_hex_basic">
    <awk><![CDATA[BEGIN { printf "%s\n", "\x41\x42\x43" }]]></awk>
    <expected_stdout match="exact"><![CDATA[ABC
]]></expected_stdout>
</testcase>
<testcase name="test_escape_hex_one_digit">
    <awk><![CDATA[BEGIN { printf "[%d]\n", length("\x9") }]]></awk>
    <expected_stdout match="exact"><![CDATA[[1]
]]></expected_stdout>
</testcase>
<testcase name="test_escape_octal_basic">
    <awk><![CDATA[BEGIN { printf "%s\n", "\101\102\103" }]]></awk>
    <expected_stdout match="exact"><![CDATA[ABC
]]></expected_stdout>
</testcase>
<testcase name="test_escape_octal_short">
    <awk><![CDATA[BEGIN { printf "[%d]\n", length("\7") }]]></awk>
    <expected_stdout match="exact"><![CDATA[[1]
]]></expected_stdout>
</testcase>
<testcase name="test_escape_octal_zero">
    <awk><![CDATA[BEGIN { printf "[%d]\n", length("\0") }]]></awk>
    <expected_stdout match="exact"><![CDATA[[1]
]]></expected_stdout>
</testcase>
<testcase name="test_escape_mixed_hex_octal">
    <awk><![CDATA[BEGIN { printf "%s\n", "\x48\x69\41" }]]></awk>
    <expected_stdout match="exact"><![CDATA[Hi!
]]></expected_stdout>
</testcase>
<testcase name="test_escape_unknown_preserved">
    <awk><![CDATA[BEGIN { printf "%s\n", "a\zb" }]]></awk>
    <expected_stdout match="exact"><![CDATA[a\zb
]]></expected_stdout>
</testcase>
```

## File modificati attesi

- `src/parser.rs` (~30 righe modificate in `decode_string_escapes`)
- `tests/testsuite.xml` (+7 testcase)

## Acceptance criteria

- [ ] `cargo build` clean (0 warning)
- [ ] `cargo test` verde, 59 + 7 = **66 testcase passano**
- [ ] Commit message format obbligatorio rispettato
- [ ] Tutti i testcase Step 1 e Step 2 ancora verdi (regression check)

---

# Step 4 — Builtin I/O: `system()`, `close()`, `fflush()`

✅ **DONE — commit `227c3e5` — 74 test verdi**

## Format commit message obbligatorio (ripetuto qui per evitare oblio)

```
feat(step4): I/O builtins system, close, fflush

IN-SCOPE:
- system(cmd): exec via /bin/sh -c, restituisce exit status
- close(target): flush+close di file/pipe, wait() su child di pipe per evitare zombie
- fflush(target): flush stream specifico o tutti se "" / no-arg
- Refactor out_files: HashMap<String, OutputStream> con enum File/Pipe per tracciare Child
- Final shutdown in run(): drain tutti gli stream prima di uscire

OUT-OF-SCOPE (debito esplicito):
- "cmd" | getline (input pipe — backlog #6)
- in_files cleanup tramite close() (sì lo gestiamo, ma getline-da-pipe arriverà dopo)
- system() che NON flush stdin del programma awk (non rilevante: awk non legge da stdin in modo bufferizzato fuori da getline)
```

## Goal
Implementare i 3 builtin I/O più richiesti per script reali:
- `system(cmd)` — esegue `cmd` via shell, ritorna exit status
- `close(target)` — chiude esplicitamente uno stream aperto, restituisce status (per pipe: exit del child)
- `fflush(target)` — forza flush di uno stream o di tutti

Senza `close()`, le pipe verso processi esterni lasciano zombie (`Child` mai `wait()`-ato). Senza `fflush()`, l'output verso file/pipe può non essere visibile fino al termine. Senza `system()`, niente shell-out.

## Decisioni di design (NON riaprire)

### D4.1 — Refactor data structure `out_files`
Cambia in `types.rs`:
```rust
pub enum OutputStream {
    File(Box<dyn std::io::Write>),
    Pipe { stdin: Box<dyn std::io::Write>, child: std::process::Child },
}
pub struct EvalContext {
    // ...
    pub out_files: HashMap<String, OutputStream>,
    // resto invariato
}
```
Aggiungi metodo helper:
```rust
impl OutputStream {
    pub fn writer(&mut self) -> &mut dyn std::io::Write {
        match self {
            OutputStream::File(w) => w.as_mut(),
            OutputStream::Pipe { stdin, .. } => stdin.as_mut(),
        }
    }
}
```

### D4.2 — `system(cmd)`
Aggiungi case in `eval_expr` per `FunctionCall("system", args)`:
```rust
"system" => {
    if args.is_empty() { return AwkValue::Number(0.0); }
    let cmd = eval_expr(&args[0], context).as_string();
    // Flush stdout per non mescolare output
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .status();
    let code = match status {
        Ok(s) => s.code().unwrap_or(-1),
        Err(_) => -1,
    };
    AwkValue::Number(code as f64)
}
```

### D4.3 — `close(target)`
Aggiungi case:
```rust
"close" => {
    if args.is_empty() { return AwkValue::Number(-1.0); }
    let target = eval_expr(&args[0], context).as_string();
    let mut status: i32 = 0;
    let mut found = false;
    
    if let Some(stream) = context.out_files.remove(&target) {
        found = true;
        match stream {
            OutputStream::File(_) => { /* drop chiude e flush, status=0 */ }
            OutputStream::Pipe { stdin, mut child } => {
                drop(stdin); // EOF al child
                if let Ok(s) = child.wait() {
                    status = s.code().unwrap_or(-1);
                } else {
                    status = -1;
                }
            }
        }
    }
    
    if context.in_files.remove(&target).is_some() {
        found = true;
    }
    
    if found { AwkValue::Number(status as f64) } else { AwkValue::Number(-1.0) }
}
```

### D4.4 — `fflush(target)`
```rust
"fflush" => {
    use std::io::Write;
    let target = if args.is_empty() {
        String::new()
    } else {
        eval_expr(&args[0], context).as_string()
    };
    
    if target.is_empty() {
        // Flush tutto: stdout + ogni out_files
        let mut ok = std::io::stdout().flush().is_ok();
        for stream in context.out_files.values_mut() {
            if stream.writer().flush().is_err() { ok = false; }
        }
        AwkValue::Number(if ok { 0.0 } else { -1.0 })
    } else if target == "stdout" || target == "/dev/stdout" {
        let r = std::io::stdout().flush();
        AwkValue::Number(if r.is_ok() { 0.0 } else { -1.0 })
    } else if let Some(stream) = context.out_files.get_mut(&target) {
        let r = stream.writer().flush();
        AwkValue::Number(if r.is_ok() { 0.0 } else { -1.0 })
    } else {
        AwkValue::Number(-1.0)
    }
}
```

### D4.5 — Update `handle_output` per costruire `OutputStream`
In `runner.rs`, sostituire la chiusura `or_insert_with` di `handle_output` per costruire `OutputStream::File` o `OutputStream::Pipe` invece di `Box<dyn Write>` direttamente. Per `|`: cattura sia `child.stdin` che `child` stesso.

```rust
let stream = context.out_files.entry(filename.clone()).or_insert_with(|| {
    if op == ">>" {
        OutputStream::File(Box::new(OpenOptions::new().create(true).append(true).open(&filename).unwrap()))
    } else if op == "|" {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&filename)
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        OutputStream::Pipe { stdin: Box::new(stdin), child }
    } else {
        OutputStream::File(Box::new(OpenOptions::new().create(true).write(true).truncate(true).open(&filename).unwrap()))
    }
});
write!(stream.writer(), "{}", output).unwrap();
```

### D4.6 — Final shutdown in `run()`
Alla fine di `pub fn run(...)`, dopo l'esecuzione dei blocchi END, drainare tutti gli stream rimasti per evitare zombie:
```rust
// Final cleanup: flush tutto, wait() su pipe children
use std::io::Write;
let _ = std::io::stdout().flush();
let streams: Vec<OutputStream> = context.out_files.drain().map(|(_,v)| v).collect();
for stream in streams {
    match stream {
        OutputStream::File(_) => {}
        OutputStream::Pipe { stdin, mut child } => {
            drop(stdin);
            let _ = child.wait();
        }
    }
}
```

## Testcase obbligatori (aggiungere a `tests/testsuite.xml` PRIMA del codice)

```xml
<testcase name="test_system_exit_code">
    <awk><![CDATA[BEGIN { x = system("exit 7"); print x }]]></awk>
    <expected_stdout match="exact"><![CDATA[7
]]></expected_stdout>
</testcase>
<testcase name="test_system_command_output">
    <awk><![CDATA[BEGIN { system("echo hello"); print "after" }]]></awk>
    <expected_stdout match="exact"><![CDATA[hello
after
]]></expected_stdout>
</testcase>
<testcase name="test_system_zero">
    <awk><![CDATA[BEGIN { print system("true") }]]></awk>
    <expected_stdout match="exact"><![CDATA[0
]]></expected_stdout>
</testcase>
<testcase name="test_close_file_reopen">
    <awk><![CDATA[BEGIN {
        print "first" > "/tmp/rawk_step4_close.txt"
        close("/tmp/rawk_step4_close.txt")
        print "second" > "/tmp/rawk_step4_close.txt"
        close("/tmp/rawk_step4_close.txt")
        getline line < "/tmp/rawk_step4_close.txt"
        print line
    }]]></awk>
    <expected_stdout match="exact"><![CDATA[second
]]></expected_stdout>
</testcase>
<testcase name="test_close_pipe">
    <awk><![CDATA[BEGIN {
        print "hello" | "cat"
        x = close("cat")
        print "x=" x
    }]]></awk>
    <expected_stdout match="exact"><![CDATA[hello
x=0
]]></expected_stdout>
</testcase>
<testcase name="test_close_unknown_returns_neg1">
    <awk><![CDATA[BEGIN { print close("not_open") }]]></awk>
    <expected_stdout match="exact"><![CDATA[-1
]]></expected_stdout>
</testcase>
<testcase name="test_fflush_no_arg">
    <awk><![CDATA[BEGIN { print "before"; fflush(); print "after" }]]></awk>
    <expected_stdout match="exact"><![CDATA[before
after
]]></expected_stdout>
</testcase>
<testcase name="test_fflush_specific_file">
    <awk><![CDATA[BEGIN {
        print "data" > "/tmp/rawk_step4_fflush.txt"
        fflush("/tmp/rawk_step4_fflush.txt")
        getline line < "/tmp/rawk_step4_fflush.txt"
        print "got: " line
        close("/tmp/rawk_step4_fflush.txt")
    }]]></awk>
    <expected_stdout match="exact"><![CDATA[got: data
]]></expected_stdout>
</testcase>
```

## File modificati attesi

- `src/types.rs` (+15 righe per `OutputStream` enum + helper)
- `src/runner.rs` (~50 righe nette: 3 builtin cases + handle_output update + final shutdown)
- `tests/testsuite.xml` (+8 testcase)

## Acceptance criteria

- [ ] `cargo build` clean (0 warning)
- [ ] `cargo test` verde, 66 + 8 = **74 testcase passano**
- [ ] Tutti i testcase Step 1-3 ancora verdi (regression check)
- [ ] Test integration robusto: i testcase con `/tmp/...` vanno cleanati o usano nomi unici (es. `/tmp/rawk_step4_*`) per evitare conflitti con run paralleli
- [ ] Nessun zombie process dopo run completo (Gemini può verificare con `ps` durante sviluppo, non parte dei test automatici)

## Anti-pattern specifici Step 4

- ❌ Lasciare `out_files: HashMap<String, Box<dyn Write>>` legacy "per compatibilità" — è un refactor totale, niente shim.
- ❌ Implementare `close()` senza `wait()` sui child di pipe (zombie process leak — è il bug che il fix vuole risolvere).
- ❌ Aggiungere builtin diversi da `system/close/fflush` "perché tanto siamo qui" (es. `gensub`, `mktime`). Sono backlog separato.

---

# Step 5 — Record Separator: paragraph mode + multi-char regex

✅ **DONE — commit `ffbc0fe` — 82 test verdi**

## Format commit message obbligatorio (ripetuto qui per evitare oblio)

```
feat(step5): RS paragraph mode and multi-char regex

IN-SCOPE:
- RS="" → paragraph mode (record separator = uno o più righe vuote)
- RS multi-char (≥ 2 caratteri) → trattato come regex (gawk extension)
- Refactor process_lines: lettura full-buffer poi split, anziché read_until streaming
- RT aggiornato con il match effettivo del separator

OUT-OF-SCOPE (debito esplicito):
- Streaming input per RS regex (per ora read-all in memoria; rilevante solo per file molto grandi)
- "cmd" | getline integrato con il nuovo modello (resta backlog #6)
- FS auto-include "\n" in paragraph mode (sì lo gestiamo, ma documentato)

Testcase aggiunti: 8. Totali: 82.
```

## Goal
Fixare `process_lines` in `runner.rs`. Oggi:
```rust
let delim = if rs_val.is_empty() { b'\n' } else { rs_val.as_bytes()[0] };
let bytes_read = reader.read_until(delim, &mut buffer)?;
```
Solo prende il primo byte di RS. Conseguenze:
- `RS=""` (paragraph mode POSIX) → cade nel default `\n` (sbagliato)
- `RS="ab"` → splita solo su `a` (sbagliato, gawk lo tratta come regex)
- `RS="\n\n"` → splita su prima `\n` (sbagliato, dovrebbe essere paragraph)

## Decisioni di design (NON riaprire)

### D5.1 — Tre modalità di splitting
- **Modalità A** (RS = 1 carattere): comportamento attuale. Streaming `read_until` byte-per-byte.
- **Modalità B** (RS = "" stringa vuota): paragraph mode. Record separati da una o più righe completamente vuote (`/\n\n+/`). Inoltre FS implicitly include "\n" (POSIX: in paragraph mode, FS default diventa `[\t\n ]+` o regex equivalente).
- **Modalità C** (RS ≥ 2 caratteri): treat as regex (compile + split). Default FS rimane.

### D5.2 — Implementation strategy: read-all + split
Per le modalità B e C, **non si può** usare `read_until` byte-per-byte. La via pragmatica:
- Modalità A: mantenere il loop streaming attuale (efficiente per il caso comune).
- Modalità B/C: leggere TUTTO l'input in una `String`, poi splittare.

Modificare `process_lines` per dispatch su una di tre branch all'inizio del loop, in base al valore corrente di RS letto via `context.get_var("RS")`.

### D5.3 — Branch B (paragraph mode)
```rust
fn process_paragraph<R: BufRead>(mut reader: R, context: &mut EvalContext, rules: &[CompiledRule]) -> anyhow::Result<FlowControl> {
    let mut all = String::new();
    reader.read_to_string(&mut all)?;
    // POSIX: split on /\n\n+/ — uno o più newline blocks
    // Trim leading newlines per non emettere record vuoto iniziale
    let trimmed = all.trim_start_matches('\n');
    let re = regex::Regex::new(r"\n\n+").unwrap();
    let mut last_end = 0;
    for mat in re.find_iter(trimmed) {
        let record = &trimmed[last_end..mat.start()];
        if !record.is_empty() {
            // RT = il match effettivo
            context.set_var("RT", AwkValue::String(mat.as_str().to_string()));
            context.update_record(record);
            // Esegui rules — usa lo stesso loop dei rules dello streaming
            let fc = run_rules_on_record(rules, context);
            if matches!(fc, FlowControl::Exit(_)) { return Ok(fc); }
        }
        last_end = mat.end();
    }
    // Ultimo paragrafo (senza trailing \n\n)
    let last = trimmed[last_end..].trim_end_matches('\n');
    if !last.is_empty() {
        context.set_var("RT", AwkValue::String(String::new()));
        context.update_record(last);
        let fc = run_rules_on_record(rules, context);
        if matches!(fc, FlowControl::Exit(_)) { return Ok(fc); }
    }
    Ok(FlowControl::None)
}
```

In paragraph mode, anche FS deve splittare su `\n\t ` per separare i campi (POSIX). Modificare `update_record` o aggiungere un check su `EvalContext`: se è in paragraph mode (helper `is_paragraph_mode()`), splittare i campi anche su `\n` oltre che FS.

Soluzione semplice: in `update_record`, se `self.fs == " "` E RS è vuoto, usare `split` regex `[ \t\n]+` invece di `split_whitespace()` (che già gestisce `\n`). In realtà `split_whitespace()` gestisce già `\n` come whitespace, quindi probabilmente basta lasciare la logica esistente. Verificare con un testcase.

### D5.4 — Branch C (regex multi-char RS)
```rust
fn process_regex_rs<R: BufRead>(mut reader: R, rs: &str, context: &mut EvalContext, rules: &[CompiledRule]) -> anyhow::Result<FlowControl> {
    let mut all = String::new();
    reader.read_to_string(&mut all)?;
    let re = match regex::Regex::new(rs) {
        Ok(r) => r,
        Err(_) => return Ok(FlowControl::None),  // RS regex invalido, ignora
    };
    let mut last_end = 0;
    for mat in re.find_iter(&all) {
        let record = &all[last_end..mat.start()];
        context.set_var("RT", AwkValue::String(mat.as_str().to_string()));
        context.update_record(record);
        let fc = run_rules_on_record(rules, context);
        if matches!(fc, FlowControl::Exit(_)) { return Ok(fc); }
        last_end = mat.end();
    }
    // Resto finale (input dopo l'ultimo match)
    let last = &all[last_end..];
    if !last.is_empty() {
        context.set_var("RT", AwkValue::String(String::new()));
        context.update_record(last);
        let fc = run_rules_on_record(rules, context);
        if matches!(fc, FlowControl::Exit(_)) { return Ok(fc); }
    }
    Ok(FlowControl::None)
}
```

### D5.5 — Helper `run_rules_on_record`
Estrarre il loop dei rules da `process_lines` in una funzione condivisa:
```rust
fn run_rules_on_record(rules: &[CompiledRule], context: &mut EvalContext) -> FlowControl {
    for rule in rules {
        let should_execute = match &rule.pattern {
            Some(CompiledPattern::Expr(e)) => eval_expr(e, context).is_truthy(),
            Some(CompiledPattern::Begin) | Some(CompiledPattern::End) | Some(CompiledPattern::BeginFile) | Some(CompiledPattern::EndFile) => false,
            None => true,
        };
        if should_execute {
            let fc = execute_action(&rule.action, context);
            if fc == FlowControl::Next { break; }
            if matches!(fc, FlowControl::Exit(_)) { return fc; }
        }
    }
    FlowControl::None
}
```
Refactor: `process_lines` originale chiama `run_rules_on_record` invece del loop inline. `process_paragraph` e `process_regex_rs` la chiamano allo stesso modo.

### D5.6 — Dispatch in `process_lines`
Il punto di ingresso `process_lines` legge RS UNA VOLTA all'inizio (è invariante per il file in input — RS può cambiare nel BEGIN block ma non a metà file in modo realistico). Dispatch:

```rust
fn process_lines<R: BufRead>(reader: R, context: &mut EvalContext, rules: &[CompiledRule]) -> anyhow::Result<FlowControl> {
    let rs_val = context.get_var("RS").as_string();
    if rs_val.is_empty() {
        process_paragraph(reader, context, rules)
    } else if rs_val.chars().count() == 1 {
        process_single_byte(reader, rs_val.as_bytes()[0], context, rules)
    } else {
        process_regex_rs(reader, &rs_val, context, rules)
    }
}
```

`process_single_byte` è il vecchio loop di `process_lines` (rinominato).

## Testcase obbligatori (aggiungere a `tests/testsuite.xml` PRIMA del codice)

```xml
<testcase name="test_rs_paragraph_basic">
    <awk><![CDATA[BEGIN { RS="" } { print NR ":" $0 }]]></awk>
    <stdin><![CDATA[foo bar
baz

second paragraph
line two

third]]></stdin>
    <expected_stdout match="exact"><![CDATA[1:foo bar
baz
2:second paragraph
line two
3:third
]]></expected_stdout>
</testcase>
<testcase name="test_rs_paragraph_multi_blank">
    <awk><![CDATA[BEGIN { RS="" } { print NR }]]></awk>
    <stdin><![CDATA[a


b



c]]></stdin>
    <expected_stdout match="exact"><![CDATA[1
2
3
]]></expected_stdout>
</testcase>
<testcase name="test_rs_paragraph_field_split">
    <awk><![CDATA[BEGIN { RS="" } { print NF }]]></awk>
    <stdin><![CDATA[one two
three four

five six]]></stdin>
    <expected_stdout match="exact"><![CDATA[4
2
]]></expected_stdout>
</testcase>
<testcase name="test_rs_multichar_regex">
    <awk><![CDATA[BEGIN { RS="X+" } { print NR ":" $0 }]]></awk>
    <stdin><![CDATA[aaaXbbbXXXcccXXdddX]]></stdin>
    <expected_stdout match="exact"><![CDATA[1:aaa
2:bbb
3:ccc
4:ddd
]]></expected_stdout>
</testcase>
<testcase name="test_rs_multichar_literal_string">
    <awk><![CDATA[BEGIN { RS="--" } { print NR ":" $0 }]]></awk>
    <stdin><![CDATA[foo--bar--baz]]></stdin>
    <expected_stdout match="exact"><![CDATA[1:foo
2:bar
3:baz
]]></expected_stdout>
</testcase>
<testcase name="test_rs_rt_paragraph">
    <awk><![CDATA[BEGIN { RS="" } { print NR ":" $0 ":[" RT "]" }]]></awk>
    <stdin><![CDATA[a

b


c]]></stdin>
    <expected_stdout match="exact"><![CDATA[1:a:[

]
2:b:[


]
3:c:[]
]]></expected_stdout>
</testcase>
<testcase name="test_rs_singlechar_unchanged">
    <awk><![CDATA[BEGIN { RS="|" } { print NR ":" $0 }]]></awk>
    <stdin><![CDATA[A|B|C]]></stdin>
    <expected_stdout match="exact"><![CDATA[1:A
2:B
3:C
]]></expected_stdout>
</testcase>
<testcase name="test_rs_default_newline_unchanged">
    <awk><![CDATA[{ print NR ":" $0 }]]></awk>
    <stdin><![CDATA[line1
line2
line3]]></stdin>
    <expected_stdout match="exact"><![CDATA[1:line1
2:line2
3:line3
]]></expected_stdout>
</testcase>
```

## File modificati attesi

- `src/runner.rs` (~80 righe nette: dispatch + 2 nuove funzioni + estrazione `run_rules_on_record`)
- `tests/testsuite.xml` (+8 testcase)

## Acceptance criteria

- [ ] `cargo build` clean (0 warning)
- [ ] `cargo test` verde, 74 + 8 = **82 testcase passano**
- [ ] Tutti i testcase Step 1-4 ancora verdi (regression check). In particolare il `test_record_separator` esistente (RS="|") deve continuare a passare.
- [ ] Niente regressioni sul caso default RS="\n" (modalità A più usata)

## Anti-pattern specifici Step 5

- ❌ Implementare paragraph mode con `read_until('\n\n')` o pseudo-streaming — non si può, serve buffer completo o pre-look.
- ❌ Trattare RS multi-char come "first byte then rest ignored" — è il bug che stiamo fixando.
- ❌ Aggiungere `nextfile` o altri statement "perché tanto sto modificando il loop" — backlog separato (#4).
- ❌ Modificare `update_record` per logica di FS paragraph-aware se non strettamente necessario (probabilmente non lo è — `split_whitespace()` gestisce già `\n`). Verificare con i testcase prima di toccarlo.

---

# Step 6 — Statement `nextfile`

🚧 **FATTO — AUDIT PENDING**

## Format commit message obbligatorio (ripetuto qui per evitare oblio)

```
feat(step6): nextfile statement

IN-SCOPE:
- Grammar: nextfile_stmt
- AST: Statement::NextFile
- FlowControl::NextFile variant
- Propagazione attraverso run_rules_on_record + process_* (esce dal loop record corrente)
- ENDFILE block ancora eseguito dopo nextfile (semantica POSIX)

OUT-OF-SCOPE (debito esplicito):
- Test multi-file richiede infrastruttura test diversa (oggi xml_runner_test usa solo stdin) — testiamo solo single-file behavior + END/ENDFILE ordering
- nextfile dentro ENDFILE/END (POSIX undefined behavior, lasciamo non-supportato)

Testcase aggiunti: 3. Totali: 85.
```

## Goal
Implementare lo statement AWK `nextfile` (POSIX). Semantica: interrompe il processing dei record del file corrente, esegue il blocco `ENDFILE`, e passa al file successivo (eseguendo `BEGINFILE` per quello). Equivalente a "fast-forward to end of current file".

Oggi `nextfile` non è in grammatica. Uno script come `BEGIN { } /skip/ { nextfile } { print }` darebbe parse error.

## Decisioni di design (NON riaprire)

### D6.1 — Grammar
In `awk.pest`, aggiungere `nextfile_stmt` accanto a `next_stmt`:
```pest
statement = {
    (if_stmt |
    ...
    next_stmt |
    nextfile_stmt |
    return_stmt |
    ...) ~ eos?
}
nextfile_stmt = { "nextfile" }
```

**ATTENZIONE alla precedenza pest**: `nextfile_stmt` deve venire PRIMA di `next_stmt` nell'alternazione, altrimenti `next` matcha il prefisso e `file` resta orfano (stesso problema risolto in Step 2 per `printf`/`print`).

### D6.2 — AST
In `ast.rs`:
```rust
Statement::NextFile,
```
Aggiungere il variant tra `Next` e `Return`.

### D6.3 — FlowControl
In `runner.rs`:
```rust
pub enum FlowControl {
    None,
    Break,
    Continue,
    Next,
    NextFile,   // ← nuovo
    Return(crate::types::AwkValue),
    Exit(i32),
}
```

### D6.4 — Parser
In `parser.rs::parse_statement`, aggiungere case:
```rust
Rule::nextfile_stmt => Statement::NextFile,
```

### D6.5 — Execute action
In `runner.rs::execute_action`, aggiungere case:
```rust
Statement::NextFile => return FlowControl::NextFile,
```

### D6.6 — Propagation in `run_rules_on_record`
Estendere il match per riconoscere NextFile e propagarlo:
```rust
let fc = execute_action(&rule.action, context);
if fc == FlowControl::Next { break; }
if fc == FlowControl::NextFile { return FlowControl::NextFile; }  // ← nuovo
if matches!(fc, FlowControl::Exit(_)) { return fc; }
```

### D6.7 — Propagation nelle 3 `process_*`
In `process_single_byte`, `process_paragraph`, `process_regex_rs`, dopo il loop di rules:
```rust
let fc = run_rules_on_record(rules, context);
if matches!(fc, FlowControl::Exit(_)) { return Ok(fc); }
if fc == FlowControl::NextFile { return Ok(FlowControl::None); }  // ← nuovo: esci dal file ma non dal run
```

Ritornare `Ok(FlowControl::None)` (non `NextFile`) perché il loop in `run()` su `files_to_process` continua naturalmente al file successivo. ENDFILE/BEGINFILE blocks sono già parte del flusso `run()` (eseguiti per ogni iterazione del for su `files_to_process`).

### D6.8 — Verifica ENDFILE
Il blocco `ENDFILE` viene eseguito in `run()` dopo `process_lines` per ogni file:
```rust
if let FlowControl::Exit(...) = process_lines(...)? { ... }
if let FlowControl::Exit(code) = execute_special_blocks(..., SpecialBlock::EndFile) { ... }
```
Quando `process_lines` ritorna `None` (incluso il caso NextFile), `ENDFILE` viene eseguito normalmente. ✓

## Testcase obbligatori (aggiungere a `tests/testsuite.xml` PRIMA del codice)

```xml
<testcase name="test_nextfile_single_file_endfile_runs">
    <awk><![CDATA[NR == 2 { nextfile } { print NR ":" $0 } ENDFILE { print "EF" } END { print "END:" NR }]]></awk>
    <stdin><![CDATA[a
b
c
d]]></stdin>
    <expected_stdout match="exact"><![CDATA[1:a
EF
END:2
]]></expected_stdout>
</testcase>
<testcase name="test_nextfile_first_record_no_print">
    <awk><![CDATA[NR == 1 { nextfile } { print NR ":" $0 } END { print "done" }]]></awk>
    <stdin><![CDATA[a
b
c]]></stdin>
    <expected_stdout match="exact"><![CDATA[done
]]></expected_stdout>
</testcase>
<testcase name="test_nextfile_in_user_function">
    <awk><![CDATA[
    function skip() { nextfile }
    NR == 2 { skip() }
    { print NR ":" $0 }
    END { print "END" }
    ]]></awk>
    <stdin><![CDATA[a
b
c]]></stdin>
    <expected_stdout match="exact"><![CDATA[1:a
END
]]></expected_stdout>
</testcase>
```

**Nota**: i testcase verificano solo single-file behavior + END/ENDFILE ordering. Multi-file requires test infrastructure changes (out-of-scope, vedi commit message).

Sul terzo testcase: `nextfile` da dentro una user function richiede che `FlowControl::NextFile` propaghi anche attraverso `Expr::FunctionCall` execution. Verifica nel codice di `eval_expr` per `FunctionCall` user-defined: oggi cattura `Return(val)` e ignora altri FlowControl. Probabilmente serve estendere la cattura a NextFile (e Exit, già gestito? verificare).

## File modificati attesi

- `src/awk.pest` (+1 rule per `nextfile_stmt`, +1 alternativa nello statement)
- `src/ast.rs` (+1 variant `Statement::NextFile`)
- `src/parser.rs` (+1 case in `parse_statement`)
- `src/runner.rs` (~10 righe nette: +1 variant FlowControl, +1 case execute_action, +3 propagation in process_*, +1 in run_rules_on_record, possibile fix in FunctionCall)
- `tests/testsuite.xml` (+3 testcase)

## Acceptance criteria

- [ ] `cargo build` clean (0 warning)
- [ ] `cargo test` verde, 82 + 3 = **85 testcase passano**
- [ ] Tutti i testcase Step 1-5 ancora verdi
- [ ] `nextfile_stmt` nell'alternazione DEVE venire prima di `next_stmt` (greedy-match issue come `printf`/`print`)

## Anti-pattern specifici Step 6

- ❌ Mettere `nextfile_stmt` dopo `next_stmt` — Pest matcha il prefisso `next` e `file` orfano dà errore.
- ❌ Implementare nextfile come "exit del run" anziché "exit del file corrente" — semantica POSIX richiede continuazione coi file successivi.
- ❌ Saltare ENDFILE block dopo nextfile — POSIX dice ENDFILE deve essere eseguito.
- ❌ Estendere il test runner XML per supportare multi-file — è un cambio infrastrutturale fuori scope di questo step.

---

# Anti-patterns globali del codice (controllo finale prima del commit)

- ❌ Dichiarare uno step "✅ fatto" se manca anche un solo sotto-task delle decisioni `D*.*`. Se incompleto, header → `🟡 PARTIAL` ed elenca i `TODO(stepN-bis):` nei file.
- ❌ Aggiungere features non in spec (es. gawk `gensub`, `mktime`, `nextfile`). Sono per step futuri del backlog.
- ❌ Riscrivere/forkare la crate `sprintf` se ha bug — apri issue upstream e usa workaround locale documentato.
- ❌ Lasciare warning `cargo build`. Sistema o `#[allow(...)]` con commento del perché.
- ❌ Mascherare errori con `.unwrap_or(default_silenzioso)` quando la spec dice "panic out". Solo dove la spec esplicita "mai panic" (es. `awk_sprintf`).

---

# Audit Log

Ogni audit di Claude termina aggiungendo una riga qui. La riga più recente è il *last audited hash* da cui parte il prossimo audit.

| Data | Step | Verdetto | Commit | Test | Note |
|---|---|---|---|---|---|
| 2026-05-03 | (pre-Step 1) | — | `453987f` | 31/31 | Baseline auditata. Step di "Step 1, 3, 5" del piano originale completati cumulativamente nei 3 commit `ed791b9` → `52c1645` → `453987f`. Workflow formalizzato a partire da qui. |
| 2026-05-03 | Step 1 (concat + CONVFMT) | 🟡 PARTIAL | `3082b1a` | 42/42 | Codice ✅: D1.1-D1.7 tutte applicate, build clean, test verdi. Process violations: commit message non conforme, file junk committati (.DS_Store, f1.txt, f2.txt, scratch.rs, pest_test.rs), header step non aggiornato, test `test_concat_func_call_disambig` silently amended (giustamente — era errore Claude — ma andava flaggato). Sblocco Step 2 condizionato a commit di cleanup. |
| 2026-05-03 | Step 1-bis (cleanup) | ✅ APPROVED | `640462d` | 42/42 | Junk files committati rimossi (`.DS_Store, f1.txt, f2.txt, src/scratch.rs, pest_test.rs`); `.gitignore` esteso. Caveat: 4 file zombie ancora tracciati (`debug.rs, dummy.txt, out.txt, scratch.rs` alla root) per inaccuratezza dello spec Claude — assegnati a T0 di Step 2. Step 1 ora ✅, Step 2 sbloccato. |
| 2026-05-03 | Step 2 (printf reale) | ✅ APPROVED | `510d2c3` | 59/59 | Tutte le D2.1-D2.7 applicate letteralmente (sprintf crate, awk_sprintf scanner, format_one mapping, decode_string_escapes a parse-time, integration in Statement::Printf e builtin). T0 cleanup completato: zombie files rimossi. Bonus fix non segnalato: swap `printf_stmt`/`print_stmt` in pest grammar per evitare greedy-match. Process: by the book. Backlog top → Step 3. |
| 2026-05-03 | Step 3 (escape `\xHH`/`\NNN`) | ✅ APPROVED | `cec7a9d` | 66/66 | D3.1-D3.4 applicate letteralmente. Implementazione di `decode_string_escapes` rifatta con `peek()` lookahead-based, hex e octal greedy-match, modulo 256 sull'octal overflow, fallback graceful per escape sconosciuti. 7 testcase con casi multibyte/zero/overflow/unknown. Process: by the book per il secondo step consecutivo. Backlog top → Step 4. |
| 2026-05-03 | Step 4 (system/close/fflush) | ✅ APPROVED | `227c3e5` | 74/74 | D4.1-D4.6 applicate letteralmente. Refactor `out_files: HashMap<_, OutputStream>` con enum File/Pipe per tracciare Child, `wait()` correttamente su close di pipe, final shutdown drain in `run()`. Side-fix non in spec ma documentato in commit message: introdotto `print_expr_list` per risolvere ambiguità POSIX `print x > "file"` (redirect vs comparison). Comportamento corretto POSIX. Avrebbe dovuto essere SPEC-Q ma trasparenza nel commit lo ha mitigato. Minor: manca riga "Testcase aggiunti: N. Totali: M." nel commit message. Backlog top → Step 5. |
| 2026-05-03 | Step 5 (RS paragraph + regex) | ✅ APPROVED | `ffbc0fe` | 82/82 | D5.1-D5.6 applicate letteralmente. Refactor `process_lines` in 3 funzioni (`process_single_byte`, `process_paragraph`, `process_regex_rs`) + dispatch top-level + helper `run_rules_on_record` estratto. Bonus sensato: default esplicito `RS="\n"` in `run()`. Commit message format perfettamente conforme (incluso "Testcase aggiunti: 8. Totali: 82.", che mancava in Step 4). 5° step consecutivo by-the-book. Backlog top → Step 6. |

---

# Backlog ordinato

Lista prioritaria dei prossimi step. Dopo ogni audit ✅, Claude prende il top e lo promuove a spec completa (Fase A). I numeri qui sono indicativi: lo step promosso prende il prossimo numero progressivo (Step 3, Step 4, ecc.).

1. ~~String literal escape `\xHH` e `\NNN` (octal)~~ → **promosso a Step 3** ✅ specced
2. ~~Builtin `system()` / `close()` / `fflush()`~~ → **promosso a Step 4** ✅ specced
3. ~~Paragraph mode `RS=""` + RS regex multi-char~~ → **promosso a Step 5** ✅ specced
4. ~~Statement `nextfile`~~ → **promosso a Step 6** ✅ specced
5. **NF assignment side-effects (donefld/donerec)** — `NF = 3` deve troncare `fields`; `$5 = "x"` con NF=3 deve estendere e ricostruire `$0` lazy.
6. **`getline` da pipe (`"cmd" | getline`)** — oggi solo `getline < file`. Estendere grammatica + runner.
7. **`printf`/`print` con `>>` append e `|` pipe** — già implementato in `handle_output`, ma testare edge case (file riaperti, append vs write, encoding).
8. **CLI: `-v var=value` reale e separatore `--`** — il flag `-v` esiste in clap ma le assegnazioni non vengono parsate e iniettate in `EvalContext`.
9. **Differential testing infrastructure** (Fase 4a della skill `legacy-port`) — `build.rs` con feature flag `differential`, FFI verso `c_awk/` compilato come libreria statica.
10. **Property-based testing** (Fase 4b) — `proptest` con property roundtrip e cross-roundtrip rawk vs c_awk.
11. **Refactor stilistico finale** — rimuovere tutti i `crate::types::AwkValue::...` qualificati, sostituire `eprintln+exit(1)` con `Result<_, AwkError>` propagato fino a `main`.

---

# Quando hai finito uno step

Aggiorna l'header dello step da `🚧 PRONTO PER GEMINI` a `🟢 FATTO — AUDIT PENDING`, fai il commit con il format obbligatorio, e poi **fermati**. Francesco triggera l'audit di Claude. **Non avviare lo step successivo** anche se il suo spec è già scritto: aspetta che `## Audit Log` registri ✅ e che il successivo passi da 🔒 LOCKED a 🚧 PRONTO PER GEMINI.

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

🚧 **PRONTO PER GEMINI** — _attivato 2026-05-03_

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

# Step 2 — Printf / sprintf con format specifiers reali

🔒 **LOCKED** — _attivare solo dopo audit ✅ di Step 1. La spec sotto è pronta ma può essere amendata se l'audit di Step 1 rivela vincoli nuovi._

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

---

# Backlog ordinato

Lista prioritaria dei prossimi step. Dopo ogni audit ✅, Claude prende il top e lo promuove a spec completa (Fase A). I numeri qui sono indicativi: lo step promosso prende il prossimo numero progressivo (Step 3, Step 4, ecc.).

1. **String literal escape `\xHH` e `\NNN` (octal)** — completare `decode_string_escapes` (introdotto in Step 2) con escape esadecimali e ottali.
2. **Builtin `system()` / `close()` / `fflush()`** — sblocca controllo I/O e shell-out. `close()` deve far `wait()` sui figli pipe per evitare zombie.
3. **Paragraph mode `RS=""` + RS regex multi-char** — fix critico in `process_lines`: oggi prende solo `rs_val.as_bytes()[0]`.
4. **Statement `nextfile`** — non in grammatica oggi. Aggiungere rule + AST + flusso.
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

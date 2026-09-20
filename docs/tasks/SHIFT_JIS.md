# Task: supporto Shift-JIS

## Obiettivo

Riprodurre il comportamento dell'AWK C per ja_JP.SJIS nei percorsi interessati:
conversioni maiuscole/minuscole, sequenze invalide/troncate e interazione con
unità carattere, regex, separatori e formattazione. Delimitare eventuali lacune
con evidenze; non dichiarare supporto completo sulla base di pochi esempi.

## Contesto minimo

1. [Regole](../../AGENTS.md) e [stato](../../STATO_PROGETTO.md).
2. [Rapporto legacy](../../diary/2026-09-20-legacy-locales.md).
3. [Sonda riproducibile](../../diary/verification-regex-performance-2026-09-20/legacy-probe.json).
4. `src/text.rs`, `src/ere.rs`, `src/unicode_ere.rs`, `src/runner/fmt.rs`;
   `tests/legacy_locale.rs` e `tests/unicode_contract.rs`.
5. Nel C: `run.c` (nawk_convert, u8_rune e formattazione), `main.c` (locale/MB_CUR_MAX).

## Metodo e accettazione

- Verificare la presenza effettiva della locale e costruire regressioni contro
  il C prima delle correzioni. Includere dati validi, invalidi, troncati e NUL.
- Il C usa il decoder strutturale BWK in alcune operazioni e la libc in altre:
  ricavare il contratto dal codice e dai risultati, non dal nome della codifica.
- Preservare C, UTF-8, ISO-8859-1/9, contratti binari e semantica delle regex.
- Eseguire test mirati, gate completo e suite release; registrare ciò che resta
  non verificabile. Linux nativo resta sospeso, non è un prerequisito locale.
- Aggiornare stato, rapporto e piano, consegnando secondo il modello.
- Nessuna riscrittura della home, nessuna estensione implicita ad altre codifiche.

La creazione del task organizza il lavoro. L'avvio dell'implementazione sarà
indicato dall'utente nel task dedicato; il primo messaggio serve al passaggio
di contesto, senza modifiche al runtime né pubblicazioni Git.

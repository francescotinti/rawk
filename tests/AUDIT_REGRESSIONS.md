# Inventario regressioni audit

Tutti i 29 casi iniziali sono corretti e verificati da `scripts/audit_regressions.py` contro il C originale (stdout e status). Le diagnostiche degli errori restano specifiche dell'implementazione e sono conservate nel JSON in `diary/audit-regression-results.json`.

I test permanenti sono distribuiti tra `cli_contract.rs` (CLI, safe e arità), `runtime_contract.rs` (flusso), `input_contract.rs` (contatori, ARGV e getline), `language_contract.rs` (sintassi, conversioni e array) e `data_contract.rs` (regex, split, sostituzioni e CSV). Il runner condiviso verifica anche stderr e timeout. I casi storici ulteriori e i limiti residui sono descritti nel rapporto del 19 settembre.

| Caso | Stato |
|---|---|
| exit_end | corretto e verificato |
| short_circuit | corretto e verificato |
| record_counter | corretto e verificato |
| fs_regex | corretto e verificato |
| rs_dynamic | corretto e verificato |
| regex_longest | corretto e verificato |
| regex_invalid | corretto e verificato |
| array_parameter | corretto e verificato |
| return_while | corretto e verificato |
| next_function | corretto e verificato |
| getline_file | corretto e verificato |
| getline_counter | corretto e verificato |
| getline_missing | corretto e verificato |
| split_empty_clear | corretto e verificato |
| gsub_count | corretto e verificato |
| numeric_string_compare | corretto e verificato |
| numeric_prefix | corretto e verificato |
| field_increment | corretto e verificato |
| escaped_quote | corretto e verificato |
| range_pattern | corretto e verificato |
| assignment_expr | corretto e verificato |
| leading_decimal | corretto e verificato |
| underscore_identifier | corretto e verificato |
| power_precedence | corretto e verificato |
| arity_panic | corretto e verificato |
| program_file_cli | corretto e verificato |
| argv_assignment | corretto e verificato |
| safe | corretto e verificato |
| csv | corretto e verificato |

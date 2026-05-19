#!/usr/bin/env bash
# Verification gates per il piano di adeguamento Rust idiomatico.
# Ogni funzione ritorna 0 se invariante OK, 1 altrimenti.
set -eu
cd "$(dirname "$0")/.."

check_no_macos_forks() {
  ! find . -name '._*' -not -path './target/*' -not -path './.git/*' | grep -q . \
    || { echo "FAIL: macOS forks presenti"; return 1; }
}
check_no_scratch_root() {
  for f in scratch scratch.rs parse_test.rs pest_test.rs debug.rs f1.txt f2.txt out.txt; do
    [ ! -e "$f" ] || { echo "FAIL: artefatto root '$f'"; return 1; }
  done
}
check_fmt() { cargo fmt --check >/dev/null; }
check_clippy() { cargo clippy --all-targets -- -D warnings >/dev/null 2>&1; }
check_tests() {
  # xml_runner_test contiene 1 test function che esegue internamente 109 casi XML.
  # cargo riporta quindi "1 passed", non "109 passed".
  # Verifichiamo summary OK + zero failures.
  cargo test --test xml_runner_test 2>&1 | grep -qE '^test result: ok\..*0 failed'
}
check_no_exit_outside_main() {
  found=$(grep -rn 'std::process::exit\|process::exit' src/ \
    | grep -v 'src/main.rs' | wc -l | tr -d ' ')
  [ "$found" = "0" ] || { echo "FAIL: $found process::exit() fuori main.rs"; return 1; }
}
check_runner_split() {
  [ -f src/runner/mod.rs ] && [ -f src/runner/builtins.rs ] \
    && [ -f src/runner/io.rs ] && [ -f src/runner/fmt.rs ] \
    || { echo "FAIL: runner non splittato"; return 1; }
}

run_all() { for fn in $(declare -F | awk '$3 ~ /^check_/ {print $3}'); do
  printf '%-30s ' "$fn"; $fn && echo OK; done; }

"${1:-run_all}"

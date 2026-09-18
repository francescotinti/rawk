#!/usr/bin/env bash
# Verification gates per il piano di adeguamento Rust idiomatico.
# Ogni funzione ritorna 0 se invariante OK, 1 altrimenti.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

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
check_tests() { cargo test --locked; }
check_no_exit_outside_main() {
  found=$( (grep -rn 'std::process::exit\|process::exit' src/ | grep -v 'src/main.rs' || true) | wc -l | tr -d ' ')
  [ "$found" = "0" ] || { echo "FAIL: $found process::exit() fuori main.rs"; return 1; }
}
check_runner_split() {
  [ -f src/runner/mod.rs ] && [ -f src/runner/builtins.rs ] \
    && [ -f src/runner/io.rs ] && [ -f src/runner/fmt.rs ] \
    || { echo "FAIL: runner non splittato"; return 1; }
}
check_diffrun_no_unexpected() {
  cargo build --locked --release --bins || return 1
  target/release/diffrun tests/testsuite.xml
}

run_all() {
  local fn result=0
  for fn in $(declare -F | awk '$3 ~ /^check_/ {print $3}'); do
    printf '%-30s ' "$fn"
    if "$fn"; then echo OK; else echo FAIL; result=1; fi
  done
  return "$result"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  "${1:-run_all}"
fi

#!/usr/bin/env bash
# Phase 7 pre-flight audit — produce report in diary/STEP19_PHASE7_AUDIT.md
set -euo pipefail
cd "$(dirname "$0")/.."

REPORT="diary/STEP19_PHASE7_AUDIT.md"

{
  echo "# Phase 7 — Pre-flight audit report"
  echo
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo
  echo "## (a) Testcase XML con stringhe non-ASCII"
  echo
  # cerca byte > 0x7f dentro <test>...</test>
  if rg --binary -l '[\x80-\xff]' tests/*.xml 2>/dev/null; then
    echo
    echo "(elenco file sopra)"
  else
    echo "Nessuno trovato — testsuite XML è ASCII pura."
  fi
  echo
  echo "## (b) Testcase con length()/substr()/index() su input multi-byte"
  echo
  rg -n 'length\(|substr\(|index\(' tests/testsuite.xml | head -30 || echo "(nessun match)"
  echo
  echo "## (c) Conteggio call site da migrare per file"
  echo
  echo '| File | String/&str hits |'
  echo '|------|-----------------:|'
  for f in src/*.rs src/runner/*.rs; do
    count=$(rg -c 'String|&str' "$f" 2>/dev/null || echo 0)
    echo "| \`$f\` | $count |"
  done
  echo
  echo "## (d) Verifica pest grammar su byte > 0x7f"
  echo
  echo "Test: \`echo 'BEGIN { print \"\\\\xff\" }' | cargo run -- -f /dev/stdin 2>&1\`"
  echo
  echo "Atteso: pest accetta escape \\NNN; verifica output:"
  echo
  echo '```'
  echo 'BEGIN { print "\xff" }' | cargo run -q -- 2>&1 | head -5 || echo "(rawk failed)"
  echo '```'
  echo
} > "$REPORT"

echo "Report written to $REPORT"

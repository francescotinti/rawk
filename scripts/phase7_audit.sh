#!/usr/bin/env bash
# Phase 7 pre-flight audit — produce report in diary/STEP19_PHASE7_AUDIT.md
# Uses portable POSIX tools (perl, BSD grep) instead of ripgrep, so it works
# outside Claude Code's interactive shell wrappers.
set -euo pipefail
cd "$(dirname "$0")/.."

REPORT="diary/STEP19_PHASE7_AUDIT.md"

# (a) helper: list XML files containing any byte 0x80-0xff
xml_with_non_ascii() {
  local f
  for f in tests/testsuite.xml tests/cases/*.xml; do
    [ -f "$f" ] || continue
    if perl -0777 -ne 'exit !/[\x80-\xff]/' "$f" 2>/dev/null; then
      echo "$f"
    fi
  done
}

{
  echo "# Phase 7 — Pre-flight audit report"
  echo
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo
  echo "## (a) Testcase XML con stringhe non-ASCII"
  echo
  hits=$(xml_with_non_ascii || true)
  if [ -n "$hits" ]; then
    printf '%s\n' "$hits"
    echo
    echo "(elenco file sopra)"
  else
    echo "Nessuno trovato — testsuite XML (manifest + 109 case) è ASCII pura."
  fi
  echo
  echo "## (b) Testcase con length()/substr()/index() su input multi-byte"
  echo
  echo "Manifest + cases (max 30 hit):"
  echo
  echo '```'
  grep -n -E 'length\(|substr\(|index\(' tests/testsuite.xml tests/cases/*.xml 2>/dev/null | head -30 || echo "(nessun match)"
  echo '```'
  echo
  echo "## (c) Conteggio call site da migrare per file"
  echo
  echo '| File | String/&str hits |'
  echo '|------|-----------------:|'
  for f in src/*.rs src/runner/*.rs; do
    [ -f "$f" ] || continue
    count=$(grep -cE 'String|&str' "$f" 2>/dev/null || true)
    count=${count:-0}
    echo "| \`$f\` | $count |"
  done
  echo
  echo "## (d) Verifica pest grammar su byte > 0x7f"
  echo
  echo "Test: \`cargo run -q -- 'BEGIN { print \"\\\\xff\" }'\`"
  echo
  echo "Atteso: pest accetta escape \\NNN; verifica output:"
  echo
  echo '```'
  cargo run -q -- 'BEGIN { print "\xff" }' 2>&1 | head -10 || echo "(rawk failed)"
  echo '```'
  echo
} > "$REPORT"

echo "Report written to $REPORT"

#!/usr/bin/env bash
# Fail closed on a different host architecture or a translated process.
set -euo pipefail
expected=${1:?expected native architecture required}
test "$(uname -s)" = Darwin
test "$(uname -m)" = "$expected"
translated=$(sysctl -in sysctl.proc_translated 2>/dev/null || true)
test "${translated:-0}" = 0
case "$expected" in
  x86_64) test "$(sysctl -n hw.optional.x86_64)" = 1 ;;
  arm64) test "$(sysctl -n hw.optional.arm64)" = 1 ;;
  *) exit 1 ;;
esac
uname -a
arch
sw_vers
printf 'sysctl.proc_translated=%s\n' "${translated:-unavailable (native)}"
sysctl machdep.cpu.brand_string
printf 'ImageOS=%s ImageVersion=%s\n' "${ImageOS:-unknown}" "${ImageVersion:-unknown}"
rustc -vV
rust_arch=$expected
if test "$expected" = arm64; then rust_arch=aarch64; fi
rustc -vV | grep -Fx "host: $rust_arch-apple-darwin" >/dev/null
cargo -V
cc --version
python3 --version
locale -a
python3 - <<'PY'
import locale
for name in ('C', 'en_US.UTF-8', 'tr_TR.UTF-8', 'en_US.ISO8859-1', 'tr_TR.ISO8859-9', 'ja_JP.SJIS'):
    print('active locale:', locale.setlocale(locale.LC_ALL, name), 'codeset:', locale.nl_langinfo(locale.CODESET))
PY
git rev-parse HEAD
git -C ../c_awk rev-parse HEAD
file ../c_awk/a.out
lipo ../c_awk/a.out -verify_arch "$expected"
otool -L ../c_awk/a.out

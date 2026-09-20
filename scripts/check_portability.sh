#!/usr/bin/env bash
# Native Unix gate. Darwin-specific corpus/diagnostic snapshots remain in
# checks.sh; these suites compare behavior with the locally built C oracle.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export LC_ALL=C
uname -a
rustc --version
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release --bins
suites=(--lib --bins
  --test unicode_contract --test legacy_locale --test dynamic_format --test driver_regressions
  --test input_contract --test runtime_contract --test data_contract
  --test language_contract --test bytes_array_keys --test bytes_printf
  --test bytes_regex_match --test harness)
cargo test --locked "${suites[@]}"
cargo test --locked --release "${suites[@]}"

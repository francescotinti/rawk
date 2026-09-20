# Native performance comparisons

Run the manual **Native performance comparison** GitHub Actions workflow with
immutable `before` and `after` revisions. The default baseline is `61009df`
(before priority 3); the candidate defaults to the dispatched workflow commit.
Four independent native jobs cover Linux x86-64/ARM64 and macOS Intel/ARM64.
There is no scheduled automation, automatic performance gate or fixed percent
threshold. Correctness CI remains separate and pinned to Rust 1.96.0.

Both revisions are exported from Git and built from scratch with Rust 1.96.0,
`cargo build --locked --release --bin rawk`, incremental compilation disabled,
empty Rust flags, separate target directories and a fresh Cargo home. Builds
finish before measurement. The pinned C oracle is built in an isolated checkout
with `CC=cc CFLAGS=-O2`; the developer's `../c_awk` is never altered. Artifacts
include compiler versions, source and binary hashes, runner image, hardware,
OS, build logs, corpus hashes, exact output hashes and all samples. Differences
in revision-owned Cargo profiles/configuration must be reviewed before attributing
changes to runtime code. Do not compare binaries built with different compilers.

For local reproduction, first install the chosen exact Rust toolchain and run:

```sh
python3 scripts/prepare_native_benchmark.py --before 61009dff859b06e85088dd7c3548a82fad0845c7 --after HEAD --work /tmp/rawk-native
# Build a clean pinned oracle in /tmp/rawk-native/oracle, following performance.yml.
python3 scripts/record_benchmark_oracle.py /tmp/rawk-native
python3 scripts/benchmark_native.py --before /tmp/rawk-native/before/target/release/rawk --after /tmp/rawk-native/after/target/release/rawk --oracle /tmp/rawk-native/oracle/a.out --metadata /tmp/rawk-native/metadata.json --output /tmp/rawk-native/results.json
```

Use a new work directory for each build. `--workload NAME` selects cases;
`--runs` defaults to nine and `--sessions` to two. Set `--before` and `--after`
to the same binary for a noise control. Do not build, test or profile concurrently
on that machine. Local development defaults (currently Rust 1.98.1 on the
maintainer's ARM64 Mac) do not override the explicit benchmark toolchain.

The 26 deterministic cases retain the priority 1–3 field, UTF-8 and I/O controls
and add high-cardinality aggregation, mixed quoted/multiline CSV and formatted
output. Mixed field widths, blank records, decimal/exponent values, dynamic
regexes, misses, expanding substitutions, long records, unterminated input,
paragraph/regex RS, getline, pipe input and 128 MiB files cover distinct costs.
This is a synthetic, reproducible corpus, not a representative census of real
users' workloads. Output-format measurement includes capturing its output;
other cases mostly emit compact aggregates. Pipe measurements include feeding
input from Python. File measurements use an inherited regular-file descriptor.

Each case establishes expected bytes from C, then runs one excluded warmup per
binary and nine interleaved blocks with rotating C/before/after order. Every
invocation checks status, empty stderr and exact stdout. A mismatch, missing RSS,
nonzero status or timeout fails the job and preserves a partial report marked
`complete: false`; no normalization is applied. Wall time includes process and
time-wrapper startup plus output capture. Linux GNU time reports peak RSS in
KiB, converted to bytes; Darwin time reports bytes. These are per-process peaks,
not allocator counts or a promise of lower memory consumption. Filesystem caches
are warm; short cases can be dominated by startup.

For each session the report retains every wall/RSS sample and a deterministic
paired bootstrap interval for the median after/before ratio. The interval is
exploratory and conditional on that session: nine samples, scheduling noise,
autocorrelation and multiple comparisons limit its interpretation. `slower` or
`faster` indicates an interval on one side of 1; `repeatable_*` requires the same
signal in both sessions. It is a candidate for investigation, not an automatic
regression verdict. The sessions are consecutive on the same host and are not
independent machines. Confirm with another dispatch and same-revision control,
inspect distributions, absolute effect size and RSS before deciding. Hosted
runner variation prohibits absolute-time comparisons across jobs/platforms.
Keep negative and inconclusive results, and never average architectures together.

The documented regex RS EOF/`$0` discrepancy remains open. `regex_control` sums
record lengths during actions, where C and rawk agree; it does not normalize the
divergent END value. The runtime and compatibility expectations are unchanged.

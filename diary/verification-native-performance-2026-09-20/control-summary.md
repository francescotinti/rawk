# Native performance observations

Each row compares paired binaries on one runner. Percentages are per-session
median paired wall-time changes; RSS values are medians, not allocation counts.
Signals are exploratory; inspect raw distributions and same-revision controls.

## macOS-15.7.9-x86_64-i386-64bit-Mach-O (x86_64)

Before `36492979de6a222c54d4110f9c06b14b53fa85ac`; after `36492979de6a222c54d4110f9c06b14b53fa85ac`.

Source: `results.json`; compiler: `rustc 1.96.0 (ac68faa20 2026-05-25)`.

| Case | Wall before → after seconds by session | Wall change by session | Wall signal | RSS before → after MiB by session |
|---|---|---|---|---|
| sum_fields | 0.1719 → 0.1692; 0.1565 → 0.1617 | -0.9%; -0.2% | requires_confirmation | 2.83 → 2.83; 2.85 → 2.83 |
| regex_fields | 0.0456 → 0.0447; 0.0469 → 0.0443 | -3.7%; -1.8% | requires_confirmation | 3.36 → 3.36; 3.36 → 3.36 |
| array_aggregation | 0.2457 → 0.2437; 0.2737 → 0.2624 | +0.6%; -5.7% | requires_confirmation | 2.86 → 2.86; 2.87 → 2.86 |
| mixed_fields | 0.3422 → 0.3384; 0.3231 → 0.3297 | +1.9%; +0.5% | requires_confirmation | 2.87 → 2.88; 2.87 → 2.87 |
| mixed_aggregation | 0.2544 → 0.2522; 0.2509 → 0.2716 | -0.7%; +1.8% | requires_confirmation | 2.89 → 2.89; 2.89 → 2.93 |
| utf8_dynamic_boolean | 0.0496 → 0.0497; 0.0528 → 0.0522 | -0.8%; -5.0% | requires_confirmation | 3.43 → 3.43; 3.43 → 3.43 |
| utf8_match_mixed | 0.0584 → 0.0582; 0.0532 → 0.0518 | -1.5%; -2.1% | requires_confirmation | 3.50 → 3.50; 3.51 → 3.50 |
| utf8_gsub_expanding | 0.0704 → 0.0711; 0.0755 → 0.0753 | -1.9%; +0.5% | requires_confirmation | 3.52 → 3.52; 3.52 → 3.52 |
| utf8_gsub_miss | 0.0450 → 0.0475; 0.0453 → 0.0445 | -1.5%; +3.6% | requires_confirmation | 3.27 → 3.27; 3.27 → 3.27 |
| utf8_match_long | 0.0684 → 0.0651; 0.0726 → 0.0722 | -3.4%; -0.5% | requires_confirmation | 3.52 → 3.52; 3.52 → 3.52 |
| utf8_empty_regex | 0.4695 → 0.4628; 0.4564 → 0.4541 | -1.4%; -1.2% | requires_confirmation | 3.40 → 3.43; 3.41 → 3.40 |
| short_file | 0.7424 → 0.7200; 0.8053 → 0.8405 | -2.4%; +3.2% | requires_confirmation | 2.88 → 2.86; 2.86 → 2.88 |
| long_1048576_file | 0.0517 → 0.0526; 0.0510 → 0.0484 | +2.4%; -3.0% | requires_confirmation | 7.95 → 7.95; 7.96 → 7.95 |
| long_4194304_file | 0.0582 → 0.0660; 0.0618 → 0.0622 | +3.5%; -0.4% | requires_confirmation | 22.95 → 22.95; 22.95 → 22.95 |
| long_8388608_file | 0.0893 → 0.0825; 0.0769 → 0.0745 | -9.1%; -2.9% | requires_confirmation | 42.95 → 42.95; 42.95 → 42.95 |
| long_pipe | 0.4327 → 0.4361; 0.4336 → 0.4511 | -2.4%; +2.8% | requires_confirmation | 22.97 → 22.98; 22.97 → 22.98 |
| long_getline | 0.0503 → 0.0552; 0.0461 → 0.0479 | -1.4%; +1.4% | requires_confirmation | 18.97 → 18.97; 18.97 → 18.97 |
| unterminated | 0.0560 → 0.0549; 0.0565 → 0.0560 | +0.3%; +2.9% | requires_confirmation | 42.95 → 42.95; 42.95 → 42.95 |
| csv_long | 0.0835 → 0.0818; 0.0801 → 0.0781 | -2.2%; -2.3% | requires_confirmation | 21.94 → 21.94; 21.94 → 21.94 |
| paragraph_control | 0.0160 → 0.0159; 0.0156 → 0.0164 | -1.7%; +1.5% | requires_confirmation | 3.35 → 3.35; 3.35 → 3.35 |
| regex_control | 0.0170 → 0.0173; 0.0215 → 0.0219 | +3.4%; -0.4% | requires_confirmation | 3.35 → 3.35; 3.35 → 3.35 |
| volume_short_128m | 6.0127 → 6.0230; 5.0937 → 5.0911 | +2.9%; +1.5% | requires_confirmation | 2.89 → 2.89; 2.89 → 2.89 |
| volume_64k_128m | 0.1776 → 0.1751; 0.1795 → 0.1823 | +0.6%; +2.4% | requires_confirmation | 3.20 → 3.20; 3.20 → 3.20 |
| csv_mixed | 0.2751 → 0.2744; 0.2654 → 0.2550 | +1.3%; -0.5% | requires_confirmation | 2.88 → 2.88; 2.89 → 2.91 |
| high_cardinality | 0.4576 → 0.4566; 0.4571 → 0.4520 | +0.5%; -1.6% | requires_confirmation | 11.55 → 11.55; 11.55 → 11.55 |
| output_format | 0.1809 → 0.1842; 0.1776 → 0.1764 | +0.8%; -1.8% | requires_confirmation | 2.89 → 2.89; 2.89 → 2.89 |

## macOS-15.7.9-arm64-arm-64bit-Mach-O (arm64)

Before `36492979de6a222c54d4110f9c06b14b53fa85ac`; after `36492979de6a222c54d4110f9c06b14b53fa85ac`.

Source: `results.json`; compiler: `rustc 1.96.0 (ac68faa20 2026-05-25)`.

| Case | Wall before → after seconds by session | Wall change by session | Wall signal | RSS before → after MiB by session |
|---|---|---|---|---|
| sum_fields | 0.0550 → 0.0513; 0.0536 → 0.0510 | -6.3%; -5.7% | requires_confirmation | 4.06 → 4.05; 3.98 → 4.03 |
| regex_fields | 0.0168 → 0.0183; 0.0190 → 0.0183 | -0.1%; -9.2% | requires_confirmation | 4.61 → 4.61; 4.61 → 4.61 |
| array_aggregation | 0.0878 → 0.0837; 0.0879 → 0.0825 | -2.0%; -0.1% | requires_confirmation | 4.09 → 4.08; 4.08 → 4.08 |
| mixed_fields | 0.1309 → 0.1522; 0.1548 → 0.1389 | +12.2%; -4.0% | requires_confirmation | 4.12 → 4.17; 4.17 → 4.17 |
| mixed_aggregation | 0.1218 → 0.1582; 0.1369 → 0.1279 | +16.8%; -2.8% | requires_confirmation | 4.17 → 4.17; 4.17 → 4.17 |
| utf8_dynamic_boolean | 0.0275 → 0.0230; 0.0233 → 0.0232 | -6.8%; +1.1% | requires_confirmation | 5.05 → 4.89; 5.02 → 4.89 |
| utf8_match_mixed | 0.0242 → 0.0245; 0.0223 → 0.0220 | -5.6%; -7.9% | requires_confirmation | 4.97 → 4.97; 4.97 → 4.78 |
| utf8_gsub_expanding | 0.0337 → 0.0329; 0.0336 → 0.0335 | +2.4%; -1.1% | requires_confirmation | 4.95 → 4.95; 4.80 → 4.98 |
| utf8_gsub_miss | 0.0183 → 0.0178; 0.0231 → 0.0239 | -2.8%; +6.6% | requires_confirmation | 4.73 → 4.77; 4.78 → 4.77 |
| utf8_match_long | 0.0607 → 0.0607; 0.0535 → 0.0528 | -7.1%; -16.3% | requires_confirmation | 5.02 → 5.03; 5.22 → 5.25 |
| utf8_empty_regex | 0.1436 → 0.1469; 0.1504 → 0.1644 | +4.5%; +1.8% | requires_confirmation | 4.84 → 4.84; 4.84 → 4.84 |
| short_file | 0.3148 → 0.2953; 0.2418 → 0.2555 | -1.2%; +3.3% | requires_confirmation | 4.09 → 4.02; 4.02 → 4.09 |
| long_1048576_file | 0.0366 → 0.0295; 0.0330 → 0.0317 | -20.9%; -2.5% | requires_confirmation | 12.11 → 12.17; 11.22 → 12.17 |
| long_4194304_file | 0.0424 → 0.0343; 0.0326 → 0.0323 | -16.4%; -0.6% | requires_confirmation | 32.30 → 32.14; 24.05 → 24.05 |
| long_8388608_file | 0.0407 → 0.0351; 0.0477 → 0.0380 | -2.2%; -8.4% | requires_confirmation | 44.06 → 52.16; 60.14 → 52.17 |
| long_pipe | 0.1577 → 0.1533; 0.1385 → 0.1472 | +4.4%; +2.6% | requires_confirmation | 40.19 → 40.20; 36.17 → 36.27 |
| long_getline | 0.0283 → 0.0251; 0.0302 → 0.0272 | -13.0%; -19.1% | repeatable_faster | 20.06 → 32.19; 20.06 → 20.06 |
| unterminated | 0.0249 → 0.0257; 0.0269 → 0.0258 | -1.8%; -6.7% | requires_confirmation | 44.17 → 44.03; 44.03 → 44.17 |
| csv_long | 0.0573 → 0.0577; 0.0545 → 0.0621 | -0.8%; +2.2% | requires_confirmation | 10.14 → 15.52; 15.52 → 21.94 |
| paragraph_control | 0.0104 → 0.0099; 0.0103 → 0.0101 | -0.9%; -1.4% | requires_confirmation | 4.61 → 4.61; 4.61 → 4.61 |
| regex_control | 0.0095 → 0.0098; 0.0100 → 0.0097 | +1.7%; +0.9% | requires_confirmation | 4.61 → 4.61; 4.61 → 4.61 |
| volume_short_128m | 1.9249 → 1.9674; 2.0648 → 2.0926 | +0.3%; +2.1% | requires_confirmation | 4.09 → 4.09; 4.09 → 4.09 |
| volume_64k_128m | 0.2080 → 0.2024; 0.2402 → 0.2254 | +3.8%; -18.4% | requires_confirmation | 4.95 → 4.95; 4.95 → 4.95 |
| csv_mixed | 0.0977 → 0.1009; 0.0981 → 0.0983 | +1.5%; -0.2% | requires_confirmation | 4.11 → 4.11; 4.11 → 4.05 |
| high_cardinality | 0.2288 → 0.2278; 0.2136 → 0.2163 | -3.5%; -3.7% | requires_confirmation | 14.00 → 14.00; 14.00 → 13.98 |
| output_format | 0.0833 → 0.0786; 0.0822 → 0.0798 | -7.6%; -3.1% | requires_confirmation | 4.08 → 4.08; 4.08 → 4.08 |

## Linux-6.17.0-1022-azure-aarch64-with-glibc2.39 (aarch64)

Before `36492979de6a222c54d4110f9c06b14b53fa85ac`; after `36492979de6a222c54d4110f9c06b14b53fa85ac`.

Source: `results.json`; compiler: `rustc 1.96.0 (ac68faa20 2026-05-25)`.

| Case | Wall before → after seconds by session | Wall change by session | Wall signal | RSS before → after MiB by session |
|---|---|---|---|---|
| sum_fields | 0.0432 → 0.0432; 0.0432 → 0.0431 | -0.0%; -0.1% | requires_confirmation | 3.71 → 3.71; 3.71 → 3.71 |
| regex_fields | 0.0100 → 0.0100; 0.0099 → 0.0098 | -0.1%; -0.2% | requires_confirmation | 4.34 → 4.34; 4.34 → 4.34 |
| array_aggregation | 0.0628 → 0.0627; 0.0626 → 0.0628 | -0.2%; +0.3% | requires_confirmation | 3.71 → 3.71; 3.71 → 3.71 |
| mixed_fields | 0.0983 → 0.0983; 0.0982 → 0.0982 | +0.1%; -0.0% | requires_confirmation | 3.71 → 3.71; 3.71 → 3.71 |
| mixed_aggregation | 0.0795 → 0.0796; 0.0795 → 0.0796 | +0.0%; -0.0% | requires_confirmation | 3.71 → 3.71; 3.71 → 3.71 |
| utf8_dynamic_boolean | 0.0127 → 0.0127; 0.0126 → 0.0126 | -0.0%; -0.3% | requires_confirmation | 4.88 → 4.88; 4.88 → 4.88 |
| utf8_match_mixed | 0.0151 → 0.0150; 0.0151 → 0.0151 | -0.1%; +0.4% | requires_confirmation | 4.82 → 4.82; 4.82 → 4.82 |
| utf8_gsub_expanding | 0.0226 → 0.0226; 0.0225 → 0.0226 | +0.2%; +0.1% | requires_confirmation | 4.82 → 4.82; 4.82 → 4.82 |
| utf8_gsub_miss | 0.0104 → 0.0103; 0.0104 → 0.0105 | -0.6%; +0.4% | requires_confirmation | 4.70 → 4.70; 4.70 → 4.70 |
| utf8_match_long | 0.0279 → 0.0277; 0.0281 → 0.0281 | -0.7%; +0.1% | requires_confirmation | 4.82 → 4.82; 4.82 → 4.82 |
| utf8_empty_regex | 0.1412 → 0.1412; 0.1415 → 0.1413 | -0.0%; -0.1% | requires_confirmation | 4.70 → 4.70; 4.70 → 4.70 |
| short_file | 0.2216 → 0.2217; 0.2218 → 0.2217 | +0.0%; -0.0% | requires_confirmation | 3.71 → 3.71; 3.71 → 3.71 |
| long_1048576_file | 0.0259 → 0.0259; 0.0260 → 0.0260 | -0.4%; -0.1% | requires_confirmation | 8.58 → 8.58; 8.58 → 8.58 |
| long_4194304_file | 0.0324 → 0.0324; 0.0324 → 0.0328 | -0.9%; +0.7% | requires_confirmation | 23.57 → 23.58; 23.58 → 23.58 |
| long_8388608_file | 0.0409 → 0.0409; 0.0408 → 0.0407 | -0.0%; -0.9% | requires_confirmation | 43.58 → 43.58; 43.58 → 43.57 |
| long_pipe | 0.0354 → 0.0354; 0.0355 → 0.0354 | +0.5%; -0.1% | requires_confirmation | 23.58 → 23.58; 23.58 → 23.58 |
| long_getline | 0.0238 → 0.0234; 0.0243 → 0.0242 | +0.0%; -0.7% | requires_confirmation | 19.58 → 19.66; 19.64 → 19.66 |
| unterminated | 0.0273 → 0.0272; 0.0271 → 0.0269 | -1.8%; -0.9% | requires_confirmation | 43.52 → 43.52; 43.52 → 43.52 |
| csv_long | 0.0454 → 0.0453; 0.0452 → 0.0451 | -0.4%; -0.4% | requires_confirmation | 9.59 → 9.59; 9.59 → 9.59 |
| paragraph_control | 0.0048 → 0.0048; 0.0049 → 0.0048 | -0.3%; -0.6% | requires_confirmation | 4.34 → 4.34; 4.34 → 4.33 |
| regex_control | 0.0050 → 0.0049; 0.0050 → 0.0049 | -1.6%; -0.7% | requires_confirmation | 4.34 → 4.34; 4.34 → 4.34 |
| volume_short_128m | 1.7547 → 1.7547; 1.7543 → 1.7549 | -0.0%; -0.0% | requires_confirmation | 3.71 → 3.71; 3.71 → 3.71 |
| volume_64k_128m | 0.1720 → 0.1726; 0.1719 → 0.1721 | +0.4%; +0.1% | requires_confirmation | 3.91 → 3.91; 3.91 → 3.91 |
| csv_mixed | 0.0948 → 0.0939; 0.0940 → 0.0942 | -0.4%; +0.3% | requires_confirmation | 3.71 → 3.71; 3.71 → 3.71 |
| high_cardinality | 0.1859 → 0.1846; 0.1850 → 0.1859 | -0.0%; -0.2% | requires_confirmation | 11.27 → 11.27; 11.27 → 11.27 |
| output_format | 0.0633 → 0.0628; 0.0627 → 0.0630 | -1.1%; +0.6% | requires_confirmation | 3.71 → 3.71; 3.71 → 3.71 |

## Linux-6.17.0-1022-azure-x86_64-with-glibc2.39 (x86_64)

Before `36492979de6a222c54d4110f9c06b14b53fa85ac`; after `36492979de6a222c54d4110f9c06b14b53fa85ac`.

Source: `results.json`; compiler: `rustc 1.96.0 (ac68faa20 2026-05-25)`.

| Case | Wall before → after seconds by session | Wall change by session | Wall signal | RSS before → after MiB by session |
|---|---|---|---|---|
| sum_fields | 0.0511 → 0.0512; 0.0513 → 0.0508 | +0.3%; -0.6% | requires_confirmation | 4.54 → 4.61; 4.62 → 4.57 |
| regex_fields | 0.0123 → 0.0121; 0.0121 → 0.0121 | +0.2%; -0.2% | requires_confirmation | 5.31 → 5.29; 5.36 → 5.33 |
| array_aggregation | 0.0753 → 0.0743; 0.0759 → 0.0738 | -0.7%; -2.6% | requires_confirmation | 4.54 → 4.55; 4.56 → 4.55 |
| mixed_fields | 0.1181 → 0.1196; 0.1175 → 0.1195 | +0.5%; +1.7% | requires_confirmation | 4.61 → 4.60; 4.61 → 4.55 |
| mixed_aggregation | 0.0991 → 0.0985; 0.0989 → 0.0987 | -1.0%; -0.4% | requires_confirmation | 4.55 → 4.59; 4.54 → 4.54 |
| utf8_dynamic_boolean | 0.0156 → 0.0156; 0.0156 → 0.0156 | +0.7%; +0.3% | requires_confirmation | 5.82 → 5.74; 5.71 → 5.71 |
| utf8_match_mixed | 0.0186 → 0.0185; 0.0185 → 0.0184 | -0.4%; -1.1% | requires_confirmation | 5.84 → 5.83; 5.80 → 5.89 |
| utf8_gsub_expanding | 0.0279 → 0.0281; 0.0279 → 0.0277 | +0.3%; -0.5% | requires_confirmation | 5.84 → 5.96; 5.80 → 5.95 |
| utf8_gsub_miss | 0.0125 → 0.0125; 0.0126 → 0.0126 | +0.4%; +1.4% | requires_confirmation | 5.50 → 5.56; 5.54 → 5.50 |
| utf8_match_long | 0.0339 → 0.0350; 0.0339 → 0.0347 | +1.8%; +0.2% | requires_confirmation | 5.70 → 5.83; 5.74 → 5.76 |
| utf8_empty_regex | 0.1708 → 0.1679; 0.1696 → 0.1674 | -1.4%; -0.8% | requires_confirmation | 5.75 → 5.73; 5.76 → 5.79 |
| short_file | 0.2249 → 0.2217; 0.2270 → 0.2243 | -0.8%; -1.6% | requires_confirmation | 4.58 → 4.55; 4.56 → 4.59 |
| long_1048576_file | 0.0234 → 0.0237; 0.0237 → 0.0236 | -1.0%; -1.2% | requires_confirmation | 9.41 → 9.41; 9.46 → 9.45 |
| long_4194304_file | 0.0296 → 0.0296; 0.0291 → 0.0292 | +0.3%; -0.4% | requires_confirmation | 24.42 → 24.48; 24.44 → 24.44 |
| long_8388608_file | 0.0316 → 0.0317; 0.0320 → 0.0321 | +0.4%; +0.8% | requires_confirmation | 44.41 → 44.45; 44.39 → 44.36 |
| long_pipe | 0.0353 → 0.0350; 0.0368 → 0.0345 | +0.3%; -0.4% | requires_confirmation | 24.41 → 24.39; 24.43 → 24.39 |
| long_getline | 0.0266 → 0.0261; 0.0265 → 0.0265 | -0.1%; +0.2% | requires_confirmation | 20.48 → 20.44; 20.44 → 20.39 |
| unterminated | 0.0223 → 0.0222; 0.0222 → 0.0222 | -0.9%; +0.1% | requires_confirmation | 44.43 → 44.43; 44.40 → 44.42 |
| csv_long | 0.0693 → 0.0696; 0.0690 → 0.0696 | -0.0%; +0.6% | requires_confirmation | 10.50 → 10.45; 10.50 → 10.48 |
| paragraph_control | 0.0049 → 0.0050; 0.0049 → 0.0048 | -0.3%; -3.2% | requires_confirmation | 5.45 → 5.40; 5.38 → 5.41 |
| regex_control | 0.0052 → 0.0050; 0.0051 → 0.0050 | -3.2%; -1.4% | requires_confirmation | 5.43 → 5.43; 5.43 → 5.41 |
| volume_short_128m | 1.7854 → 1.7759; 1.7804 → 1.7624 | -0.5%; -1.3% | requires_confirmation | 4.57 → 4.59; 4.57 → 4.59 |
| volume_64k_128m | 0.1814 → 0.1810; 0.1804 → 0.1807 | -0.2%; +0.1% | requires_confirmation | 4.80 → 4.76; 4.80 → 4.77 |
| csv_mixed | 0.1178 → 0.1161; 0.1175 → 0.1170 | -1.4%; -1.0% | requires_confirmation | 4.60 → 4.54; 4.56 → 4.55 |
| high_cardinality | 0.2155 → 0.2090; 0.2211 → 0.2183 | -2.4%; -0.2% | requires_confirmation | 12.14 → 12.09; 12.19 → 12.18 |
| output_format | 0.0916 → 0.0985; 0.0894 → 0.0887 | +4.3%; -0.8% | requires_confirmation | 4.45 → 4.51; 4.50 → 4.52 |

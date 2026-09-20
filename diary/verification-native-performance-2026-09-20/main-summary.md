# Native performance observations

Each row compares paired binaries on one runner. Percentages are per-session
median paired wall-time changes; RSS values are medians, not allocation counts.
Signals are exploratory; inspect raw distributions and same-revision controls.

## macOS-15.7.9-x86_64-i386-64bit-Mach-O (x86_64)

Before `61009dff859b06e85088dd7c3548a82fad0845c7`; after `36492979de6a222c54d4110f9c06b14b53fa85ac`.

Source: `results.json`; compiler: `rustc 1.96.0 (ac68faa20 2026-05-25)`.

| Case | Wall before → after seconds by session | Wall change by session | Wall signal | RSS before → after MiB by session |
|---|---|---|---|---|
| sum_fields | 0.1676 → 0.1612; 0.1682 → 0.1602 | -2.9%; -1.2% | requires_confirmation | 2.83 → 2.85; 2.83 → 2.83 |
| regex_fields | 0.0444 → 0.0407; 0.0473 → 0.0452 | -4.2%; -2.8% | requires_confirmation | 3.35 → 3.36; 3.36 → 3.38 |
| array_aggregation | 0.2325 → 0.2264; 0.2438 → 0.2375 | -0.8%; -1.1% | requires_confirmation | 2.85 → 2.86; 2.87 → 2.89 |
| mixed_fields | 0.3434 → 0.3361; 0.3331 → 0.3601 | -1.5%; -0.3% | requires_confirmation | 2.90 → 2.87; 2.91 → 2.90 |
| mixed_aggregation | 0.2934 → 0.2625; 0.2282 → 0.2254 | +3.5%; -1.8% | requires_confirmation | 2.92 → 2.92; 2.92 → 2.93 |
| utf8_dynamic_boolean | 0.0409 → 0.0412; 0.0398 → 0.0401 | +1.0%; -0.1% | requires_confirmation | 3.43 → 3.43; 3.43 → 3.43 |
| utf8_match_mixed | 0.0452 → 0.0449; 0.0491 → 0.0464 | +1.0%; +2.6% | requires_confirmation | 3.49 → 3.55; 3.49 → 3.50 |
| utf8_gsub_expanding | 0.0725 → 0.0701; 0.0700 → 0.0707 | -10.0%; +3.7% | requires_confirmation | 3.50 → 3.52; 3.50 → 3.52 |
| utf8_gsub_miss | 0.0447 → 0.0418; 0.0410 → 0.0401 | +3.1%; -4.2% | requires_confirmation | 3.27 → 3.27; 3.26 → 3.27 |
| utf8_match_long | 0.0690 → 0.0679; 0.0616 → 0.0600 | -1.7%; -0.6% | requires_confirmation | 3.52 → 3.52; 3.52 → 3.52 |
| utf8_empty_regex | 0.4573 → 0.4723; 0.4355 → 0.4610 | +6.7%; +6.2% | requires_confirmation | 3.40 → 3.43; 3.41 → 3.44 |
| short_file | 0.9221 → 1.0128; 0.7970 → 0.8235 | +0.3%; +4.7% | requires_confirmation | 2.89 → 2.89; 2.88 → 2.88 |
| long_1048576_file | 0.4726 → 0.0556; 0.5177 → 0.0603 | -88.9%; -88.4% | repeatable_faster | 7.98 → 7.95; 7.96 → 7.95 |
| long_4194304_file | 1.6672 → 0.0713; 1.8818 → 0.0668 | -95.8%; -96.4% | repeatable_faster | 23.00 → 22.95; 22.99 → 22.95 |
| long_8388608_file | 3.5328 → 0.0912; 3.5393 → 0.0906 | -97.4%; -97.6% | repeatable_faster | 42.99 → 42.95; 43.00 → 42.95 |
| long_pipe | 1.8943 → 0.4861; 1.8489 → 0.4669 | -74.7%; -74.4% | repeatable_faster | 22.99 → 22.99; 22.99 → 22.98 |
| long_getline | 1.5733 → 0.0531; 1.5899 → 0.0511 | -96.5%; -96.6% | repeatable_faster | 19.01 → 18.97; 19.01 → 18.97 |
| unterminated | 1.5673 → 0.0681; 1.5132 → 0.0637 | -96.0%; -95.8% | repeatable_faster | 42.97 → 42.95; 42.95 → 42.95 |
| csv_long | 0.8098 → 0.1062; 0.8517 → 0.1117 | -87.1%; -86.7% | repeatable_faster | 20.91 → 18.00; 22.29 → 21.94 |
| paragraph_control | 0.0237 → 0.0239; 0.0228 → 0.0222 | -0.3%; -4.3% | requires_confirmation | 3.34 → 3.35; 3.34 → 3.35 |
| regex_control | 0.0240 → 0.0233; 0.0238 → 0.0235 | +1.1%; +0.5% | requires_confirmation | 3.33 → 3.35; 3.33 → 3.35 |
| volume_short_128m | 6.4272 → 6.2002; 5.8719 → 6.1071 | -3.1%; +0.6% | requires_confirmation | 2.89 → 2.89; 2.89 → 2.89 |
| volume_64k_128m | 0.3843 → 0.1992; 0.3652 → 0.2436 | -35.5%; -34.6% | repeatable_faster | 3.22 → 3.20; 3.20 → 3.21 |
| csv_mixed | 0.3550 → 0.3436; 0.3573 → 0.3414 | -4.2%; -8.1% | requires_confirmation | 2.88 → 2.88; 2.89 → 2.89 |
| high_cardinality | 0.5931 → 0.6179; 0.6100 → 0.6161 | -0.0%; +0.8% | requires_confirmation | 11.54 → 11.54; 11.54 → 11.54 |
| output_format | 0.2342 → 0.2393; 0.2327 → 0.2453 | +1.3%; +3.0% | requires_confirmation | 2.89 → 2.89; 2.89 → 2.88 |

## macOS-15.7.9-arm64-arm-64bit-Mach-O (arm64)

Before `61009dff859b06e85088dd7c3548a82fad0845c7`; after `36492979de6a222c54d4110f9c06b14b53fa85ac`.

Source: `results.json`; compiler: `rustc 1.96.0 (ac68faa20 2026-05-25)`.

| Case | Wall before → after seconds by session | Wall change by session | Wall signal | RSS before → after MiB by session |
|---|---|---|---|---|
| sum_fields | 0.0562 → 0.0593; 0.0585 → 0.0637 | +3.3%; +4.0% | requires_confirmation | 4.00 → 4.05; 3.92 → 3.97 |
| regex_fields | 0.0206 → 0.0209; 0.0196 → 0.0210 | -1.4%; +7.1% | requires_confirmation | 4.56 → 4.61; 4.56 → 4.61 |
| array_aggregation | 0.0882 → 0.0950; 0.0779 → 0.0791 | +9.3%; -2.7% | requires_confirmation | 3.97 → 4.02; 3.95 → 4.03 |
| mixed_fields | 0.1208 → 0.1211; 0.1228 → 0.1280 | +0.7%; +4.2% | requires_confirmation | 4.05 → 4.11; 4.05 → 4.00 |
| mixed_aggregation | 0.1126 → 0.1113; 0.0976 → 0.1033 | -4.0%; +2.3% | requires_confirmation | 4.05 → 4.11; 4.05 → 4.11 |
| utf8_dynamic_boolean | 0.0231 → 0.0226; 0.0233 → 0.0227 | -3.0%; +4.1% | requires_confirmation | 4.84 → 4.72; 4.67 → 4.72 |
| utf8_match_mixed | 0.0238 → 0.0246; 0.0276 → 0.0252 | +3.0%; -7.2% | requires_confirmation | 4.73 → 4.78; 4.92 → 4.97 |
| utf8_gsub_expanding | 0.0315 → 0.0321; 0.0376 → 0.0318 | +2.9%; -2.2% | requires_confirmation | 4.94 → 4.80; 4.75 → 4.80 |
| utf8_gsub_miss | 0.0173 → 0.0172; 0.0199 → 0.0176 | +2.6%; -7.1% | requires_confirmation | 4.52 → 4.56; 4.52 → 4.56 |
| utf8_match_long | 0.0520 → 0.0475; 0.0392 → 0.0387 | -6.6%; +0.3% | requires_confirmation | 4.77 → 4.80; 4.77 → 4.80 |
| utf8_empty_regex | 0.1531 → 0.1555; 0.1463 → 0.1450 | +0.2%; +0.8% | requires_confirmation | 4.78 → 4.73; 4.78 → 4.81 |
| short_file | 0.3258 → 0.3022; 0.3489 → 0.3511 | -3.6%; +4.7% | requires_confirmation | 4.06 → 4.09; 4.06 → 4.09 |
| long_1048576_file | 0.6473 → 0.0475; 0.5719 → 0.0380 | -92.5%; -93.3% | repeatable_faster | 15.19 → 9.31; 15.19 → 11.27 |
| long_4194304_file | 2.1482 → 0.0482; 1.9915 → 0.0440 | -97.7%; -97.8% | repeatable_faster | 36.22 → 32.20; 40.20 → 32.20 |
| long_8388608_file | 3.8225 → 0.0542; 3.6017 → 0.0469 | -98.7%; -98.6% | repeatable_faster | 60.19 → 52.23; 60.14 → 52.14 |
| long_pipe | 1.8499 → 0.1452; 1.6839 → 0.1580 | -91.9%; -91.0% | repeatable_faster | 36.14 → 36.14; 36.11 → 40.16 |
| long_getline | 1.6699 → 0.0310; 1.7301 → 0.0370 | -98.1%; -98.0% | repeatable_faster | 32.16 → 20.16; 40.20 → 28.11 |
| unterminated | 1.6469 → 0.0283; 1.6934 → 0.0261 | -98.2%; -98.4% | repeatable_faster | 52.14 → 44.03; 52.05 → 44.03 |
| csv_long | 0.9855 → 0.0537; 1.0532 → 0.0679 | -94.2%; -93.3% | repeatable_faster | 27.62 → 11.41; 26.64 → 15.47 |
| paragraph_control | 0.0125 → 0.0130; 0.0114 → 0.0120 | +3.1%; -5.4% | requires_confirmation | 4.59 → 4.61; 4.59 → 4.61 |
| regex_control | 0.0108 → 0.0106; 0.0131 → 0.0143 | +0.9%; +5.8% | requires_confirmation | 4.58 → 4.61; 4.58 → 4.61 |
| volume_short_128m | 2.5151 → 2.4619; 2.3789 → 2.3990 | -1.0%; -2.4% | requires_confirmation | 4.06 → 4.27; 4.19 → 4.33 |
| volume_64k_128m | 0.4189 → 0.1966; 0.4230 → 0.2017 | -53.4%; -49.0% | repeatable_faster | 4.92 → 4.95; 4.92 → 4.95 |
| csv_mixed | 0.1148 → 0.1085; 0.1131 → 0.1053 | -2.4%; -4.1% | requires_confirmation | 4.12 → 4.16; 4.06 → 4.11 |
| high_cardinality | 0.2451 → 0.2350; 0.2382 → 0.2440 | -4.5%; +3.5% | requires_confirmation | 13.97 → 14.00; 13.97 → 14.00 |
| output_format | 0.0820 → 0.0775; 0.0730 → 0.0788 | -10.3%; +4.6% | requires_confirmation | 4.00 → 4.08; 3.98 → 4.09 |

## Linux-6.17.0-1022-azure-aarch64-with-glibc2.39 (aarch64)

Before `61009dff859b06e85088dd7c3548a82fad0845c7`; after `36492979de6a222c54d4110f9c06b14b53fa85ac`.

Source: `results.json`; compiler: `rustc 1.96.0 (ac68faa20 2026-05-25)`.

| Case | Wall before → after seconds by session | Wall change by session | Wall signal | RSS before → after MiB by session |
|---|---|---|---|---|
| sum_fields | 0.0433 → 0.0430; 0.0435 → 0.0432 | -0.8%; -0.6% | requires_confirmation | 3.71 → 3.71; 3.71 → 3.71 |
| regex_fields | 0.0101 → 0.0100; 0.0100 → 0.0099 | -1.4%; -1.0% | requires_confirmation | 4.34 → 4.34; 4.34 → 4.34 |
| array_aggregation | 0.0627 → 0.0630; 0.0628 → 0.0626 | +0.6%; -0.1% | requires_confirmation | 3.71 → 3.71; 3.71 → 3.71 |
| mixed_fields | 0.0979 → 0.0982; 0.0979 → 0.0981 | -0.1%; +0.5% | requires_confirmation | 3.71 → 3.71; 3.71 → 3.71 |
| mixed_aggregation | 0.0792 → 0.0795; 0.0790 → 0.0795 | +0.7%; +0.7% | requires_confirmation | 3.71 → 3.71; 3.71 → 3.71 |
| utf8_dynamic_boolean | 0.0128 → 0.0127; 0.0127 → 0.0127 | -0.3%; -0.3% | requires_confirmation | 4.88 → 4.88; 4.88 → 4.88 |
| utf8_match_mixed | 0.0151 → 0.0151; 0.0151 → 0.0151 | +0.0%; -0.5% | requires_confirmation | 4.82 → 4.82; 4.82 → 4.82 |
| utf8_gsub_expanding | 0.0227 → 0.0226; 0.0228 → 0.0226 | -0.6%; -0.5% | requires_confirmation | 4.82 → 4.82; 4.82 → 4.82 |
| utf8_gsub_miss | 0.0104 → 0.0104; 0.0105 → 0.0105 | -0.4%; +0.1% | requires_confirmation | 4.70 → 4.70; 4.70 → 4.70 |
| utf8_match_long | 0.0278 → 0.0279; 0.0279 → 0.0278 | +0.7%; -0.2% | requires_confirmation | 4.82 → 4.82; 4.82 → 4.82 |
| utf8_empty_regex | 0.1420 → 0.1412; 0.1413 → 0.1412 | -0.6%; +0.0% | requires_confirmation | 4.70 → 4.70; 4.70 → 4.70 |
| short_file | 0.2222 → 0.2216; 0.2224 → 0.2216 | -0.3%; -0.2% | repeatable_faster | 3.71 → 3.71; 3.71 → 3.71 |
| long_1048576_file | 0.3544 → 0.0263; 0.3539 → 0.0261 | -92.6%; -92.6% | repeatable_faster | 8.59 → 8.58; 8.58 → 8.58 |
| long_4194304_file | 1.3499 → 0.0323; 1.3478 → 0.0322 | -97.6%; -97.6% | repeatable_faster | 23.58 → 23.58; 23.58 → 23.58 |
| long_8388608_file | 2.6936 → 0.0407; 2.6924 → 0.0408 | -98.5%; -98.5% | repeatable_faster | 43.58 → 43.58; 43.58 → 43.58 |
| long_pipe | 1.3523 → 0.0353; 1.3526 → 0.0352 | -97.4%; -97.4% | repeatable_faster | 23.58 → 23.58; 23.59 → 23.58 |
| long_getline | 1.3410 → 0.0236; 1.3407 → 0.0239 | -98.2%; -98.2% | repeatable_faster | 19.66 → 19.66; 19.66 → 19.65 |
| unterminated | 1.3553 → 0.0281; 1.3552 → 0.0284 | -97.9%; -97.9% | repeatable_faster | 43.52 → 43.52; 43.52 → 43.52 |
| csv_long | 0.7240 → 0.0456; 0.7240 → 0.0458 | -93.7%; -93.7% | repeatable_faster | 9.60 → 9.59; 9.60 → 9.59 |
| paragraph_control | 0.0050 → 0.0049; 0.0050 → 0.0050 | -2.3%; +1.1% | requires_confirmation | 4.34 → 4.34; 4.34 → 4.34 |
| regex_control | 0.0053 → 0.0053; 0.0051 → 0.0051 | -0.2%; -0.4% | requires_confirmation | 4.34 → 4.34; 4.34 → 4.34 |
| volume_short_128m | 1.7595 → 1.7543; 1.7601 → 1.7543 | -0.2%; -0.3% | repeatable_faster | 3.71 → 3.71; 3.71 → 3.71 |
| volume_64k_128m | 0.3139 → 0.1721; 0.3158 → 0.1718 | -45.1%; -45.6% | repeatable_faster | 3.91 → 3.91; 3.91 → 3.91 |
| csv_mixed | 0.0989 → 0.0950; 0.0989 → 0.0942 | -3.5%; -5.0% | repeatable_faster | 3.71 → 3.71; 3.71 → 3.71 |
| high_cardinality | 0.1832 → 0.1828; 0.1830 → 0.1837 | -0.3%; +0.6% | requires_confirmation | 11.28 → 11.28; 11.27 → 11.27 |
| output_format | 0.0619 → 0.0619; 0.0619 → 0.0620 | +0.4%; +1.3% | requires_confirmation | 3.71 → 3.71; 3.71 → 3.71 |

## Linux-6.17.0-1022-azure-x86_64-with-glibc2.39 (x86_64)

Before `61009dff859b06e85088dd7c3548a82fad0845c7`; after `36492979de6a222c54d4110f9c06b14b53fa85ac`.

Source: `results.json`; compiler: `rustc 1.96.0 (ac68faa20 2026-05-25)`.

| Case | Wall before → after seconds by session | Wall change by session | Wall signal | RSS before → after MiB by session |
|---|---|---|---|---|
| sum_fields | 0.0510 → 0.0515; 0.0509 → 0.0515 | +0.3%; +0.5% | requires_confirmation | 4.56 → 4.54; 4.54 → 4.61 |
| regex_fields | 0.0118 → 0.0121; 0.0118 → 0.0121 | +2.2%; +1.7% | repeatable_slower | 5.30 → 5.37; 5.28 → 5.39 |
| array_aggregation | 0.0746 → 0.0752; 0.0741 → 0.0748 | +2.0%; +1.2% | requires_confirmation | 4.55 → 4.56; 4.58 → 4.56 |
| mixed_fields | 0.1171 → 0.1197; 0.1182 → 0.1194 | +1.8%; +1.0% | requires_confirmation | 4.57 → 4.54; 4.60 → 4.58 |
| mixed_aggregation | 0.0984 → 0.0994; 0.0978 → 0.0992 | +1.3%; +1.3% | requires_confirmation | 4.54 → 4.55; 4.60 → 4.55 |
| utf8_dynamic_boolean | 0.0152 → 0.0154; 0.0153 → 0.0156 | +0.5%; +1.2% | repeatable_slower | 5.71 → 5.74; 5.66 → 5.72 |
| utf8_match_mixed | 0.0180 → 0.0181; 0.0180 → 0.0181 | +0.5%; +0.7% | requires_confirmation | 5.83 → 5.82; 5.81 → 5.78 |
| utf8_gsub_expanding | 0.0273 → 0.0273; 0.0273 → 0.0274 | +0.3%; +0.6% | requires_confirmation | 5.83 → 5.87; 5.88 → 5.86 |
| utf8_gsub_miss | 0.0124 → 0.0123; 0.0123 → 0.0122 | -1.4%; -1.4% | requires_confirmation | 5.56 → 5.57; 5.54 → 5.52 |
| utf8_match_long | 0.0336 → 0.0344; 0.0341 → 0.0339 | +2.6%; -2.1% | requires_confirmation | 5.79 → 5.77; 5.77 → 5.78 |
| utf8_empty_regex | 0.1700 → 0.1737; 0.1697 → 0.1731 | +2.5%; +1.8% | requires_confirmation | 5.67 → 5.71; 5.67 → 5.74 |
| short_file | 0.2255 → 0.2348; 0.2233 → 0.2350 | +4.4%; +5.5% | repeatable_slower | 4.57 → 4.52; 4.54 → 4.57 |
| long_1048576_file | 0.3618 → 0.0224; 0.3614 → 0.0226 | -93.8%; -93.7% | repeatable_faster | 9.39 → 9.47; 9.46 → 9.47 |
| long_4194304_file | 1.3743 → 0.0269; 1.3758 → 0.0272 | -98.0%; -98.0% | repeatable_faster | 24.37 → 24.41; 24.44 → 24.40 |
| long_8388608_file | 2.7185 → 0.0304; 2.7190 → 0.0307 | -98.9%; -98.9% | repeatable_faster | 44.44 → 44.40; 44.37 → 44.34 |
| long_pipe | 1.4074 → 0.0362; 1.3911 → 0.0326 | -97.4%; -97.6% | repeatable_faster | 24.37 → 24.32; 24.37 → 24.45 |
| long_getline | 1.3710 → 0.0242; 1.3718 → 0.0241 | -98.2%; -98.2% | repeatable_faster | 20.42 → 20.38; 20.39 → 20.45 |
| unterminated | 1.3664 → 0.0213; 1.3675 → 0.0213 | -98.4%; -98.4% | repeatable_faster | 44.46 → 44.41; 44.36 → 44.39 |
| csv_long | 0.7113 → 0.0675; 0.7105 → 0.0676 | -90.5%; -90.6% | repeatable_faster | 10.48 → 10.52; 10.41 → 10.48 |
| paragraph_control | 0.0043 → 0.0043; 0.0042 → 0.0044 | +0.9%; +1.8% | requires_confirmation | 5.38 → 5.38; 5.36 → 5.46 |
| regex_control | 0.0044 → 0.0045; 0.0045 → 0.0045 | +2.0%; +0.4% | requires_confirmation | 5.40 → 5.46; 5.38 → 5.41 |
| volume_short_128m | 1.7627 → 1.8599; 1.7664 → 1.8468 | +5.5%; +4.1% | repeatable_slower | 4.58 → 4.54; 4.56 → 4.56 |
| volume_64k_128m | 0.3249 → 0.1812; 0.3266 → 0.1806 | -44.3%; -44.6% | repeatable_faster | 4.83 → 4.76; 4.76 → 4.79 |
| csv_mixed | 0.1210 → 0.1172; 0.1202 → 0.1172 | -2.8%; -2.3% | requires_confirmation | 4.55 → 4.57; 4.59 → 4.56 |
| high_cardinality | 0.1985 → 0.2036; 0.2005 → 0.2024 | +2.1%; +1.4% | requires_confirmation | 12.19 → 12.15; 12.09 → 12.14 |
| output_format | 0.0877 → 0.0870; 0.0867 → 0.0871 | -0.2%; +0.4% | requires_confirmation | 4.45 → 4.52; 4.46 → 4.54 |

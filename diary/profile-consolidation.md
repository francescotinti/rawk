# Profiling prima delle ottimizzazioni — 19 settembre 2026

macOS ARM64, build release Rust 1.96.0, `LC_ALL=C`. Campionamento nativo
con `/usr/bin/sample PID 3 1 -file /tmp/rawk-profile-before.txt` durante
`rawk '{s += $2} END{print s}'`, stdin di 6.000.000 righe `1 2 3`.
Il programma termina con status 0 e stdout `12000000\n`.

Lo stack di `parse_number` include allocazione e liberazione della stringa
prodotta da `to_ascii_lowercase`; `RecordReader::next` include copie del buffer
residuo dopo ogni record. Le due modifiche candidate rimuovono tali operazioni.
I campioni sono un'indicazione dei punti caldi, non percentuali esatte di CPU;
il profilo include anche l'avvio del processo. L'effetto viene misurato
separatamente con benchmark intercalati, senza profiler.

```text
Sort by top of stack, same collapsed (when >= 5):
        _dyld_start  (in dyld)        314
        _xzm_free  (in libsystem_malloc.dylib)        312
        _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib)        256
        mach_absolute_time  (in libsystem_kernel.dylib)        236
        _platform_memmove  (in libsystem_platform.dylib)        160
        <deduplicated_symbol>  (in libsystem_malloc.dylib)        105
        _malloc_zone_malloc  (in libsystem_malloc.dylib)        73
        _free  (in libsystem_malloc.dylib)        70
        _xzm_xzone_malloc  (in libsystem_malloc.dylib)        61
        rawk::types::parse_number::hec207c1f05eb0666  (in rawk-before-consolidation-opt)        45
        _$LT$core..hash..sip..Hasher$LT$S$GT$$u20$as$u20$core..hash..Hasher$GT$::write::hff6e402093bd0dd6  (in rawk-before-consolidation-opt)        42
        rawk::types::EvalContext::get_var::hec34d5cb3e60a58d  (in rawk-before-consolidation-opt)        41
        mach_msg2_trap  (in dyld)        38
        _platform_memcmp  (in libsystem_platform.dylib)        35
        rawk::runner::eval_expr::hee825a65f154be15  (in rawk-before-consolidation-opt)        35
        core::hash::BuildHasher::hash_one::h64aff262e0551547  (in rawk-before-consolidation-opt)        33
        _$LT$alloc..vec..Vec$LT$T$GT$$u20$as$u20$alloc..vec..spec_from_iter_nested..SpecFromIterNested$LT$T$C$I$GT$$GT$::from_iter::h2b8f8fb6f8ac1aa5  (in rawk-before-consolidation-opt)        30
        DYLD-STUB$$free  (in rawk-before-consolidation-opt)        27
        core::hash::BuildHasher::hash_one::h584435dd479feb3f  (in rawk-before-consolidation-opt)        26
        _RNvXs2_NtNtCs6sq8b9ugfBC_4core3num11float_parsedNtNtNtB9_3str6traits7FromStr8from_str  (in rawk-before-consolidation-opt)        25
        rawk::runner::run::h62f245732df94f15  (in rawk-before-consolidation-opt)        25
        _RNvCsGIExRX8pES_7___rustc11___rdl_alloc  (in rawk-before-consolidation-opt)        23
        _RNvNtNtNtNtCs6sq8b9ugfBC_4core3num3imp7dec2flt5parse12parse_number  (in rawk-before-consolidation-opt)        23
        rawk::runner::read_main::h6175cdef86f42240  (in rawk-before-consolidation-opt)        21
        rawk::types::EvalContext::update_record::h6fa02ad961960783  (in rawk-before-consolidation-opt)        21
        rawk::input::RecordReader::next::ha78c55dad6f62941  (in rawk-before-consolidation-opt)        19
        rawk::types::EvalContext::set_var::h0c8610bc208fe1a3  (in rawk-before-consolidation-opt)        19
        rawk::runner::execute_action_inner::h5955a0b3fafb0a5e  (in rawk-before-consolidation-opt)        18
        hashbrown::map::HashMap$LT$K$C$V$C$S$C$A$GT$::insert::hf95c882c96c1b954  (in rawk-before-consolidation-opt)        16
        _RNvNtNtCs6sq8b9ugfBC_4core3str8converts9from_utf8  (in rawk-before-consolidation-opt)        14
        _$LT$alloc..vec..into_iter..IntoIter$LT$T$C$A$GT$$u20$as$u20$core..iter..traits..iterator..Iterator$GT$::fold::h7b0bcf421018502a  (in rawk-before-consolidation-opt)        13
        _RNvCsGIExRX8pES_7___rustc35___rust_no_alloc_shim_is_unstable_v2  (in rawk-before-consolidation-opt)        12
        rawk::types::EvalContext::array::habe47a1b55bfa6e6  (in rawk-before-consolidation-opt)        11
        xzm_malloc_zone_malloc_type_malloc  (in libsystem_malloc.dylib)        11
        xzm_malloc_zone_try_free_default  (in libsystem_malloc.dylib)        11
        DYLD-STUB$$malloc  (in rawk-before-consolidation-opt)        9
        rawk::runner::target::h43a075b7836da65b  (in rawk-before-consolidation-opt)        9
        rawk::types::EvalContext::get_field::h4a5bfb909101ae27  (in rawk-before-consolidation-opt)        9
        DYLD-STUB$$mach_absolute_time  (in libsystem_malloc.dylib)        8
        __bzero  (in libsystem_platform.dylib)        8
        rawk::types::AwkValue::as_number::hd023e33d72f1b30d  (in rawk-before-consolidation-opt)        8
        read  (in libsystem_kernel.dylib)        8
        DYLD-STUB$$memcpy  (in rawk-before-consolidation-opt)        7
        _RNvCsGIExRX8pES_7___rustc14___rust_dealloc  (in rawk-before-consolidation-opt)        7
        free  (in libsystem_malloc.dylib)        7
        malloc  (in libsystem_malloc.dylib)        7
        _RNvCsGIExRX8pES_7___rustc13___rdl_dealloc  (in rawk-before-consolidation-opt)        6
        rawk::types::AwkValue::as_string_convfmt::h6d00696ac7f08d6d  (in rawk-before-consolidation-opt)        6
```

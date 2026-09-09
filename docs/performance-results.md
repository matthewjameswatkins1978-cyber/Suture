# 1.9.0 performance evidence

These are local Windows x86-64 smoke measurements from Rust `1.98.1`
(`LLVM` as bundled by that toolchain). Each binary ran the built-in `tough`
profile, which warms and repeats the cases; the values are the provider's
reported average microseconds. Every run reported `wrong_applied: 0`.

| Build | Size | tiny | config 30k | text 1m | text 5m | text 32m | many lines | long-line | 250 small files |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| portable release | 34,688,512 | 389.956 | 699.410 | 7,481.520 | 37,073.540 | 240,496.800 | 27,590.690 | 22,122.920 | 3,977.808 |
| portable maxperf | 33,084,416 | 475.708 | 711.678 | 7,947.524 | 40,352.530 | 258,230.033 | 30,982.170 | 23,185.860 | 1,246.616 |
| modern x86-64-v3 | 33,177,088 | 409.793 | 675.416 | 7,956.200 | 32,575.700 | 213,768.200 | 28,438.770 | 21,324.540 | 1,208.274 |
| native local | 33,056,768 | 411.670 | 702.258 | 6,535.620 | 32,555.510 | 210,113.667 | 24,886.050 | 20,269.060 | 1,292.970 |

The portable 1.8.1 executable previously installed on this PC was
27,543,552 bytes. The 1.9.0 portable binary is larger because of the four
admitted Tree-sitter grammars, YAML grammar and the coverage registry.

Settings tested here are the existing release profile (`opt-level` and thin
LTO), the named `maxperf` profile (`opt-level=3`, fat LTO, one codegen unit,
non-incremental, stripped symbols), `maxperf` plus `-C target-cpu=x86-64-v3`,
and `maxperf` plus `-C target-cpu=native`. This is evidence for the build
flavours and correctness smoke, not a claim that one configuration wins every
workload. The full opt-level 2/3 and thin/fat configuration matrix and a
separate PGO training/validation run remain follow-up work before declaring a
performance winner or shipping PGO.

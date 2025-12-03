# bench baseline

Search Default Depth 6  time:   [61.854 ms 63.164 ms 64.712 ms]
Search Default Depth 7  time:   [109.54 ms 110.61 ms 112.92 ms]
Search Default Depth 8  time:   [371.94 ms 376.61 ms 381.97 ms]

Search Kiwipete Depth 6 time:   [615.22 ms 617.86 ms 620.86 ms]
Search Kiwipete Depth 7 time:   [2.3331 s 2.3481 s 2.3640 s]
Search Kiwipete Depth 8 time:   [5.0647 s 5.1400 s 5.2154 s]

## nnue eval

Search Default Depth 6  time:   [58.999 ms 59.849 ms 60.817 ms]
Search Default Depth 7  time:   [139.01 ms 141.94 ms 146.10 ms]
Search Default Depth 8  time:   [319.97 ms 325.73 ms 331.11 ms]

Search Kiwipete Depth 6 time:   [354.00 ms 363.42 ms 374.98 ms]
Search Kiwipete Depth 7 time:   [869.75 ms 887.71 ms 908.98 ms]
Search Kiwipete Depth 8 time:   [2.0844 s 2.1314 s 2.1948 s]

### matches

0-10
1-9-0
10-20
3-6-1
20-30
2-7-1
flipped 6-4-0

## Vectorize update accumulator refresh cache

Search Default Depth 6  time:   [58.687 ms 59.166 ms 59.604 ms]
Search Default Depth 7  time:   [145.05 ms 146.50 ms 147.93 ms]
Search Default Depth 8  time:   [327.24 ms 331.82 ms 337.04 ms]

Search Kiwipete Depth 6 time:   [324.79 ms 329.98 ms 335.66 ms]
Search Kiwipete Depth 7 time:   [804.69 ms 812.26 ms 821.26 ms]
Search Kiwipete Depth 8 time:   [2.0026 s 2.0126 s 2.0227 s]

## Vectorize update incremental

Search Default Depth 6  time:   [50.766 ms 51.534 ms 52.325 ms]
Search Default Depth 7  time:   [126.25 ms 132.19 ms 139.51 ms]
Search Default Depth 8  time:   [297.57 ms 302.31 ms 307.08 ms]

Search Kiwipete Depth 6 time:   [284.23 ms 290.54 ms 296.93 ms]
Search Kiwipete Depth 7 time:   [734.06 ms 756.57 ms 778.34 ms]
Search Kiwipete Depth 8 time:   [1.8021 s 1.8384 s 1.8749 s]

## test what happens without time_up() check

Search Default Depth 6  time:   [49.169 ms 49.363 ms 49.646 ms]
Search Default Depth 7  time:   [128.51 ms 128.80 ms 129.11 ms]
Search Default Depth 8  time:   [270.48 ms 277.15 ms 283.94 ms]

Search Kiwipete Depth 6 time:   [295.61 ms 303.60 ms 311.78 ms]
Search Kiwipete Depth 7 time:   [685.97 ms 703.07 ms 728.28 ms]
Search Kiwipete Depth 8 time:   [1.7054 s 1.7257 s 1.7507 s]

## LMR + NULL + FUTILITY More aggressive

Search Default Depth 6  time:   [40.250 ms 41.369 ms 42.905 ms]
Search Default Depth 7  time:   [97.100 ms 99.122 ms 100.45 ms]
Search Default Depth 8  time:   [205.79 ms 207.91 ms 210.24 ms]

Search Kiwipete Depth 6 time:   [165.68 ms 168.02 ms 170.82 ms]
Search Kiwipete Depth 7 time:   [411.16 ms 425.97 ms 446.03 ms]
Search Kiwipete Depth 8 time:   [1.0369 s 1.0476 s 1.0633 s]

## match results
# Work to Do

## Move Optimization

- Implement better move ordering
- MVV/LVA **2**
- Killer Moves **3**
- Move extensions when move results in check
- Queiesence Search I think is in? How about tt
- Optimal mate length

- Won't take the damn rook in this position after rook g1 3k2r1/3np3/4Q1B1/3N4/1P1P1P2/4P2P/3B4/4K2R w K - 1 41
- Refine aspiration based on position complexity?

## Evaluation Optimization

- Compute all shared resources once, enemy pawns my pawns ..
- Evaluate initiative
- Improve position evaluation **1**

## Gameplay Needs

- Verify En Passant Computer works
- Check stalemate avoidance
- Promoting to knights?? Pawn pushing instead of mate. Seems to be minimizing when mate occurs

## Performance History

1. bcf049a3048e48aa3849b04c4da7916c756a8ed5 INITIAL Version transition to PLECO

    - Engine Evaluations/Full  Evaluation (Sample 50, Warmup 20)
        **time:   [705.07 ns 705.50 ns 706.01 ns]**
        change: [+0.8278% +1.1403% +1.3906%] (p = 0.00 < 0.05)

    (Sample 10, Warmup 150)
    - Search Default Depth 5  
        **time:   [258.31 ms 260.00 ms 261.04 ms]**
        change: [-17.784% -17.585% -17.376%] (p = 0.00 < 0.05)

    - Search Default Depth 6  
        **time:   [2.7154 s 2.7167 s 2.7185 s]**
        change: [-20.491% -20.312% -20.137%] (p = 0.00 < 0.05)

    - Search Kiwipete Depth 5
        **time:   [1.1140 s 1.1176 s 1.1210 s]**

    <!-- - Search Kiwipete Depth 6
        **time:   [1.1400 s 1.1456 s 1.1572 s]** -->

2. 5478720ee835874d07454db9791cd779331300dd Add move ordering

    - Search Default Depth 5
        **time:   [314.49 ms 314.68 ms 314.89 ms]**
    - Search Default Depth 6  
        **time:   [3.0018 s 3.0066 s 3.0134 s]**
    - Search Kiwipete Depth 5
        **time:   [356.81 ms 358.15 ms 360.90 ms]**
    - Search Kiwipete Depth 6
        **time:   [1.3655 s 1.3661 s 1.3671 s]**

3. f89c312be1051e8710cae487477e308d7d897435 Add transposition table

    - Search Default Depth 5
        **time:   [393.80 ms 396.14 ms 397.81 ms]**
        change: [+25.154% +25.492% +25.927%] (p = 0.00 < 0.05)

    - Search Default Depth 6
        **time:   [2.1285 s 2.1404 s 2.1493 s]**
        change: [-29.567% -29.230% -28.860%] (p = 0.00 < 0.05)

    - Search Kiwipete Depth 5
        **time:   [328.41 ms 328.78 ms 329.54 ms]**
        change: [-9.6647% -8.8777% -8.2030%] (p = 0.00 < 0.05)

    - Search Kiwipete Depth 6
        **time:   [1.1320 s 1.1342 s 1.1356 s]**
        change: [-17.338% -17.197% -17.048%] (p = 0.00 < 0.05)

4. 77687a1bfd2cbec35b7a805907b7d46215a4409e Add aspiration window (20 but dynamic correction after depth 3)

    - Search Default Depth 5
        **time:   [343.09 ms 345.62 ms 349.63 ms]**
        change: [-5.0490% -4.7326% -4.4691%] (p = 0.00 < 0.05)

    - Search Default Depth 6
        **time:   [308.47 ms 311.19 ms 314.07 ms]**
        change: [-84.227% -84.156% -84.090%] (p = 0.00 < 0.05)

    - Search Kiwipete Depth 5
        **time:   [196.71 ms 197.09 ms 197.59 ms]**
        change: [-33.032% -32.873% -32.738%] (p = 0.00 < 0.05)

    - Search Kiwipete Depth 6
        **time:   [502.18 ms 502.67 ms 503.13 ms]**
        change: [-57.749% -57.631% -57.472%] (p = 0.00 < 0.05)

5. TBD. Add quiescence search, aspiration windows were broken without quiescence. Add tt best move mv prioritization. Effective Rating 1450.

    - Engine Evaluations/Full Evaluation
        **time:   [707.22 ns 709.75 ns 714.80 ns]**
    - Search Default Depth 5
        **time:   [44.374 ms 44.452 ms 44.513 ms]**
    - Search Default Depth 6
        **time:   [493.99 ms 498.20 ms 502.93 ms]**
    - Search Default Depth 7
        **time:   [2.1191 s 2.1206 s 2.1226 s]**

    - Search Kiwipete Depth 5
        **time:   [149.23 ms 149.78 ms 151.05 ms]**
    - Search Kiwipete Depth 6
        **time:   [247.04 ms 249.20 ms 250.93 ms]**
    - Search Kiwipete Depth 7
        **time:   [927.48 ms 927.81 ms 928.17 ms]**

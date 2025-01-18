# Work to Do

## Move Optimization

1. Move extensions when depth == 0 and the move leads to check
2. Queiesence Search
3. Transposition Table, prefetch?
4. Optimal mate length
5. Aspiration Windows

## Evaluation Optimization

1. Compute all shared resources once, enemy pawns my pawns ..
2. Evaluate initiative

## Gameplay Needs

1. Fix castling **DONE**
2. Verify En Passant Computer works
3. Check stalemate avoidance
4. Promoting to knights?? Pawn pushing instead of mate. Seems to be minimizing when mate occurs

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

2. Add move ordering

    - Search Default Depth 5
        **time:   [314.49 ms 314.68 ms 314.89 ms]**
    - Search Default Depth 6  
        **time:   [3.0018 s 3.0066 s 3.0134 s]**
    - Search Kiwipete Depth 5
        **time:   [356.81 ms 358.15 ms 360.90 ms]**
    - Search Kiwipete Depth 6
        **time:   [1.3655 s 1.3661 s 1.3671 s]**

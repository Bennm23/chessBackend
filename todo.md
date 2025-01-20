# Work to Do

## Move Optimization

- Go back to the basics. Remove all the nonesense I have for evaluation, revert random search changes.
    Add module for evaluation, in mod rs have evaluate. then have different files for different kinds
- Commit after ^ clean up chess directory
- Add timer for search
- Add root move search. After each depth, search the moves sorted by previous depth order
- Add killer moves

---------------------

- Implement better move ordering
- Killer Moves **2**
- Move extensions when move results in check
- Queiesence Search I think is in? How about tt
- Optimal mate length

- Won't take the damn rook in this position after rook g1 3k2r1/3np3/4Q1B1/3N4/1P1P1P2/4P2P/3B4/4K2R w K - 1 41
- Refine aspiration based on position complexity?

- Add Debugger to Search, print out each depth best move and best move sequence
- Add search root function that remembers the best order of root moves as the search deepens, then uses
    the previous sort order eval

## Evaluation Optimization

- Compute all shared resources once, enemy pawns my pawns ..
- Evaluate initiative
- Improve position evaluation **1**
- Encourage pawn promotion more ? Thought I was but sometimes no luck. Promotion very akward

## Gameplay Needs

- Verify En Passant Computer works
- add turn timer **1**
- Check stalemate avoidance
- Promoting to knights?? Pawn pushing instead of mate. Seems to be minimizing when mate occurs
- DRAW AVOIDANCE WITH THIS POSITION
    2r3k1/4n2p/Q3R1p1/8/3BB3/5P2/P1Pq1KPP/7R w - - 5 33
- Need half move count? Or something threefold repetition

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
    - Search Default Depth 6
        **time:   [2.1285 s 2.1404 s 2.1493 s]**
    - Search Kiwipete Depth 5
        **time:   [328.41 ms 328.78 ms 329.54 ms]**
    - Search Kiwipete Depth 6
        **time:   [1.1320 s 1.1342 s 1.1356 s]**

4. 77687a1bfd2cbec35b7a805907b7d46215a4409e Add aspiration window (20 but dynamic correction after depth 3)

    - Search Default Depth 5
        **time:   [343.09 ms 345.62 ms 349.63 ms]**
    - Search Default Depth 6
        **time:   [308.47 ms 311.19 ms 314.07 ms]**
    - Search Kiwipete Depth 5
        **time:   [196.71 ms 197.09 ms 197.59 ms]**
    - Search Kiwipete Depth 6
        **time:   [502.18 ms 502.67 ms 503.13 ms]**

5. a027286c9c1c8c3396eb5a8a9e5282e8449ab707 Add quiescence search, aspiration windows were broken without quiescence. Add tt best move mv prioritization. Effective Rating 1450.

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

6. 2f31660d2cf541e7a9dee721b5fa4002953e686a MVV_LVA Ordering. Beat Jonas (1700) Effective Rating 1800. Added some pawn late game logic to encourage promotion
    - Search Default Depth 5
        **time:   [44.409 ms 44.521 ms 44.667 ms]**
    - Search Default Depth 6
        **time:   [494.33 ms 497.49 ms 501.00 ms]**
    - Search Default Depth 7
        **time:   [2.1064 s 2.1129 s 2.1194 s]**
    - Search Kiwipete Depth 5
        **time:   [93.471 ms 94.087 ms 94.953 ms]**
    - Search Kiwipete Depth 6
        **time:   [181.08 ms 181.25 ms 181.42 ms]**
    - Search Kiwipete Depth 7
        **time:   [604.72 ms 606.83 ms 608.36 ms]**
    - Search Kiwipete Depth 8
        **time:   [1.7377 s 1.7434 s 1.7507 s]**

7. 169c4ffa4a2007cf3aed356cc87c01638d1388e3 Expand MVV_LVA Ordering to quiescence search. Little/Worse impact for start board but decent gains in kiwipete
    - Search Default Depth 5
        **time:   [45.962 ms 46.031 ms 46.105 ms]**
    - Search Default Depth 6
        **time:   [505.16 ms 506.04 ms 506.91 ms]**
    - Search Default Depth 7
        **time:   [2.0937 s 2.0963 s 2.0996 s]**
    - Search Kiwipete Depth 5
        **time:   [82.563 ms 82.635 ms 82.755 ms]**
    - Search Kiwipete Depth 6
        **time:   [154.24 ms 154.49 ms 154.73 ms]**
    - Search Kiwipete Depth 7
        **time:   [553.88 ms 554.17 ms 554.38 ms]**
    - Search Kiwipete Depth 8
        **time:   [1.6307 s 1.6405 s 1.6514 s]**

8. Add searcher class with pawn table and material for eval

Search Default Depth 5  time:   [194.67 ms 196.80 ms 201.48 ms]

Search Default Depth 6  time:   [391.06 ms 394.06 ms 399.22 ms]
Found 2 outliers among 10 measurements (20.00%)
  1 (10.00%) high mild
  1 (10.00%) high severe

Search Default Depth 7  time:   [2.7177 s 2.7260 s 2.7323 s]
Found 1 outliers among 10 measurements (10.00%)
  1 (10.00%) high mild

Search Kiwipete Depth 5 time:   [76.831 ms 77.110 ms 77.438 ms]
Found 1 outliers among 10 measurements (10.00%)
  1 (10.00%) high mild

Search Kiwipete Depth 6 time:   [210.15 ms 210.37 ms 210.53 ms]
Found 1 outliers among 10 measurements (10.00%)
  1 (10.00%) low mild

Search Kiwipete Depth 7 time:   [524.57 ms 525.99 ms 526.61 ms]

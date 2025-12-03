# Work to Do

- implement layer sizes with traits?
- cleanup nnue and add docs
- implement layer sizes with traits?
- cleanup nnue and add docs
- nnue evaluation
- fmt files
- shuffled into draw when leading <https://www.chess.com/analysis/game/computer/479315123/analysis?move=84>
- failed to avoid draw when leading <https://www.chess.com/analysis/game/computer/479322881/analysis?move=70>
- weird promo to knight <https://www.chess.com/analysis/game/computer/479344011/analysis?move=117>
- Blundered mate 1k1rr3/pp3p1Q/5q2/P7/4n1B1/1P1p3P/3P1PP1/1R3K1R w - - 2 25
    <https://www.chess.com/analysis/game/computer/461571475/review?move=47&move=47&tab=review&classification=greatfind&autorun=true>
- bench nnue eval

- put test ops behind a config flag or something *MOVE IT TO BIN*
- avoid draw in this position 3Q4/8/6p1/1p3qk1/p3p3/P5P1/6P1/6KR b - - 11 50

- add dev display for frontend. Add customizable game settings mod
- test/bench
- parallelize?
- PGO optimization for compilation

- delete pleco eval, cleanup everything. Run new engine versus those old scenarios

## Move Optimization

- why does sorting the root take so much longer?
- expand quiescence and use tt for it?, investigate cap at 5

- Add search root function that remembers the best order of root moves as the search deepens, then uses
    the previous sort order eval

## Gameplay Needs

- Need half move count? Or something threefold repetition maybe not accurate?

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

8. New Searcher Class. Beat isla bot, effective rating 1650

    Engine Evaluations/New Full Evaluation
        time:   [1.7525 us 1.7541 us 1.7557 us]
    Search Default Depth 5
        time:   [136.34 ms 136.93 ms 137.37 ms]
    Search Default Depth 6
        time:   [683.09 ms 685.17 ms 686.93 ms]
    Search Default Depth 7
        time:   [3.0728 s 3.0790 s 3.0858 s]
    Search Kiwipete Depth 5
        time:   [112.99 ms 113.19 ms 113.43 ms]
    Search Kiwipete Depth 6
        time:   [211.40 ms 212.75 ms 213.69 ms]
    Search Kiwipete Depth 7
        time:   [686.32 ms 689.40 ms 692.36 ms]

9. Add search timer. Beat lorenzo bot, effective rating 1800
    Same times as above

10. add root move ordering

    Search Default Depth 5  time:   [189.32 ms 189.47 ms 189.66 ms]
    Search Default Depth 6  time:   [750.00 ms 753.42 ms 758.06 ms]
    Search Default Depth 7  time:   [4.8521 s 4.8616 s 4.8675 s]
    Search Kiwipete Depth 5 time:   [209.98 ms 210.54 ms 211.62 ms]
    Search Kiwipete Depth 6 time:   [346.70 ms 347.32 ms 348.53 ms]
    Search Kiwipete Depth 7 time:   [1.0153 s 1.0188 s 1.0216 s]

11. TBD: Remove root move ordering, change to soft cutoff prune(don't think it changed anything) And added evaluation tests. **Beat Fatima Bot (2000) with effective rating 2250**

    - Engine Evaluations/New Full Evaluation
        **time:   [1.8559 us 1.8789 us 1.9116 us]**
    - Search Default Depth 5
        **time:   [136.28 ms 136.39 ms 136.49 ms]**
    - Search Default Depth 6
        **time:   [672.12 ms 673.80 ms 675.60 ms]**
    - Search Default Depth 7
        **time:   [3.0204 s 3.0232 s 3.0270 s]**
    - Search Kiwipete Depth 5
        **time:   [116.35 ms 117.30 ms 118.48 ms]**
    - Search Kiwipete Depth 6
        **time:   [187.42 ms 187.66 ms 188.17 ms]**
    - Search Kiwipete Depth 7
        **time:   [626.75 ms 628.90 ms 633.65 ms]**
    - Search Kiwipete Depth 8
        **time:   [2.2806 s 2.2901 s 2.2994 s]**

12. 833e44bb318f2606503323b51cadb36fe46cc3d6 Beat NORA effective rating 2250. Added some pawn structure benefits, switched to PLECO and fixed black king error
    Engine Evaluations/New Full Evaluation
        **time:   [2.2614 us 2.3174 us 2.3782 us]**
    Search Default Depth 5
        **time:   [138.49 ms 151.20 ms 161.06 ms]**
    Search Default Depth 6
        **time:   [594.40 ms 600.33 ms 613.24 ms]**
    Search Default Depth 7
        **time:   [1.9842 s 1.9870 s 1.9897 s]**
    Search Kiwipete Depth 5
        **time:   [135.29 ms 135.79 ms 136.44 ms]**
    Search Kiwipete Depth 6
        **time:   [194.24 ms 194.59 ms 195.00 ms]**
    Search Kiwipete Depth 7
        **time:   [738.13 ms 739.49 ms 740.91 ms]**

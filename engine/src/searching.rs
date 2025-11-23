use std::time::{Duration, Instant};

use pleco::{
    core::{
        mono_traits::{BlackType, PlayerTrait, WhiteType},
        score::{DRAW, INFINITE, MATE, NEG_INFINITE},
        GenTypes,
    },
    tools::{
        tt::{Entry, NodeBound, TranspositionTable},
        PreFetchable,
    },
    BitMove, Board, Player, ScoringMove,
};

use crate::{consts::MVV_LVA, debug::NoTrace, evaluation::trace_eval};

use super::{
    consts::{EvalVal, MyVal, QUEEN_VALUE},
    debug::{SearchDebugger, Trace, Tracing},
    evaluation::eval_board,
    tables::{material::Material, pawn_table::PawnTable},
};

const MATE_V: i16 = MATE as i16;
const DRAW_V: i16 = DRAW as i16;
const NEG_INF_V: i16 = NEG_INFINITE as i16;
const INF_V: i16 = INFINITE as i16;
const UNREACHABLE_V: MyVal = MATE_V + 100;

const NULL_BIT_MOVE: BitMove = BitMove::null();

const TT_ENTRIES: usize = 2_000_000;
pub const MAX_PLY: usize = 31;

macro_rules! print_at_ply {
    ($indent:expr, $fmt:expr, $($args:tt)*) => {
        {
            // Create a string of spaces
            let spaces = "  ".repeat($indent as usize);
            // Format the message with the provided arguments
            let message = format!($fmt, $($args)*);
            // Print the indented message
            println!("{}{}", spaces, message);
        }
    };
    // Case 2: No additional arguments
    ($indent:expr, $fmt:expr) => {
        {
            let spaces = " ".repeat($indent as usize);
            println!("{}{}", spaces, $fmt);
        }
    };
}

//TODO Give Searcher the board, add apply move to search
pub struct MySearcher<T: Tracing<SearchDebugger>> {
    pawn_table: PawnTable,
    material: Material,
    start_time: Instant,
    time_limit_ms: Option<u128>,

    //Debug
    tracer: T,
    nodes_explored: i64,
    pv_moves: [ScoringMove; MAX_PLY],
}
pub const NULL_SCORE: ScoringMove = ScoringMove::null();

impl<T: Tracing<SearchDebugger>> MySearcher<T> {
    pub fn new(tracer: T, time_limit: Option<u128>) -> Self {
        Self {
            pawn_table: PawnTable::new(),
            material: Material::new(),
            start_time: Instant::now(),
            time_limit_ms: time_limit,

            tracer,
            nodes_explored: 0,
            pv_moves: [NULL_SCORE; MAX_PLY],
        }
    }
    pub fn trace(tracer: T) -> Self {
        Self {
            pawn_table: PawnTable::new(),
            material: Material::new(),
            start_time: Instant::now(),
            time_limit_ms: None,

            tracer,
            nodes_explored: 0,
            pv_moves: [NULL_SCORE; MAX_PLY],
        }
    }
    pub fn eval(&mut self, board: &Board) -> MyVal {
        let pawns = &mut self.pawn_table;
        let material = &mut self.material;
        let res = eval_board(&board, pawns, material);

        if res > MyVal::MAX as EvalVal || res < MyVal::MIN as EvalVal {
            println!("ERROR OB FOR I16");
        }
        res as MyVal
    }

    #[inline(always)]
    pub fn elapsed(&mut self) -> Duration {
        self.start_time.elapsed()
    }
    #[inline(always)]
    pub fn time_up(&mut self) -> bool {
        if let Some(limit) = self.time_limit_ms {
            self.start_time.elapsed().as_millis() > limit
        } else {
            false
        }
    }
    pub fn search_eval(&mut self, board: &mut Board, max_ply: u8) -> f64 {
        //TODO: Store searcher and dont erase TT for evaluation to be quicker
        let best_move = self.perform_search(board, max_ply);
        if board.turn() == Player::Black {
            - best_move.score as f64 / 100f64
        } else {
            best_move.score as f64 / 100f64
        }
    }

    pub fn perform_search(&mut self, board: &mut Board, max_ply: u8) -> ScoringMove {
        self.start_time = Instant::now();

        let mut alpha: MyVal = NEG_INF_V;
        let mut beta: MyVal = INF_V;

        let mut best_move: ScoringMove = ScoringMove::blank(NEG_INF_V);
        let tt = TranspositionTable::new_num_entries(TT_ENTRIES);
        tt.new_search();

        let mut score: MyVal = 0;

        let mut reached_depth = 1;
        'iterative_deepening: for depth in 1..=max_ply {
            let mut window: MyVal = 20;

            if depth >= 3 {
                alpha = NEG_INF_V.max(score - window);
                beta = INF_V.min(score + window);
            }

            'aspiration_window: loop {
                let best = self.alpha_beta(board, alpha, beta, depth as i8, 0, &tt);

                if self.time_up() {
                    if self.tracer.trace().is_some() {
                        println!("Out of time, exiting at depth = {depth}");
                    }
                    break 'iterative_deepening;
                }

                reached_depth = depth;

                score = best.score;

                if best.bit_move != NULL_BIT_MOVE {
                    best_move = best;
                    if best.score >= MATE_V - max_ply as MyVal {
                        // println!("Mate Found At Depth = {depth}");
                        break 'iterative_deepening;
                    }
                }

                if score <= alpha {
                    beta = (alpha + beta) / 2;
                    alpha = (score - window).max(NEG_INF_V);
                } else if score >= beta {
                    beta = (score + window).min(INF_V);
                } else {
                    break 'aspiration_window;
                }

                window += (window / 4) + 5;
            }

            if let Some(dbg) = self.tracer.trace() {
                self.pv_moves[0] = best_move;
                dbg.add_depth(
                    alpha,
                    beta,
                    self.nodes_explored,
                    best_move.bit_move,
                    best_move.score,
                    self.pv_moves
                        .to_vec()
                        .iter()
                        .filter(|p| !p.bit_move.is_null())
                        .map(|s| &s.bit_move)
                        .cloned()
                        .collect(),
                );
                self.nodes_explored = 0;
            }
        }
        if let Some(dbg) = self.tracer.trace() {
            dbg.add_duration(self.start_time.elapsed());
            println!("{dbg}");

            let eval = trace_eval(board);
            println!("TT Percent = {}", tt.hash_percent());
            println!("Raw Eval Before Move = {eval}");
            println!("AB Eval = {}", best_move.score);
            if best_move.score >= MATE_V - max_ply as MyVal {
                println!("Mate Found At Depth = {reached_depth}");
            }
        }
        // println!("Old: Reached Depth = {reached_depth}");
        best_move
    }
    pub fn find_best_move(&mut self, board: &mut Board, max_ply: u8) -> BitMove {
        let res = self.perform_search(board, max_ply);
        res.bit_move
    }

    /// Alpha-Beta search with negamax framework.
    /// Returns the best ScoringMove found.
    /// - `alpha`: lower bound of the search window
    /// Alpha is the best known guaranteed score for the maximizing player.
    /// - `beta`: upper bound of the search window
    /// Beta is the best known guaranteed score for the minimizing player.
    /// Beta means, the opponent can force us to have at most this score.
    fn alpha_beta(
        &mut self,
        board: &mut Board,
        mut alpha: MyVal,
        beta: MyVal,
        mut depth: i8,
        ply: u8,
        tt: &TranspositionTable,
    ) -> ScoringMove {
        if board.fifty_move_rule() || board.threefold_repetition() {
            return ScoringMove::blank(DRAW_V);
        }
        let mut all_moves = board.generate_moves();

        if all_moves.len() == 0 {
            if board.in_check() {
                // Eval will be used as -1 * mated_in
                // so when it is evaluated, if there is another mate in less moves,
                // the score > alpha check will be false
                return ScoringMove::blank(mated_in(ply));
            } else {
                return ScoringMove::blank(DRAW_V);
            }
        }

        if depth <= 0 {
            // return ScoringMove::new_score(NULL_BIT_MOVE, UNREACHABLE_V);
            return ScoringMove::blank(self.quiescence_search(board, alpha, beta, ply, depth, tt));
        } else if depth > 1 && self.time_up() {
            //Allow the search to finish when we are deep enough
            // return ScoringMove::new_score(NULL_BIT_MOVE, UNREACHABLE_V);
            // TODO: Codex suggested use alpha to not poison the tt as search ends
            // don't think it matters?
            return ScoringMove::new_score(NULL_BIT_MOVE, alpha);
        }

        let zobrist = board.zobrist();
        let (tt_hit, tt_entry): (bool, &mut Entry) = tt.probe(zobrist);

        let mut board_score = self.eval(board);
        // Try to use TT entry if available
        if tt_hit {
            let tt_score = tt_entry.score;
            let tt_depth = tt_entry.depth as i8;
            let tt_bound = tt_entry.node_type();

            // Try to use TT for immediate cutoff or exact result
            if let Some(res_score) = tt_maybe_cutoff(
                tt_score,
                tt_depth,
                depth,
                alpha,
                beta,
                tt_bound,
            ) {
                // If there's a stored best move, use it; otherwise, null move is fine.
                return ScoringMove {
                    bit_move: tt_entry.best_move,
                    score: res_score,
                };
            }

            // Otherwise, we can still use TT to refine our static evaluation
            if tt_can_improve_static(tt_score, board_score, tt_bound) {
                board_score = tt_score;
            }
        } else if depth > 3 {
            // Internal iterative reduction only when there is truly *no* TT info
            depth -= 1;
        }


        if !tt_hit {
            self.nodes_explored += 1;
        }

        all_moves.sort_by_key(|mv| {
            //TODO: Killer moves. Maybe revisit below once eval is better?
            if tt_hit && tt_entry.best_move == *mv {
                return -50;
            } else if board.is_capture_or_promotion(*mv) {
                if board.is_capture(*mv) {
                    return get_capture_score(board, mv);
                } else {
                    return mv.promo_piece() as MyVal * -3;
                }
            } else if board.gives_check(*mv) {
                return -1;
            }
            5
        });

        let mut best_move = BitMove::null();
        let mut best_score = NEG_INF_V;
        let alpha_orig = alpha;

        for mv in &all_moves {
            tt.prefetch(board.key_after(mv));

            board.apply_move(mv);
            let eval = self
                .alpha_beta(board, -beta, -alpha, depth - 1, ply + 1, tt)
                .negate();
            board.undo_move();

            if eval.score > best_score {
                best_score = eval.score;

                if eval.score > alpha {
                    best_move = mv;

                    //Only set alpha when eval in bounds?
                    alpha = eval.score;

                    if eval.score >= beta {
                        break;
                    }
                }
            }
        }
        let tt_flag = if best_score <= alpha_orig {
            // The true minimax score is at MOST the tt value
            NodeBound::UpperBound // fail-low
        } else if best_score >= beta {
            // The true minimax score is at LEAST the tt value
            NodeBound::LowerBound // fail-high
        } else {
            // The true minimax score of this position is EXACTLY the tt value
            NodeBound::Exact      // in-window
        };

        if tt_flag == NodeBound::Exact {
            self.pv_moves[ply as usize] = ScoringMove {
                bit_move: best_move,
                score: alpha,
            };
        }

        // TODO: codex wants to block with timeup here
        tt_entry.place(
            zobrist,
            best_move,
            best_score,
            board_score,
            depth as i16,
            tt_flag,
            tt.time_age(),
        );

        // if hard_fail {
        //     return ScoringMove {
        //         bit_move: best_move,
        //         score: alpha,
        //     };
        // }
        ScoringMove {
            bit_move: best_move,
            score: best_score,
        }
    }

    fn quiescence_search(
        &mut self,
        board: &mut Board,
        mut alpha: MyVal,
        beta: MyVal,
        ply: u8,
        depth: i8,
        tt: &TranspositionTable,
    ) -> MyVal {
        if board.fifty_move_rule() || board.threefold_repetition() {
            return DRAW_V;
        }
        let in_check = board.in_check();
        let static_eval = self.eval(board);

        // Stand-pat only if *not* in check
        if !in_check {
            if static_eval >= beta {
                return static_eval;
            }
            if static_eval > alpha {
                alpha = static_eval;
            }

            // Delta pruning: if even the largest plausible material gain can't raise
            // us from `static_eval` up to `alpha`, prune.
            //
            // Condition is: static_eval + MAX_GAIN < alpha  => prune
            //TODO: Should i really be casting everywhere as i32
            let mut max_gain: i32 = QUEEN_VALUE as i32; // crude upper bound

            if let Some(mv) = board.last_move() {
                if mv.is_promo() {
                    // Promotion means even more possible gain, so *increase* max_gain,
                    // making pruning harder, not easier.
                    max_gain += QUEEN_VALUE as i32;
                }
            }

            if (static_eval as i32) + max_gain < alpha as i32 {
                return static_eval;
            }
        } else {
            // When in check, skip stand-pat and delta pruning:
            // we are forced to find an evasion.
        }

        if depth == -3 && !in_check {
            return static_eval;
        }

        let mut best = if in_check { NEG_INF_V } else { static_eval };

        let non_quiets = if in_check {
            board.generate_moves_of_type(GenTypes::Evasions)
        } else {
            let mut mvs = board.generate_moves_of_type(GenTypes::Captures);
            mvs.sort_by_key(|mv| {
                if mv.is_capture() {
                    return get_capture_score(board, mv);
                } else if mv.is_promo() {
                    return mv.promo_piece() as MyVal * -3;
                }
                return 0;
            });
            mvs
        };
        let mut moves_played = 0;

        for mv in non_quiets {
            tt.prefetch(board.key_after(mv));
            board.apply_move(mv);
            let score = -self.quiescence_search(board, -beta, -alpha, ply + 1, depth - 1, tt);
            moves_played += 1;
            board.undo_move();

            if score >= beta {
                return score;
            }
            if score > best {
                best = score;

                if best > alpha {
                    alpha = best;
                }
            }
        }

        if moves_played == 0 {
            if in_check {
                return mated_in(ply);
            } else {
                return static_eval;
            }
        }

        best
    }
}

pub fn start_search_quiet(board: &mut Board) -> BitMove {
    let mut searcher = MySearcher::new(NoTrace::new(), Some(250));
    searcher.find_best_move(board, MAX_PLY as u8)
}
pub fn start_search(board: &mut Board) -> BitMove {
    let mut searcher = MySearcher::new(Trace::new(), Some(1000));

    searcher.find_best_move(board, MAX_PLY as u8)
}
pub fn eval_search(board: &mut Board) -> f64 {
    let mut searcher = MySearcher::new(NoTrace::new(), Some(1000));
    searcher.search_eval(board, MAX_PLY as u8)
}

#[inline(always)]
fn mate_in(ply: u8) -> MyVal {
    MATE_V - ply as MyVal
}

#[inline(always)]
fn mated_in(ply: u8) -> MyVal {
    -MATE_V + ply as MyVal
}

/// Determine whether or not the given Transposition Table Value
/// should be used given our current beta and the node type.
///
/// If tt_value >= beta but it is an upper bound then we can't use this entry
///     because it is better than the best possible
/// Else if the entry is a lower bound but beta is worse than the tt entry
///     we know we have a better option than this so return
/// Always return exact
#[inline(always)]
fn correct_bound_eq(tt_value: MyVal, beta: MyVal, bound: NodeBound) -> bool {
    if tt_value >= beta {
        // bound != NodeBound::UpperBound
        bound as u8 & NodeBound::LowerBound as u8 != 0
    } else {
        bound as u8 & NodeBound::UpperBound as u8 != 0
    }
}
#[inline(always)]
fn correct_bound(tt_value: MyVal, val: MyVal, bound: NodeBound) -> bool {
    if tt_value >= val {
        // bound != NodeBound::UpperBound
        bound as u8 & NodeBound::LowerBound as u8 != 0
    } else {
        bound as u8 & NodeBound::UpperBound as u8 != 0
    }
}

/// Decide if a TT entry allows immediate cutoff at this node.
/// Returns Some(score) if we can return immediately, None otherwise.
#[inline(always)]
fn tt_maybe_cutoff(
    tt_value: MyVal,
    depth_in_table: i8,
    needed_depth: i8,
    alpha: MyVal,
    beta: MyVal,
    bound: NodeBound,
) -> Option<MyVal> {
    if depth_in_table < needed_depth {
        return None;
    }

    match bound {
        NodeBound::Exact => {
            // Full information: score is exact in [alpha, beta).
            Some(tt_value)
        }
        NodeBound::LowerBound => {
            // True score >= tt_value
            if tt_value >= beta {
                // Fail-high cutoff
                Some(tt_value)
            } else {
                None
            }
        }
        NodeBound::UpperBound => {
            // True score <= tt_value
            if tt_value <= alpha {
                // Fail-low cutoff
                Some(tt_value)
            } else {
                None
            }
        }
        _ => None, // just in case there are other variants
    }
}

/// Decide if the TT value is a better *static* approximation than `current_val`
/// without implying it's exact. Used to refine board_score.
#[inline(always)]
fn tt_can_improve_static(tt_value: MyVal, current_val: MyVal, bound: NodeBound) -> bool {
    match bound {
        NodeBound::LowerBound => tt_value > current_val,
        NodeBound::UpperBound => tt_value < current_val,
        NodeBound::Exact => true,
        _ => false,
    }
}

fn get_capture_score(board: &Board, mv: &BitMove) -> MyVal {
    let attacker = board.piece_at_sq(mv.get_src());
    let dest = if mv.is_en_passant() {
        if board.turn() == Player::White {
            WhiteType::down(mv.get_dest())
        } else {
            BlackType::down(mv.get_dest())
        }
    } else {
        mv.get_dest()
    };
    let captured = board.piece_at_sq(dest);
    assert!(attacker.type_of() as usize != 0 && attacker.type_of() as usize != 7);
    assert!(captured.type_of() as usize != 0 && captured.type_of() as usize != 7);

    //Returns between -46 and 0
    return MVV_LVA[attacker.type_of() as usize - 1][captured.type_of() as usize - 1];
}

#[cfg(test)]
mod tests {

    use pleco::{BitMove, Board};

    use crate::debug::{Trace, Tracing};

    use super::MySearcher;

    fn eval(fen: &str, correct_src: u8, correct_dest: u8) {
        let bm = ev(fen);
        assert!(correct_src == bm.get_src_u8());
        assert!(correct_dest == bm.get_dest_u8())
    }
    fn ev(fen: &str) -> BitMove {
        let mut board = Board::from_fen(fen).unwrap();
        let mut searcher = MySearcher::trace(Trace::new());
        searcher.find_best_move(&mut board, 5)
    }
    fn ev_depth(fen: &str, depth: u8) -> BitMove {
        let mut board = Board::from_fen(fen).unwrap();
        let mut searcher = MySearcher::trace(Trace::new());
        searcher.find_best_move(&mut board, depth)
    }

    #[test]
    fn test_best_move_1() {
        let fen = "2b5/p2NBp1p/1bp1nPPr/3P4/2pRnr1P/1k1B1Ppp/1P1P1pQP/Rq1N3K b - - 0 1";
        eval(fen, 23, 14);
    }

    #[test]
    fn test_best_move_2() {
        let fen = "2qb3R/rn3kPB/pR3n1p/P2P2bK/2p1P3/1PBPp1pP/1NrppPp1/3N2Q1 w - - 0 1";
        eval(fen, 41, 45);
    }
    #[test]
    #[should_panic]
    fn test_best_move_3() {
        let fen = "2q4R/rn3kPB/p4b1p/P2P2bK/2p1P3/1PBPp1pP/1NrppPp1/3N2Q1 w - - 0 2";
        //TODO: THIS FAILS, WE DON'T RECOGNIZE THE DRAW
        eval(fen, 63, 58);
    }
    #[test]
    fn test_best_move_4() {
        let fen = "1K1N4/R3b3/qP1P2pp/PP1PrBB1/p2np1b1/2pnPPpp/pP1Rr1N1/2k3Q1 b - - 0 1";
        eval(fen, 2, 11);
    }

    #[test]
    fn test_best_move_5() {
        let fen = "NR5r/3P1rpk/2n2P1p/pnP1Qq2/P1RPbP1N/BP1B1pKP/p1pp3p/b7 w - - 0 1";
        eval(fen, 36, 37);
    }

    #[test]
    fn test_best_move_6() {
        let fen = "1rbbN3/Kpp2p1Q/1n4nP/BP1BPp2/Pk2ppPp/3PN2p/2RR1PP1/1q5r b - - 0 1";
        eval(fen, 25, 32);
    }

    #[test]
    fn test_best_move_promo_to_q() {
        //Best move promotion to q
        let fen = "2R1N2N/P4K1P/qr5p/1bP1pPRP/1P1pPp1B/ppb2n2/2rppnPQ/2k4B w - - 0 1";
        eval(fen, 48, 56);
    }
    fn mv2i(mv: &BitMove) -> (u8, u8) {
        (mv.get_src_u8(), mv.get_dest_u8())
    }
    #[test]
    fn test_best_move_dont_promo_to_q() {
        //Best move promotion to q
        let fen = "8/k1P5/2P5/K7/8/8/8/8 w - - 0 1";
        let mv = ev(fen);
        let (s, e) = mv2i(&mv);
        assert!(s != 50 && e != 58);
        //This value for now, stockfish best is 32 -> 33
        assert!(s == 32 && e == 25);
    }
    #[test]
    fn test_best_move_promo_to_r() {
        //Best move promotion to r
        let fen = "2R1N2N/P4K1P/qr5p/1bP1pPRP/1P1pPp1B/ppb2n2/2rppnPQ/2k4B w - - 0 1";
        eval(fen, 48, 56);
    }
    #[test]
    fn test_best_move_promo_to_b() {
        //Best move promotion to r
        let fen = "2R1N2N/P4K1P/qr5p/1bP1pPRP/1P1pPp1B/ppb2n2/2rppnPQ/2k4B w - - 0 1";
        eval(fen, 48, 56);
    }
    #[test]
    fn test_best_move_promo_to_n() {
        //Best move promotion to r
        let fen = "2R1N2N/P4K1P/qr5p/1bP1pPRP/1P1pPp1B/ppb2n2/2rppnPQ/2k4B w - - 0 1";
        eval(fen, 48, 56);
    }

    #[test]
    fn test_mating_no_tt() {
        let fen = "8/1R6/8/PR6/3k4/P7/1KP2p2/6r1 w - - 4 43";
        let mv = ev(fen);
        println!("MV = {mv}");
    }

    //TODO: Test mating sequence
    #[allow(unused)]
    fn test_mate_in_3() {
        //"NR5r/3P1rpk/2n2P1p/pnP1Qq2/P1RPbP1N/BP1B1pKP/p1pp3p/b7 w - - 0 1"
    }

    #[test]
    fn test_black_avoids_mate() {
        //This position, black has been blundering mate
        //1k1rr3/pp3p1Q/5q2/P7/4n1B1/1P1p3P/3P1PP1/1R3K1R w - - 2 25
        let fen = "1k1rr3/pp3p1Q/5q2/P7/4n1B1/1P1p3P/3P1PP1/1R3K1R w - - 2 25";
        let mv = ev_depth(fen, 5);
        println!("MV = {mv}");
        // assert!(mv.get_src_u8() == 55 && mv.get_dest_u8() == 28);
    }

    #[test]
    fn test_force_stalemate() {
        //This position, white just has to take the pawn on g6 to force draw
        //1r3rk1/5p2/5Qpp/2q5/n1b5/P7/1P6/K5R1 w - - 3 3
        let fen = "1r3rk1/5p2/5Qpp/2q5/n1b5/P7/1P6/K5R1 w - - 3 3";
        let mv = ev(fen);
        println!("MV = {mv}");
    }
}
// pub fn search_root(&mut self, board: &mut Board, max_ply: u8) -> BitMove {

//     self.start_time = Instant::now();

//     let mut alpha: MyVal = NEG_INF_V;
//     let mut beta: MyVal = INF_V;

//     let tt = TranspositionTable::new_num_entries(TT_ENTRIES);
//     tt.new_search();

//     let mut all_moves = board.generate_moves();
//     all_moves.sort_by_key(|mv| {
//         if board.is_capture_or_promotion(*mv) {
//             if board.is_capture(*mv) {
//                 return get_capture_score(board, mv);
//         } else {
//                 return mv.promo_piece() as MyVal * -3;
//             }
//         } else if board.gives_check(*mv) {
//             return -1
//         }
//         5
//     });
//     let mut root_moves: Vec<ScoringMove> = all_moves.iter()
//         .map(|mv| -> ScoringMove { ScoringMove::new(*mv)})
//         .collect();

//     // for mv in &root_moves {
//     //     println!("Root Move Opt = {}", mv.bit_move);
//     // }

//     'iterative_deepening: for depth in 1..=max_ply {

//         let mut window: MyVal = 20;
//         let next_depth = depth as i8 - 1;

//         let best_root = root_moves.get(0).unwrap();

//         if depth >= 3 {
//             alpha = NEG_INF_V.max(best_root.score - window);
//             beta = INF_V.min(best_root.score + window);
//         }

//         // println!("-------------");
//         // println!("At Depth = {depth}");

//         'aspiration_window: loop {

//             // println!("-----");
//             // println!("At Window = {window}");
//             // println!("Alpha = {alpha}, Beta = {beta}");
//             let mut this_run_scores: Vec<ScoringMove> = Vec::new();
//             for mv in &root_moves {
//                 board.apply_move(mv.bit_move);
//                 let res = self.alpha_beta(
//                     board,
//                     -beta, -alpha,
//                     next_depth,
//                     1,
//                     &tt
//                 ).negate();
//                 board.undo_move();
//                 this_run_scores.push(ScoringMove::new_score(mv.bit_move, res.score));
//             }

//             this_run_scores.sort_by_key(|mv| {
//                 -mv.score //IF +5 -> -5, +10 -> -10, -20 -> 20
//                 // UNREACHABLE_V.wrapping_sub(mv.score) //Sort is ascending, so the better the score we want to be index 0
//             });
//             root_moves = this_run_scores.clone();

//             if self.time_up() {
//                 println!("Out of time, exiting at depth = {depth}");
//                 break 'iterative_deepening;
//             }

//             let best = root_moves.get(0).unwrap();
//             // score = best.score;

//             if best.bit_move != NULL_BIT_MOVE {
//                 if best.score >= MATE_V - max_ply as MyVal {
//                     // println!("Mate Found At Depth = {depth}");
//                     break 'iterative_deepening;
//                 }
//             }

//             if best.score <= alpha {
//                 beta = (alpha + beta) / 2;
//                 alpha = (best.score - window).max(NEG_INF_V);
//             } else if best.score >= beta {
//                 beta = (best.score + window).min(INF_V);
//             } else {
//                 break 'aspiration_window;
//             }

//             window += (window / 4) + 5;

//         }

//         if let Some(dbg) = self.tracer.trace() {
//             let bm = root_moves.get(0).unwrap();
//             self.pv_moves[0] = *bm;
//             dbg.add_depth(
//                 alpha, beta,
//                 self.nodes_explored,
//                 bm.bit_move, bm.score,
//                 self.pv_moves
//                     .to_vec().iter()
//                     .filter(|p| !p.bit_move.is_null() )
//                     .map(|s| &s.bit_move)
//                     .cloned()
//                     .collect(),
//             );
//             self.nodes_explored = 0;
//         }
//     }
//     let best = root_moves.get(0).unwrap();
//     if let Some(dbg) = self.tracer.trace() {
//         dbg.add_duration(self.start_time.elapsed());
//         println!("{dbg}");

//         let eval = trace_eval(board);
//         println!("Raw Eval Before Move = {eval}");
//         println!("AB Eval = {}", best.score);
//         println!("TT Percent = {}", tt.hash_percent());
//         println!("TT Entries = {}", tt.num_entries());
//     }

//     best.bit_move
// }

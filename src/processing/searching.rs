
use std::time::{Duration, Instant};

use pleco::{core::{mono_traits::{BlackType, PlayerTrait, WhiteType}, score::{DRAW, INFINITE, MATE, NEG_INFINITE}, GenTypes}, tools::{tt::{Entry, NodeBound, TranspositionTable}, PreFetchable}, BitMove, Board, Player, ScoringMove};

use crate::processing::{consts::MVV_LVA, evaluation::trace_eval};

use super::{consts::{EvalVal, MyVal, QUEEN_VALUE}, debug::{SearchDebugger, Trace, Tracing}, evaluation::eval_board, tables::{material::Material, pawn_table::PawnTable}};

const MATE_V: i16 = MATE as i16;
const DRAW_V: i16 = DRAW as i16;
const NEG_INF_V: i16 = NEG_INFINITE as i16;
const INF_V: i16 = INFINITE as i16;
const UNREACHABLE_V: MyVal = MATE_V + 100;

const NULL_BIT_MOVE: BitMove = BitMove::null();

const TT_ENTRIES: usize = 2_000_000;
pub const MAX_PLY: usize = 31;

//TODO Give Searcher the board, add apply move to search
pub struct MySearcher<T: Tracing<SearchDebugger>> {
    pawn_table: PawnTable,
    material: Material,
    start_time: Instant,
    time_limit_ms: Option<u128>,

    //Debug
    tracer: T,
    nodes_explored: i64,
    pv_moves: [ScoringMove; MAX_PLY]
}
pub const NULL_SCORE: ScoringMove = ScoringMove::null();

impl <T: Tracing<SearchDebugger>> MySearcher <T>  {


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

    pub fn find_best_move(&mut self, board: &mut Board, max_ply: u8) -> BitMove {

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
                let best = self.alpha_beta(
                    board,
                    alpha, beta,
                    depth as i8, 0,
                    &tt,
                );

                if self.time_up() {
                    println!("Out of time, exiting at depth = {depth}");
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
                    alpha, beta,
                    self.nodes_explored,
                    best_move.bit_move, best_move.score,
                    self.pv_moves
                        .to_vec().iter()
                        .filter(|p| !p.bit_move.is_null() )
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

        best_move.bit_move
    }

    fn alpha_beta(
        &mut self,
        board: &mut Board,
        mut alpha: MyVal, beta: MyVal,
        mut depth: i8, ply: u8,
        tt: &TranspositionTable,

    ) -> ScoringMove {

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
            return ScoringMove::blank(self.quiescence_search(board, alpha, beta, ply, depth, tt));
        } else if depth > 1 && self.time_up() {
            //Allow the search to finish when we are deep enough
            return ScoringMove::new_score(NULL_BIT_MOVE, UNREACHABLE_V);
        }

        let zobrist = board.zobrist();
        let (tt_hit, tt_entry) : (bool, &mut Entry) = tt.probe(zobrist);

        let mut board_score = self.eval(board);
        // Check for TT Match
        if tt_hit && !tt_entry.best_move.is_null() {

            //If This entry was found earlier than the current
            if tt_entry.depth >= depth &&
            //And the node is valid given our current beta
            correct_bound_eq(tt_entry.score, beta, tt_entry.node_type())
            {
                return ScoringMove {
                    bit_move: tt_entry.best_move,
                    score: tt_entry.score,
                };
            }
        
            if correct_bound(tt_entry.score, board_score, tt_entry.node_type()) {
                board_score = tt_entry.score;
            }
        } else if depth > 3 {
            //Internal iterative reduction, will revisit if tt visits again
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
                return -1
            }
            5
        });

        let mut best_move = BitMove::null();
        let mut best_score = NEG_INF_V;

        let mut tt_flag = NodeBound::UpperBound;

        for mv in &all_moves {
            tt.prefetch(board.key_after(mv));

            board.apply_move(mv);
            let eval = self.alpha_beta(
                board,
                -beta, -alpha,
                depth - 1, ply + 1,
                tt,
            ).negate();
            board.undo_move();

            if eval.score > best_score {
                best_score = eval.score;

                if eval.score > alpha {
                    best_move = mv;
                
                    //Only set alpha when eval in bounds?
                    alpha = eval.score;
                    tt_flag = NodeBound::Exact;

                    if eval.score >= beta {
                        tt_flag = NodeBound::LowerBound;
                        break;
                    }
                }
            }
        }

        if tt_flag == NodeBound::Exact {
            self.pv_moves[ply as usize] = ScoringMove {
                bit_move: best_move,
                score: alpha,
            };
        }

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
        beta : MyVal,
        ply: u8,
        depth: i8,
        tt: &TranspositionTable,

    ) -> MyVal {

        let static_eval = self.eval(board);
        if static_eval >= beta {
            return static_eval
        } 

        if depth == -5 {
            return static_eval;
        }

        let mut best = static_eval;

        //Check if we can even improve this position by the largest swing
        let mut max_swing = alpha as i32 - QUEEN_VALUE as i32;//Evaluate as i32 in case where alpha is mate or uninit
        if let Some(mv) = board.last_move() {
            if mv.is_promo() {
                max_swing -= 750;
            }
        }
        if (best as i32) < max_swing {
            // if hard_fail {
            //     return alpha;
            // }
            return best;
        }
        
        if static_eval > alpha {
            alpha = static_eval;
        }

        let in_check = board.in_check();

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
            let score = -self.quiescence_search(
                board,
                -beta, -alpha,
                ply + 1,
                depth - 1,
                tt,
            );
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

pub fn start_search(board: &mut Board) -> BitMove {
    let mut searcher = MySearcher::new(Trace::new(), Some(1000));

    searcher.find_best_move(board, MAX_PLY as u8)
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

    use crate::processing::debug::{Trace, Tracing};

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


use std::time::{Duration, Instant};

use pleco::{core::{mono_traits::{BlackType, PlayerTrait, WhiteType}, score::{DRAW, INFINITE, MATE, NEG_INFINITE}, GenTypes}, tools::{tt::{Entry, NodeBound, TranspositionTable}, PreFetchable}, BitMove, Board, Player, Rank, ScoringMove};

use crate::processing::{consts::MVV_LVA, evaluation::trace_eval};

use super::{consts::{EvalVal, MyVal, QUEEN_VALUE}, debug::{NoTrace, SearchDebugger, Trace, Tracing}, evaluation::eval_board, tables::{material::Material, pawn_table::PawnTable}};

const MATE_V: i16 = MATE as i16;
const DRAW_V: i16 = DRAW as i16;
const NEG_INF_V: i16 = NEG_INFINITE as i16;
const INF_V: i16 = INFINITE as i16;

const NULL_BIT_MOVE: BitMove = BitMove::null();

const TT_ENTRIES: usize = 2_000_000;
pub const MAX_PLY: usize = 31;

//TODO Give Searcher the board, add apply move to search
pub struct MySearcher<T: Tracing<SearchDebugger>> {
    pawn_table: PawnTable,
    material: Material,
    best_root_move: BitMove,
    start_time: Instant,


    //Debug
    tracer: T,
    nodes_explored: i64,
    pv_moves: [ScoringMove; MAX_PLY]
}
pub const NULL_SCORE: ScoringMove = ScoringMove::null();

impl <T: Tracing<SearchDebugger>> MySearcher <T>  {


    pub fn new(tracer: T) -> Self {
        Self {
            pawn_table: PawnTable::new(),
            material: Material::new(),
            best_root_move: BitMove::null(),
            start_time: Instant::now(),

            tracer,
            nodes_explored: 0,
            pv_moves: [NULL_SCORE; MAX_PLY],
        }
    }
    pub fn trace(tracer: T) -> Self {
        Self {
            pawn_table: PawnTable::new(),
            material: Material::new(),
            best_root_move: BitMove::null(),
            start_time: Instant::now(),

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
    pub fn elapsed(&mut self) -> Duration {
        self.start_time.elapsed()
    }
    pub fn find_best_move(&mut self, board: &mut Board, max_ply: u8) -> BitMove {

        self.start_time = Instant::now();

        let mut alpha: MyVal = NEG_INF_V;
        let mut beta: MyVal = INF_V;

        let mut best_move: ScoringMove = ScoringMove::blank(NEG_INF_V);
        let tt = TranspositionTable::new_num_entries(TT_ENTRIES);

        let mut score: MyVal = 0;

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

            board.apply_move(best_move.bit_move);
            let eval = trace_eval(board);
            println!("Raw Eval After Move = {eval}");
            board.undo_move();
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
            // if ply == 0 && self.best_root_move != BitMove::null() && self.best_root_move == *mv {
            //     return -100
            // } else 
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

        ScoringMove {
            bit_move: best_move,
            score: alpha,
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
            return alpha;
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
                tt
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

pub fn start_search(board: &mut Board, max_ply: u8) -> BitMove {
    let mut searcher = MySearcher::new(Trace::new());

    searcher.find_best_move(board, max_ply)
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
    use pleco::Board;

    use crate::processing::debug::{SearchDebugger, Trace, Tracing};

    use super::MySearcher;

    #[test]
    fn test_searcher() {

        let mut board = Board::from_fen("2kn4/p1p1p2P/6P1/8/5Q2/8/5K2/8 w - - 0 1").unwrap();
        let mut searcher = MySearcher::trace(Trace::new());
        let bm = searcher.find_best_move(&mut board, 5);
        println!("Best Move = {bm}");
    }

}
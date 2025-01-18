
use pleco::{board, core::{score::{DRAW, INFINITE, MATE, NEG_INFINITE}, GenTypes}, tools::{tt::{self, Entry, NodeBound, TranspositionTable}, PreFetchable}, BitMove, Board, ScoringMove};

use super::{consts::{MyVal, STALEMATE}, evaluator::eval_board};

const MATE_V: i16 = MATE as i16;
const DRAW_V: i16 = DRAW as i16;
const NEG_INF_V: i16 = NEG_INFINITE as i16;
const INF_V: i16 = INFINITE as i16;

const NULL_BIT_MOVE: BitMove = BitMove::null();

const TT_ENTRIES: usize = 2_000_000;

pub fn find_best_move(board: &mut Board, max_ply: u8) -> BitMove {


    let mut alpha: MyVal = NEG_INF_V;
    let mut beta: MyVal = INF_V;

    let mut best_move: ScoringMove = ScoringMove::blank(NEG_INFINITE as i16);
    let tt = TranspositionTable::new_num_entries(TT_ENTRIES);


    for i in 1..=max_ply {

        let best = alpha_beta(
            board,
            alpha, beta,
            i as i8, 0,
            &tt,
        );

        if best.bit_move != NULL_BIT_MOVE {
            best_move = best;
            if best.score >= MATE_V - max_ply as MyVal {
                println!("Mate Found At Depth = {i}");
                break;
            }
        }
    }
    
    best_move.bit_move
}

fn alpha_beta(
    board: &mut Board,
    mut alpha: MyVal, beta: MyVal,
    depth: i8, ply: u8,
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

    let zobrist = board.zobrist();
    let (tt_hit, tt_entry) : (bool, &mut Entry) = tt.probe(zobrist);

    // Check for TT Match
    if tt_hit && !tt_entry.best_move.is_null() {

        if tt_entry.depth >= depth && //If This entry was found earlier than the current
           //And the node is valid given our current beta
           correct_bound_eq(tt_entry.score, beta, tt_entry.node_type())
        {
            return ScoringMove {
                bit_move: tt_entry.best_move,
                score: tt_entry.score,
            };
        }
    }

    all_moves.sort_by_key(|mv| {
        //Killer moves, if it is the tt_move
        if board.is_capture_or_promotion(*mv) {
            return 0
        } else if board.gives_check(*mv) {
            return 1
        }
        2
    });

    if depth == 0 {
        //TODO: quiesence search
        return ScoringMove::blank(eval_board(board));
    }

    let mut best_move = BitMove::null();
    let mut best_score = NEG_INF_V;

    let mut tt_flag = NodeBound::UpperBound;

    for mv in &all_moves {
        board.apply_move(mv);
        let eval = alpha_beta(
            board,
            -beta, -alpha,
            depth - 1, ply + 1,
            tt,
        ).negate();
        board.undo_move();

        tt.prefetch(board.zobrist());

        if eval.score > best_score {
            best_score = eval.score;

        if eval.score > alpha {
                best_move = mv;
            alpha = eval.score;
                tt_flag = NodeBound::Exact;
            
                if eval.score >= beta {
                    tt_flag = NodeBound::LowerBound;
                    break;
                }
            }
            
        }
    }

    tt_entry.place(
        zobrist, 
        best_move, 
        best_score, 
        0, 
        depth as i16, 
        tt_flag, 
        tt.time_age(),
    );

    return ScoringMove {
        bit_move: best_move,
        score: alpha,
    };
}


fn mate_in(ply: u8) -> MyVal {
    MATE_V - ply as MyVal
}

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
fn correct_bound_eq(tt_value: MyVal, beta: MyVal, bound: NodeBound) -> bool {
    if tt_value >= beta {
        // bound != NodeBound::UpperBound
        bound as u8 & NodeBound::LowerBound as u8 != 0
    } else {
        bound as u8 & NodeBound::UpperBound as u8 != 0
    }

}
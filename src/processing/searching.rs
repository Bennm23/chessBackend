use pleco::{board, core::{score::{DRAW, INFINITE, MATE, NEG_INFINITE}, GenTypes}, BitMove, Board, ScoringMove};

use super::{consts::{MyVal, STALEMATE}, evaluator::eval_board};

const MATE_V: i16 = MATE as i16;
const DRAW_V: i16 = DRAW as i16;
const NEG_INF_V: i16 = NEG_INFINITE as i16;
const INF_V: i16 = INFINITE as i16;

const NULL_BIT_MOVE: BitMove = BitMove::null();


pub fn find_best_move(board: &mut Board, max_ply: u8) -> BitMove {


    let mut alpha: MyVal = NEG_INF_V;
    let mut beta: MyVal = INF_V;

    let mut best_move: ScoringMove = ScoringMove::blank(NEG_INFINITE as i16);

    for i in 1..max_ply {

        let best = alpha_beta(board, alpha, beta, i, 0);

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

fn alpha_beta(board: &mut Board, mut alpha: MyVal, beta: MyVal, depth: u8, ply: u8) -> ScoringMove {

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

    //tt.prefetch board

    let mut best_move = BitMove::null();

    for mv in &all_moves {
        board.apply_move(mv);
        let eval = alpha_beta(board, -beta, -alpha, depth - 1, ply + 1).negate();
        board.undo_move();

        if eval.score > alpha {
            alpha = eval.score;
            best_move = mv;
        }

        if alpha >= beta {

            return ScoringMove {
                bit_move: mv,
                score: alpha,
            };
            
        }
    }

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
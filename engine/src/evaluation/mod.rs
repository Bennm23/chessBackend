use pleco::{Board, Player};

use super::{
    consts::EvalVal,
    debug::{NoTrace, Trace, Tracing},
    tables::{material::Material, pawn_table::PawnTable},
};

mod ai_eval;

pub fn eval_board(board: &Board, pawn_table: &mut PawnTable, material: &mut Material) -> EvalVal {
    let mut evaluator = ai_eval::BasicEvaluator::new(board, NoTrace::new(), pawn_table, material);
    let mut res = evaluator.white_score(); // white POV

    // Convert to side-to-move POV.
    if board.turn() == Player::Black {
        res = -res;
    }

    // Simple tempo bonus: side to move gets a small edge.
    const TEMPO_BONUS: EvalVal = 10;
    if board.turn() == Player::White {
        res + TEMPO_BONUS
    } else {
        res - TEMPO_BONUS
    }
}

pub fn trace_eval(board: &Board) -> EvalVal {
    let mut pawn_table = PawnTable::new();
    let mut material = Material::new();

    let mut evaluator = ai_eval::BasicEvaluator::new(board, Trace::new(), &mut pawn_table, &mut material);
    let res = evaluator.white_score();

    if board.turn() == Player::Black {
        return -res;
    }
    res
}

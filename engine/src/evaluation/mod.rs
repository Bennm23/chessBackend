use basic_eval::BasicEvaluator;
use pleco::{Board, Player};

use super::{consts::EvalVal, debug::{NoTrace, Trace, Tracing}, tables::{material::Material, pawn_table::PawnTable}};

mod basic_eval;
mod ai_eval;


pub fn eval_board(board: &Board, pawn_table: &mut PawnTable, material: &mut Material) -> EvalVal {
    let mut evaluator =  BasicEvaluator::new(
        board, NoTrace::new(),
        pawn_table,
        material,
    );
    let res = evaluator.white_score();
    if board.turn() == Player::Black {
        return -res
    }
    res
}
pub fn eval_board_ai(board: &Board, pawn_table: &mut PawnTable, material: &mut Material) -> EvalVal {
    let mut evaluator = ai_eval::BasicEvaluator::new(
        board,
        NoTrace::new(),
        pawn_table,
        material,
    );
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

    let mut evaluator =  BasicEvaluator::new(
        board,
        Trace::new(),
        &mut pawn_table,
        &mut material
    );
    let res = evaluator.white_score();

    if board.turn() == Player::Black {
        return -res
    }
    res
}


#[cfg(test)]
mod tests {
    use pleco::Board;

    use super::trace_eval;


    #[test]
    fn test_eval() {
        // let fen = "8/1R6/8/P7/1R1k4/P7/1KP2p2/6r1 b - - 5 43";
        let fen = "2k5/R7/8/8/4Q3/P7/1K6/8 b - - 8 53";

        let board = Board::from_fen(fen).unwrap();

        let res = trace_eval(&board);
        println!("Returned Eval = {res}");
    }
}
use basic_eval::BasicEvaluator;
use pleco::{board, Board, Player};

use super::{consts::EvalVal, debug::{NoTrace, Trace, Tracing}, tables::{material::Material, pawn_table::PawnTable}};

mod basic_eval;


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
        let fen = "3qkb1r/3ppp2/3r1np1/2Q4p/5P2/1P3B2/P1P1PP1P/R2NK2R b k - 0 22";

        let board = Board::from_fen(fen).unwrap();

        let res = trace_eval(&board);
        println!("Returned Eval = {res}");
    }
}
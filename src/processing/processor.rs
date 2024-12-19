use std::fmt::Display;

use crate::generated::chess::{*};

pub fn get_valid_moves(request : &GetValidMoves) -> Option<Vec<Position>> {
    
    let board = match request.board.as_ref() {
        Some(b) => b,
        None => {
            println!("Could not read board from Get Valid Moves Message");
            return None;
        }
    };

    let piece = request.piece_to_move.clone().unwrap();
    Some(piece.get_valid_moves(&board))
}


fn square_occupied(board : &Board, col : i32, row : i32) -> bool {
    let index:usize = (row * 8 + col % 8) as usize;

    match board.pieces.get(index) {
        Some(piece) => {

            return piece.type_.unwrap().eq(&PieceType::NONE);
        },
        None => { return false; }
    }
}
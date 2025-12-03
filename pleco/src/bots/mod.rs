//! Contains all of the currently completed standard bots/searchers/AIs.
//!
//! These are mostly for example purposes, to see how one can create a chess AI.

extern crate rand;

pub mod alphabeta;

use board::Board;
use core::piece_move::*;
use core::score::*;
use tools::eval::*;
use tools::Searcher;

const MAX_PLY: u16 = 4;
const MATE_V: i16 = MATE as i16;
const DRAW_V: i16 = DRAW as i16;
const NEG_INF_V: i16 = NEG_INFINITE as i16;
const INF_V: i16 = INFINITE as i16;

struct BoardWrapper<'a> {
    b: &'a mut Board,
}


/// Searcher that uses an alpha-beta algorithm to search for a best move.
pub struct AlphaBetaSearcher {}

impl Searcher for AlphaBetaSearcher {
    fn name() -> &'static str {
        "AlphaBeta Searcher"
    }

    fn best_move(board: Board, depth: u16) -> BitMove {
        let alpha = NEG_INF_V;
        let beta = INF_V;
        alphabeta::alpha_beta_search(&mut board.shallow_clone(), alpha, beta, depth).bit_move
    }
}

#[doc(hidden)]
pub fn eval_board(board: &Board) -> ScoringMove {
    ScoringMove::blank(Eval::eval_low(board) as i16)
}

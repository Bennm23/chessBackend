use std::{cmp::Ordering, collections::HashMap, sync::Arc, time::Instant};

use dashmap::DashMap;
use protobuf::{Enum, EnumOrUnknown, MessageField};
use rand::Rng;
use rayon::{iter::ParallelIterator, slice::ParallelSlice};

use crate::{generated::chess::*, NUM_THREADS};

#[derive(PartialEq)]
pub enum ReturnState {
    True,
    TrueExit,
    False,
}

#[derive(Copy, Clone)]
enum NodeType {
    Exact,
    Lower,
    Upper,
}
#[derive(Copy, Clone)]
struct TranspositionEntry {
    eval: f32,
    depth: i8,
    node_type: NodeType,
}
type TranspositionTable = DashMap<u64, TranspositionEntry>;

const BOARD_SIZE: usize = 64;
const NUM_PIECES: usize = 64;

pub struct ZobristHash {
    random_table: [[u64; BOARD_SIZE]; NUM_PIECES],
    side_to_move: u64,
    castling_rights: [u64; 4],
}

impl ZobristHash {
    fn new() -> Self {
        let mut rng = rand::thread_rng();
        let mut random_table = [[0u64; BOARD_SIZE]; NUM_PIECES];
        for piece in 0..NUM_PIECES {
            for square in 0..BOARD_SIZE {
                random_table[piece][square] = rng.gen();
            }
        }
        let side_to_move = rng.gen();
        let castling_rights = [rng.gen(), rng.gen(), rng.gen(), rng.gen()];

        Self {
            random_table,
            side_to_move,
            castling_rights,
        }
    }
}

#[derive(Clone)]
struct EvalPair(Move, f32);

impl Move {
    pub fn set_piece(&mut self, piece: ProtoPiece) {
        self.piece_to_move = MessageField::some(piece);
    }
    pub fn set_move(&mut self, pos: Position) {
        self.end_position = MessageField::some(pos);
    }
    pub fn set_secondary_move(&mut self, pos: Position) {
        self.secondary_end_pos = MessageField::some(pos);
    }
    pub fn get_type(&self) -> PieceType {
        self.piece_to_move.type_.unwrap()
    }
}

impl PieceColor {
    pub fn opposite(&self) -> PieceColor {
        PieceColor::from_i32((self.value() + 1) % 2).unwrap()
    }
}

impl Board {
    pub fn get_piece(&self, row: i32, col: i32) -> Option<&ProtoPiece> {
        let index: usize = (row * 8 + col % 8) as usize;

        match self.pieces.get(index) {
            Some(piece) => Some(piece),
            None => None,
        }
    }
    pub fn square_empty(&self, position: &Position) -> bool {
        self.square_empty_grid(position.col, position.row)
    }
    pub fn square_empty_grid(&self, col: i32, row: i32) -> bool {
        let index: usize = (row * 8 + col % 8) as usize;

        match self.pieces.get(index) {
            Some(piece) => {
                return piece.type_.unwrap().eq(&PieceType::NONE);
            }
            None => {
                return false;
            }
        }
    }

    pub fn validate_position(&self, position: &Position, piece: &ProtoPiece) -> ReturnState {
        self.validate_grid_position(position.col, position.row, piece)
    }
    pub fn validate_grid_position(&self, col: i32, row: i32, piece: &ProtoPiece) -> ReturnState {
        if Position::out_of_bounds(col, row) {
            return ReturnState::False;
        }
        if self.square_empty_grid(col, row) {
            ReturnState::True
        } else if piece.can_capture_grid(col, row, self) {
            ReturnState::TrueExit
        } else {
            return ReturnState::False;
        }
    }

    fn get_all_moves_for_current(&self) -> Vec<Move> {
        let mut all_moves: Vec<Move> = Vec::new();

        let color = self.player_to_move.unwrap();

        for piece in &self.pieces {
            if !piece.color.unwrap().eq(&color) {
                continue;
            }
            let mut this_move: Move = Move::default();
            this_move.set_piece(piece.clone());
            for move_pos in piece.get_valid_moves(&self) {
                this_move.set_move(move_pos.primary_move().clone());
                if let Some(secondary) = move_pos.secondary_move() {
                    this_move.set_secondary_move(secondary.clone());
                }
                all_moves.insert(0, this_move.clone());
            }
        }

        all_moves
    }

    fn update_castle_rights(&mut self) {
        let black_king = self.get_piece(0, 4);
        if black_king.is_some()
            && black_king
                .unwrap()
                .is_piece(PieceColor::BLACK, PieceType::KING)
        {
            if self.black_long_castle {
                if let Some(long_rook) = self.get_piece(0, 0) {
                    self.black_castle = long_rook.is_piece(PieceColor::BLACK, PieceType::ROOK);
                }
            }
            if self.black_castle {
                if let Some(short_rook) = self.get_piece(0, 7) {
                    self.black_castle = short_rook.is_piece(PieceColor::BLACK, PieceType::ROOK);
                }
            }
        } else {
            self.black_castle = false;
            self.black_long_castle = false;
        }
        let white_king = self.get_piece(7, 4);
        if white_king.is_some()
            && white_king
                .unwrap()
                .is_piece(PieceColor::WHITE, PieceType::KING)
        {
            if self.white_long_castle {
                if let Some(long_rook) = self.get_piece(7, 0) {
                    self.white_castle = long_rook.is_piece(PieceColor::WHITE, PieceType::ROOK);
                }
            }
            if self.white_castle {
                if let Some(short_rook) = self.get_piece(7, 7) {
                    self.white_castle = short_rook.is_piece(PieceColor::WHITE, PieceType::ROOK);
                }
            }
        } else {
            self.white_castle = false;
            self.white_long_castle = false;
        }
    }

    pub fn make_move(&self, mv: &Move) -> Board {
        let mut new = self.clone();
        let curr_piece = mv.piece_to_move.as_ref().unwrap();

        if new.player_to_move.unwrap() == PieceColor::WHITE {
            new.player_to_move = EnumOrUnknown::new(PieceColor::BLACK);
        } else {
            new.player_to_move = EnumOrUnknown::new(PieceColor::WHITE);
        }

        new.update_castle_rights();

        let old_index = (&curr_piece.row * 8 + &curr_piece.col % 8) as usize;
        let new_index = (&mv.end_position.row * 8 + &mv.end_position.col % 8) as usize;
        //Update destination with old position values
        new.pieces[old_index].col = mv.end_position.col;
        new.pieces[old_index].row = mv.end_position.row;
        //Update the new piece with old position values
        new.pieces[new_index].col = curr_piece.col;
        new.pieces[new_index].row = curr_piece.row;
        new.pieces[new_index].type_ = EnumOrUnknown::from_i32(0);
        //Move old to new pos
        new.pieces.swap(old_index, new_index);

        new
    }

    pub fn raw_score(&self) -> f32 {
        let mut white_score = 0.0;
        let mut black_score = 0.0;
        for piece in &self.pieces {
            if piece.type_.unwrap() == PieceType::NONE {
                continue;
            }
            if piece.color.unwrap() == PieceColor::WHITE {
                white_score += piece.get_score(self);
                // white_score += piece.get_score(self, all_moves);
            } else {
                black_score += piece.get_score(self);
                // black_score += piece.get_score(self, all_moves.as_slice());
            }
        }
        white_score - black_score
    }

    fn alpha_beta(
        &self,
        depth: i8,
        mut alpha: f32,
        mut beta: f32,
        maximizer: bool,
        transpositions: &mut TranspositionTable,
        zobrist: &ZobristHash,
    ) -> f32 {
        let board_hash = self.eval_hash(zobrist);
        if let Some(entry) = transpositions.get(&board_hash) {
            if entry.depth >= depth {
                match entry.node_type {
                    NodeType::Exact => return entry.eval,
                    NodeType::Lower => alpha = alpha.max(entry.eval),
                    NodeType::Upper => beta = beta.min(entry.eval),
                }

                if alpha >= beta {
                    return entry.eval;
                }
            }
        }

        if depth == 0 {
            let eval = self.raw_score();

            transpositions.insert(
                board_hash,
                TranspositionEntry {
                    eval,
                    depth,
                    node_type: NodeType::Exact,
                },
            );
            return eval;
        }

        let mut best_eval = if maximizer {
            f32::NEG_INFINITY
        } else {
            f32::INFINITY
        };
        let mut node_type = NodeType::Exact;

        let moves = self.get_all_moves_for_current();

        for mv in &moves {
            let new_board = self.make_move(mv);
            let eval =
                new_board.alpha_beta(depth - 1, alpha, beta, !maximizer, transpositions, zobrist);

            if maximizer {
                best_eval = best_eval.max(eval);
                alpha = alpha.max(eval);
                if alpha >= beta {
                    node_type = NodeType::Lower;
                    break;
                }
            } else {
                best_eval = best_eval.min(eval);
                beta = beta.min(eval);
                if beta <= alpha {
                    node_type = NodeType::Upper;
                    break;
                }
            }
        }
        transpositions.insert(
            board_hash,
            TranspositionEntry {
                eval: best_eval,
                depth,
                node_type,
            },
        );

        best_eval
    }

    pub fn find_best_move(&self, depth: i8, _player: &PieceColor) -> Option<Move> {
        let start = Instant::now();

        let mut best_move: Option<Move> = None;
        let mut best_score = f32::NEG_INFINITY;

        let moves = self.get_all_moves_for_current();

        let mut transpositions: TranspositionTable = TranspositionTable::new();
        let zobrist = ZobristHash::new();

        for mv in &moves {
            let new_board = self.make_move(mv);
            let eval = new_board.alpha_beta(
                depth - 1,
                f32::NEG_INFINITY,
                f32::INFINITY,
                false,
                &mut transpositions,
                &zobrist,
            );
            if eval > best_score {
                best_score = eval;
                best_move = Some(mv.clone());
            }
        }

        let elapsed = start.elapsed();

        println!("Find Took {} ms", elapsed.as_millis());

        best_move
    }

    pub fn find_best_move_chunks(&self, depth: i8, _player: &PieceColor) -> Option<Move> {
        let start = Instant::now();

        let moves = self.get_all_moves_for_current();
        let chunk_size = (moves.len() + NUM_THREADS - 1) / NUM_THREADS;

        //Same zobrist, ref dash table
        // Find Chunks Took 252 ms
        // Find Chunks Took 877 ms
        // Find Chunks Took 2900 ms
        // Find Chunks Took 4155 ms
        // Find Chunks Took 7004 ms

        //Unique zobrist, arc reference
        // Find Chunks Took 95 ms
        // Find Chunks Took 302 ms
        // Find Chunks Took 1113 ms
        // Find Chunks Took 1360 ms
        // Find Chunks Took 1676 ms

        //unique zobrist, arc ref no clone
        // Find Chunks Took 79 ms
        // Find Chunks Took 285 ms
        // Find Chunks Took 887 ms
        // Find Chunks Took 1160 ms
        // Find Chunks Took 1354 ms

        //One zobrist, arc reference
        // Find Chunks Took 89 ms
        // Find Chunks Took 277 ms
        // Find Chunks Took 843 ms
        // Find Chunks Took 1066 ms
        // Find Chunks Took 1501 ms

        //One zobrist, arc clone
        // Find Chunks Took 76 ms
        // Find Chunks Took 236 ms
        // Find Chunks Took 864 ms
        // Find Chunks Took 1376 ms
        // Find Chunks Took 1415 ms

        //TODO: no hashing is faster? Maybe need hash for more depth

        let zobrist = ZobristHash::new();

        let transpositions: Arc<TranspositionTable> = Arc::new(TranspositionTable::new());
        // let transpositions: TranspositionTable = TranspositionTable::new();

        let bests: Vec<EvalPair> = moves
            .par_chunks(chunk_size)
            .map(|chunk| {
                let mut best_move: Option<Move> = None;
                let mut best_score = f32::NEG_INFINITY;

                // let zobrist = ZobristHash::new();
                // let transpositions: TranspositionTable = TranspositionTable::new();

                for mv in chunk {
                    let new_board = self.make_move(mv);

                    let eval = new_board.alpha_beta_p(
                        depth - 1,
                        f32::NEG_INFINITY,
                        f32::INFINITY,
                        false,
                        Arc::clone(&transpositions),
                        &zobrist,
                    );
                    if eval > best_score {
                        best_score = eval;
                        best_move = Some(mv.clone());
                    }
                }

                EvalPair {
                    0: best_move.unwrap(),
                    1: best_score,
                }
            })
            .collect();

        let best_move = bests
            .iter()
            .max_by(|a, b| -> Ordering { a.1.total_cmp(&b.1) });

        let elapsed = start.elapsed();
        println!("Find Chunks Took {} ms", elapsed.as_millis());
        Some(best_move.unwrap().0.clone())
    }

    fn alpha_beta_p(
        &self,
        depth: i8,
        mut alpha: f32,
        mut beta: f32,
        maximizer: bool,
        transpositions: Arc<TranspositionTable>,
        // transpositions: &TranspositionTable,
        zobrist: &ZobristHash,
    ) -> f32 {
        // let board_hash = self.eval_hash(zobrist);
        // if let Some(entry) = transpositions.get(&board_hash) {
        //     if entry.depth >= depth {
        //         match entry.node_type {
        //             NodeType::Exact => return entry.eval,
        //             NodeType::Lower => alpha = alpha.max(entry.eval),
        //             NodeType::Upper => beta = beta.min(entry.eval),
        //         }

        //         if alpha >= beta {
        //             return entry.eval;
        //         }
        //     }
        // }

        if depth == 0 {
            let eval = self.raw_score();

            // transpositions.insert(
            //     board_hash,
            //     TranspositionEntry {
            //         eval,
            //         depth,
            //         node_type: NodeType::Exact,
            //     },
            // );
            return eval;
        }

        let mut best_eval = if maximizer {
            f32::NEG_INFINITY
        } else {
            f32::INFINITY
        };
        let mut node_type = NodeType::Exact;

        let moves = self.get_all_moves_for_current();

        for mv in &moves {
            let new_board = self.make_move(mv);
            let eval = new_board.alpha_beta_p(
                depth - 1,
                alpha,
                beta,
                !maximizer,
                transpositions.clone(),
                zobrist,
            );

            if maximizer {
                best_eval = best_eval.max(eval);
                alpha = alpha.max(eval);
                if alpha >= beta {
                    node_type = NodeType::Lower;
                    break;
                }
            } else {
                best_eval = best_eval.min(eval);
                beta = beta.min(eval);
                if beta <= alpha {
                    node_type = NodeType::Upper;
                    break;
                }
            }
        }
        // transpositions.insert(
        //     board_hash,
        //     TranspositionEntry {
        //         eval: best_eval,
        //         depth,
        //         node_type,
        //     },
        // );

        best_eval
    }

    fn eval_hash(&self, zobrist: &ZobristHash) -> u64 {
        let mut hash = 0u64;

        for (square, piece) in self.pieces.iter().enumerate() {
            if piece.type_.unwrap() == PieceType::NONE {
                continue;
            }
            hash ^= zobrist.random_table[piece.piece_index()][square];
        }
        if self.player_to_move.unwrap() == PieceColor::WHITE {
            hash ^= zobrist.side_to_move;
        }
        if self.black_castle {
            hash ^= zobrist.castling_rights[0];
        }
        if self.black_long_castle {
            hash ^= zobrist.castling_rights[1];
        }
        if self.white_castle {
            hash ^= zobrist.castling_rights[2];
        }
        if self.white_long_castle {
            hash ^= zobrist.castling_rights[3];
        }
        hash
    }
}

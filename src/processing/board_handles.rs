
use std::{cmp::Ordering, collections::HashMap, hash::{Hash, Hasher}, sync::{Arc, Mutex}, time::Instant};

use protobuf::{MessageField, Enum, EnumOrUnknown};
use rand::Rng;
use rayon::{iter::ParallelIterator, slice::ParallelSlice};

use crate::{generated::chess::*, NUM_THREADS} ;

#[derive(PartialEq)]
pub enum ReturnState {
    TRUE,
    TrueExit,
    FALSE
    
}
#[derive(Clone, Copy)]
pub struct Transpose {
    depth : i8,
    value : f32,
    // best_move : Move,
}

#[derive(Copy,Clone)]
enum NodeType {
    Exact,
    Lower,
    Upper,
}
#[derive(Copy,Clone)]
struct TranspositionEntry {
    eval : f32,
    depth: i8,
    node_type: NodeType,
}
type TranspositionTable = HashMap<u64, TranspositionEntry>;

impl Eq for Board {}

impl Hash for Board {
    fn hash<H: Hasher>(&self, state : &mut H) {
        self.pieces.hash(state);
        self.player_to_move.unwrap().hash(state);
    }
}

impl Board {
    pub fn get_piece(&self, row : i32, col : i32) -> Option<&ProtoPiece> {
        let index:usize = (row * 8 + col % 8) as usize;

        match self.pieces.get(index) {
            Some(piece) => {
                Some(piece)
            },
            None => { None }
        }
    }
    pub fn square_empty(&self, position : &Position) -> bool {
        let index:usize = (position.row * 8 + position.col % 8) as usize;

        match self.pieces.get(index) {
            Some(piece) => {

                return piece.type_.unwrap().eq(&PieceType::NONE);
            },
            None => { return false; }
        }
    }
    pub fn square_empty_grid(&self, col : i32, row : i32) -> bool {
        let index:usize = (row * 8 + col % 8) as usize;

        match self.pieces.get(index) {
            Some(piece) => {

                return piece.type_.unwrap().eq(&PieceType::NONE);
            },
            None => { return false; }
        }

    }

    pub fn validate_grid_position(&self, col : i32, row : i32, piece : &ProtoPiece) -> ReturnState {
        if Position::out_of_bounds(col, row) {
           return ReturnState::FALSE; 
        }
        if  self.square_empty_grid(col, row) {
            ReturnState::TRUE
        } else if piece.can_capture_grid(col, row, self) {
            ReturnState::TrueExit
        } else {
           return ReturnState::FALSE; 
        }
    }
    pub fn validate_position(&self, position : &Position, piece : &ProtoPiece) -> ReturnState {
        self.validate_grid_position(position.col, position.row, piece)
    }

    fn fake_hash(&self) -> i64 {
        let mut total : i64 = (31 * self.turnCount).into();
        for piece in &self.pieces {
            total += piece.fake_hash();
        }

        total
    }
    fn get_all_moves_for_current(&self) -> Vec<Move> {
        let mut all_moves : Vec<Move> = Vec::new();

        let color = self.player_to_move.unwrap();

        for piece in &self.pieces {
            if !piece.color.unwrap().eq(&color) { continue; }
            let mut this_move : Move = Move::default();
            this_move.set_piece(piece.clone());
            for move_pos in piece.get_valid_moves(&self){
                this_move.set_move(move_pos.primary_move().clone());
                if let Some(secondary) = move_pos.secondary_move() {

                    this_move.set_secondary_move(secondary.clone());
                    
                }
                all_moves.insert(0, this_move.clone());
            }
        }

        all_moves
    }

    pub fn make_move(&self, mv : &Move) -> Board {
        let mut new = self.clone();
        let curr_piece = mv.piece_to_move.as_ref().unwrap();

        if new.player_to_move.unwrap() == PieceColor::WHITE {
            new.player_to_move = EnumOrUnknown::new(PieceColor::BLACK);
        } else {
            new.player_to_move = EnumOrUnknown::new(PieceColor::WHITE);
        }

        let black_king = self.get_piece(0, 4);
        if black_king.is_some() && black_king.unwrap().is_piece(PieceColor::BLACK, PieceType::KING) {
            if new.black_long_castle {
                if let Some(long_rook) = self.get_piece(0, 0) {
                    new.black_castle = long_rook.is_piece(PieceColor::BLACK, PieceType::ROOK);
                }
            }
            if new.black_castle {
                if let Some(short_rook) = self.get_piece(0, 7) {
                    new.black_castle = short_rook.is_piece(PieceColor::BLACK, PieceType::ROOK);
                }
            }
        } else {
            new.black_castle = false;
            new.black_long_castle = false;
        }
        let white_king = self.get_piece(7, 4);
        if white_king.is_some() && white_king.unwrap().is_piece(PieceColor::WHITE, PieceType::KING) {
            if new.white_long_castle {
                if let Some(long_rook) = self.get_piece(7, 0) {
                    new.white_castle = long_rook.is_piece(PieceColor::WHITE, PieceType::ROOK);
                }
            }
            if new.white_castle {
                if let Some(short_rook) = self.get_piece(7, 7) {
                    new.white_castle = short_rook.is_piece(PieceColor::WHITE, PieceType::ROOK);
                }
            }
        } else {
            new.white_castle = false;
            new.white_long_castle = false;
        }

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

    // pub fn evaluate(&self, color : &PieceColor, all_moves : &[Move]) -> f32 {
    //     let mut white_score = 0.0;
    //     let mut black_score = 0.0;
    //     for piece in &self.pieces {
    //         if piece.type_.unwrap() == PieceType::NONE {
    //             continue;
    //         }
    //         if piece.color.unwrap() == PieceColor::WHITE {
    //             white_score += piece.get_score(self);
    //             // white_score += piece.get_score(self, all_moves);
    //         } else {
    //             black_score += piece.get_score(self);
    //             // black_score += piece.get_score(self, all_moves.as_slice());
    //         }
    //     }
    //     if color == &PieceColor::WHITE {
    //         white_score - black_score
    //     } else {
    //         black_score - white_score
    //     }
    // }


    // NEW BLOCK

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
    
    fn ab(
        &self,
        depth : i8,
        mut alpha : f32, mut beta : f32,
        maximizer : bool,
        transpositions : &mut TranspositionTable,
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
                    return entry.eval
                }
            }
        }

        if depth == 0 {
            let eval = self.raw_score();
            // return self.raw_score()

            transpositions.insert(
                board_hash,
                TranspositionEntry {eval, depth, node_type: NodeType::Exact}
            );
            return eval;

            //TODO: Hashing, zobrist and transposition table. Ordering of moves at beggining? Handle when in check
        }

        let mut best_eval = if maximizer { f32::NEG_INFINITY } else { f32::INFINITY };
        let mut node_type = NodeType::Exact;

        let moves= self.get_all_moves_for_current();

        for mv in &moves {
            let new_board = self.make_move(mv);
            let eval = new_board.ab(depth - 1, alpha, beta, !maximizer, transpositions, zobrist);

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
            TranspositionEntry {eval: best_eval, depth, node_type}
        );

        best_eval
    }

    pub fn find_best_move(&self, depth : i8, player : &PieceColor) -> Option<Move> {
        
        let start = Instant::now();

        let mut best_move: Option<Move> = None;
        let mut best_score = f32::NEG_INFINITY;

        let moves = self.get_all_moves_for_current();

        let mut transpositions: TranspositionTable = HashMap::new();
        let zobrist = ZobristHash::new();

        for mv in &moves {
            let new_board = self.make_move(mv);
            let eval = new_board.ab(
                depth - 1,
                f32::NEG_INFINITY, f32::INFINITY,
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

    pub fn find_best_move_chunks(&self, depth : i8, player : &PieceColor) -> Option<Move> {
        
        let start = Instant::now();

        let moves = self.get_all_moves_for_current();

        let chunk_size = (moves.len() + NUM_THREADS - 1) / NUM_THREADS;

        let bests: Vec<EvalPair> = moves.par_chunks(chunk_size)
            .map(|chunk| {

                let mut best_move: Option<Move> = None;
                let mut best_score = f32::NEG_INFINITY;
                let mut transpositions: TranspositionTable = HashMap::new();
                let zobrist = ZobristHash::new();

                for mv in chunk {
                    let new_board = self.make_move(mv);

                    let eval = new_board.ab(
                        depth - 1,
                        f32::NEG_INFINITY, f32::INFINITY,
                        false,
                        &mut transpositions,
                        &zobrist,
                    );
                    if eval > best_score {
                        best_score = eval;
                        best_move = Some(mv.clone());
                    }
                }

                EvalPair{0: best_move.unwrap(), 1: best_score}
            })
            .collect();

        let best_move = bests.iter()
            .max_by(|a, b| -> Ordering {
                a.1.total_cmp(&b.1)
            });

        let elapsed = start.elapsed();
        println!("Find Chunks Took {} ms", elapsed.as_millis());
        Some(best_move.unwrap().0.clone())
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
    pub fn set_piece(&mut self, piece : ProtoPiece) {
        self.piece_to_move = MessageField::some(piece);
    }
    pub fn set_move(&mut self, pos : Position) {
        self.end_position = MessageField::some(pos);
    }
    pub fn set_secondary_move(&mut self, pos : Position) {
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
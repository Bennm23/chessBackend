
use core::{num,};
use std::{collections::HashMap, hash::Hasher, hash::Hash, sync::{Arc, Mutex}, clone};

use protobuf::{MessageField, Enum, EnumOrUnknown, Message, SpecialFields, UnknownFields, rt::CachedSize};
use rand::{seq::SliceRandom, Rng};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator, IntoParallelIterator};

use crate::{generated::chess::{*}, SEARCH_DEPTH };
#[derive(PartialEq)]
pub enum ReturnState {
    TRUE,
    TRUE_EXIT,
    FALSE
    
}
#[derive(Clone, Copy)]
pub struct Transpose {
    depth : i8,
    value : f32,
    // best_move : Move,
}
impl Eq for Board {}

impl Hash for Board {
    fn hash<H: Hasher>(&self, state : &mut H) {
        self.pieces.hash(state);
        self.player_to_move.unwrap().hash(state);
    }
}

    pub fn minus(val : Arc<Mutex<f32>>) {
        let mut v = **val.lock().as_mut().unwrap();
        v = -v;
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
            ReturnState::TRUE_EXIT
        } else {
           return ReturnState::FALSE; 
        }
    }
    pub fn validate_position(&self, position : &Position, piece : &ProtoPiece) -> ReturnState {
        self.validate_grid_position(position.col, position.row, piece)
    }

    //Alpha beta pruning.
    //Alpha - the best value that the maximizer can guarantee at that level or above
    //Beta - the best value that the minimizer can guarentee at that level or above


    // pub fn apb(self, depth : i8, alpha : Arc<Mutex<f32>>, beta : Arc<Mutex<f32>> , color : &PieceColor,
    //                 maxing : bool) -> (f32,Option<Move>) {
    //     let all_moves : Vec<Move> = self.get_all_moves_for_color(color);
    //     if depth == 0 {
    //         return (self.evaluate(color, all_moves.as_slice()), None);
    //     }
    //     // let mut max_score : f32 = f32::MIN;
    //     // let mut best_move: Option<Move> = None;
    //     let max_score : Arc<Mutex<f32>> = Arc::new(Mutex::new(f32::MIN));
    //     let best_move : Arc<Mutex<Option<Move>>> = Arc::new(Mutex::new(None));

    //     let stream = all_moves.into_par_iter()
    //             .map(|mv| {

    //                 let new_board = self.make_move(&mv);
                    
    //                 let (res, _) = new_board.apb(depth -1, alpha , beta, &color.opposite(), !maxing);

    //                 if maxing {
    //                     let mut max_clone = 0.0;
    //                     {
    //                         let mut new_max = max_score.lock().as_mut().unwrap();
    //                         if **new_max < -res {
    //                             **new_max = -res;
    //                             let mut bm = best_move.lock().as_mut().unwrap();
    //                             **bm = Some(mv);

    //                         }
    //                             max_clone = new_max.clone();
    //                     }
    //                     {
    //                         let mut new_alpha = alpha.lock().as_mut().unwrap();
    //                         **new_alpha = new_alpha.max(max_clone);
    //                         if **beta.lock().as_ref().unwrap() <= **new_alpha {
    //                             return;
    //                         }
    //                     }
    //                 } else {
    //                     let mut max_clone = 0.0;
    //                     {
    //                         let mut new_max = max_score.lock().as_mut().unwrap();
    //                         if **new_max < -res {
    //                             **new_max = -res;
    //                             let mut bm = best_move.lock().as_mut().unwrap();
    //                             **bm = Some(mv);

    //                         }
    //                             max_clone = new_max.clone();
    //                     }
    //                     {
    //                         let mut new_beta = beta.lock().as_mut().unwrap();
    //                         **new_beta = new_beta.max(max_clone);
    //                         if **alpha.lock().as_ref().unwrap() >= **new_beta {
    //                             return;
    //                         }
    //                     }

    //                 }


    //             });
        

    //     return (0.0, None)
    // }

    pub fn alpha_beta(self, depth : i8,
                      mut alpha : f32, beta : f32, color : &PieceColor,
                      board_map : &mut HashMap<Board, Transpose>
                ) -> (f32, Option<Move>) {

        let transpose = board_map.get(&self);
        if let Some(transpose) = transpose {
            if transpose.depth >= depth {
                return (transpose.value, None);
            }
        }

        //All available moves at this depth
        let all_moves:Vec<Move> = self.get_all_moves_for_color(color);
        //If at lowest level of evaluation, just evaluate board
        if depth == 0 {
            return (self.evaluate(color, all_moves.as_slice()), None);
        }

        let mut max_score : f32 = f32::MIN;
        let mut best_move: Option<Move> = None;
        
        for mv in all_moves {
            //Make the move
            let new_board = self.make_move(&mv);
            let (res,_) = new_board.alpha_beta(depth - 1, -beta, -alpha, &color.opposite(), board_map);

            //res will return the best score for the opposite player
            //if max_score is less than the opposite of the best move for the other player,
            //we found a better move
            if max_score < -res {
                max_score = -res;
                best_move = Some(mv);
            }
            alpha = alpha.max(max_score);
            //alpha is the best score found for this player so far
            //If the worst result for us is less than the best result we've found, break because the opponent will choose the other
            if beta <= alpha {
                break;
            }
        }

        let entry = Transpose {
            depth,
            value: max_score,
            // best_move : best_move
        };
        board_map.insert(self, entry);

        if depth == SEARCH_DEPTH {
            (max_score, best_move)
        } else {
            (max_score, None)
        }
    }

    fn fake_hash(&self) -> i64 {
        let mut total : i64 = (31 * self.turnCount).into();
        for piece in &self.pieces {
            total += piece.fake_hash();
        }

        total
    }

    // pub fn alpha_beta_parallel(
    //                   &self, depth : i8,
    //                   mut alpha : f32, beta : f32, color : &PieceColor,
    //                 //   board_map : &mut HashMap<Board, Transpose>, num_threads : usize
    //                   board_map : &Arc<Mutex<&mut HashMap<i64, Transpose>>>
    //             ) -> (f32, Option<Move>) {

    //     let binding = board_map.lock().unwrap();
    //     let transpose = binding.get(&self.clone().fake_hash());
    //     if let Some(transpose) = transpose {
    //         if transpose.depth >= depth {
    //             return (transpose.value, None);
    //         }
    //     }

    //     //All available moves at this depth
    //     let all_moves:Vec<Move> = self.get_all_moves_for_color(color);
    //     //If at lowest level of evaluation, just evaluate board
    //     if depth == 0 {
    //         return (self.evaluate(color, all_moves.as_slice()), None);
    //     }

    //     // let chunks = all_moves.chunks(all_moves.len() / num_threads + 1);

    //     let best_move : Arc<Mutex<Option<Move>>> = Arc::new(Mutex::new(None));
    //     let max_score : Arc<Mutex<f32>> = Arc::new(Mutex::new(f32::MIN));
    //     let alpha_arc : Arc<Mutex<f32>> = Arc::new(Mutex::new(alpha));
    //     let beta_arc  : Arc<Mutex<f32>> = Arc::new(Mutex::new(beta));
    //     // let trans_table = Arc::new(Mutex::new(board_map));
        
    //     all_moves.par_iter().for_each(|mv| {
    //         let new_board = self.make_move(mv);
    //         let (res, _) = new_board.alpha_beta_parallel(depth - 1, -beta, -alpha, &color.opposite(), &board_map);

    //         let mut max= max_score.lock().unwrap();
    //         let mut best= best_move.lock().unwrap();
    //         let mut alpha = alpha_arc.lock().unwrap();
    //         let mut be = beta_arc.lock().unwrap();
    //         if *max < -res {
    //             *max = -res;
    //             *best= Some(mv.clone());
    //         }
    //         *alpha= alpha.max(*max_score);
    //         //alpha is the best score found for this player so far
    //         //If the worst result for us is less than the best result we've found, break because the opponent will choose the other
    //         if *beta <= *alpha {
    //             return;
    //         }
    //     });

    //     let entry = Transpose {
    //         depth,
    //         value: *max_score.lock().unwrap(),
    //         // best_move : best_move
    //     };
    //     board_map.lock().unwrap().insert(self.fake_hash(), entry);

    //     if depth == SEARCH_DEPTH {
    //         (*max_score.lock().unwrap(), best_move.lock().unwrap().clone())
    //     } else {
    //         (*max_score.lock().unwrap(), None)
    //     }
    // }

    pub fn get_best_move(&mut self, color : &PieceColor) -> Move {
        let all_moves:Vec<Move> = self.get_all_moves_for_color(color);

        let mut best_move = Move::default();
        let mut max = f32::MIN;
        for mv in all_moves {
            let new_board = self.make_move(&mv);
            // let score = new_board.evaluate(color);
            let score = -1.0 * new_board.get_best_move_response(&PieceColor::from_i32((color.value() + 1) % 2).unwrap(),1);
            if score > max {
                max = score;
                // println!("MAX = {max}");
                // println!("MOVE = {:#?}", mv);
                best_move = mv;
            }
        }
        best_move.clone()
        // all_moves.choose(&mut rand::thread_rng()).unwrap().choose(&mut rand::thread_rng()).unwrap().clone()
    }
    fn get_best_move_response(&self, color : &PieceColor, depth : i8) -> f32 {
        let all_moves:Vec<Move> = self.get_all_moves_for_color(color);

        let mut max = f32::MIN;
        for mv in &all_moves {
            let new_board = self.make_move(&mv);
            let mut score = 0.0;
            if depth == 1 {
                score = -1.0 * new_board.get_best_move_response(&PieceColor::from_i32((color.value() + 1) % 2).unwrap(),2);
            } else {
                score = new_board.evaluate(color, all_moves.as_slice());
            }
            if score > max {
                max = score;
                // println!("MAX = {max}");
            }
        }
        max
    }

    fn get_all_moves_for_color(&self, color : &PieceColor) -> Vec<Move> {
        let mut all_moves : Vec<Move> = Vec::new();

        for piece in &self.pieces {
            if !piece.color.unwrap().eq(color) { continue; }
            let mut this_move : Move = Move::default();
            this_move.set_piece(piece.clone());
            for end_pos in piece.get_valid_moves(&self){
                this_move.set_move(end_pos.clone());
                all_moves.insert(0, this_move.clone());
            }
        }

        all_moves
    }

    fn make_move(&self, mv : &Move) -> Board {
        let mut new = self.clone();
        let curr_piece = mv.piece_to_move.as_ref().unwrap();

        let old_index = (&curr_piece.row * 8 + &curr_piece.col % 8) as usize;
        let new_index = (&mv.end_position.row * 8 + &mv.end_position.col % 8) as usize;
        // println!("OLD INDEX VAL = {old_index}");
        // println!("NEW INDEX VAL = {new_index}");
        //Update destination with old position values 
        new.pieces[old_index].col = mv.end_position.col;
        new.pieces[old_index].row = mv.end_position.row;

        // println!("OLD INDEX = {:#?}",new.pieces[old_index]);
        //Update the new piece with old position values
        new.pieces[new_index].col = curr_piece.col;
        new.pieces[new_index].row = curr_piece.row;
        new.pieces[new_index].type_ = EnumOrUnknown::from_i32(0);
        // println!("NEW INDEX = {:#?}",new.pieces[new_index]);
        //Move old to new pos
        new.pieces.swap(old_index, new_index);

        new
    }

    fn evaluate(&self, color : &PieceColor, all_moves : &[Move]) -> f32 {
        let mut white_score = 0.0;
        let mut black_score = 0.0;
        for piece in &self.pieces {
            if piece.type_.unwrap() == PieceType::NONE {
                continue;
            }
            if piece.color.unwrap() == PieceColor::WHITE {
                white_score += piece.get_score(self, all_moves);
                // white_score += piece.get_score(self, all_moves);
            } else {
                black_score += piece.get_score(self, all_moves);
                // black_score += piece.get_score(self, all_moves.as_slice());
            }
        }
        if color == &PieceColor::WHITE {
            white_score - black_score
        } else {
            black_score - white_score
        }
    }
}

impl Move {
    pub fn set_piece(&mut self, piece : ProtoPiece) {
        self.piece_to_move = MessageField::some(piece.clone());
    }
    pub fn set_move(&mut self, pos : Position) {
        self.end_position = MessageField::some(pos.clone());
    }
}

impl PieceColor {
    pub fn opposite(&self) -> PieceColor {
        PieceColor::from_i32((self.value() + 1) % 2).unwrap()
    }
}

// impl Hash for PieceColor {
//     fn hash<H: Hasher>(&self, state : &mut H) {
//         self.value().hash(state);
//     }
// }
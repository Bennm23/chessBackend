use std::hash::{Hash, Hasher};

use protobuf::Enum;

use crate::generated::chess::{*};

use crate::processing::board_handles::{*};

impl Hash for ProtoPiece {
    fn hash<H: Hasher>(&self, state : &mut H) {
        self.col.hash(state);
        self.row.hash(state);
        // self.color.hash(state);
    }
}

impl ProtoPiece {
    pub fn can_capture(&self, other : &ProtoPiece) -> bool {
        !other.type_.unwrap().eq(&PieceType::NONE) && !self.color.unwrap().eq(&other.color.unwrap())
    }
    pub fn fake_hash(&self) -> i64 {
        (17 * self.col + 31 * self.row + 37 * self.color.value() + 11 * self.type_.unwrap().value()).into() 
    }
    pub fn can_capture_position(&self, position : &Position, board : &Board) -> bool {
        match board.get_piece(position.row, position.col) {
            Some(piece) => {
                self.can_capture(piece)
            },
            None => { false }
        }
    }

    pub fn can_capture_grid(&self, col : i32, row : i32, board : &Board) -> bool {
        match board.get_piece(row, col) {
            Some(piece) => {
                self.can_capture(piece)
            },
            None => { false }
        }
    }

    pub fn get_valid_moves(&self, board : &Board) -> Vec<Position> {
        let mut color_multiple = -1;
        if self.color.unwrap().eq(&PieceColor::BLACK) {
            color_multiple = 1;
        }

        let mut moves: Vec<Position> = Vec::new();

        let mut position : Position = Position::new();
        position.row = self.row;
        position.col = self.col;
        match self.type_.unwrap() {
            PieceType::PAWN => {
                position.row = self.row + color_multiple * 1;
                position.col = self.col;
                if board.square_empty( &position) {
                    moves.insert(0, position.clone());
                }
                position.col = self.col + 1;
                if self.col != 7 && self.can_capture_position(&position, &board){
                    moves.insert(0, position.clone());
                }
                position.col = self.col - 1;
                if self.col != 0 && self.can_capture_position(&position, &board){
                    moves.insert(0, position.clone());
                }
                if self.row == 1 || self.row == 6 {
                    position.col = self.col;
                    position.row = self.row + color_multiple * 2;
                    let mut in_between = Position::new();
                    in_between.col = self.col;
                    in_between.row = self.row + color_multiple;
                    if board.square_empty( &position) && board.square_empty(&in_between){
                        moves.insert(0, position.clone());
                    }
                }
            },
            PieceType::ROOK => {
                moves.append(&mut self.get_horizontal_moves(&board, &position));
            },
            PieceType::BISHOP => {
                moves.append(&mut self.get_diagonal_moves(&board, &position));
            },
            PieceType::QUEEN => {
                moves.append(&mut self.get_horizontal_moves(&board, &position));
                moves.append(&mut self.get_diagonal_moves(&board, &position));
            },
            PieceType::KNIGHT => {
                let mut new_pos = position.move_to(2, -1); //up 1 right 2
                if board.validate_position(&new_pos, self) != ReturnState::FALSE  {
                    moves.insert(0, new_pos.clone());
                }
                new_pos = position.move_to(-2, -1); //up 1 left 2
                if board.validate_position(&new_pos, self) != ReturnState::FALSE  {
                    moves.insert(0, new_pos.clone());
                }
                new_pos = position.move_to(2, 1); //down 1 right 2
                if board.validate_position(&new_pos, self) != ReturnState::FALSE  {
                    moves.insert(0, new_pos.clone());
                }
                new_pos = position.move_to(-2, 1); //down 1 left 2
                if board.validate_position(&new_pos, self) != ReturnState::FALSE  {
                    moves.insert(0, new_pos.clone());
                }

                new_pos = position.move_to(1, -2); 
                if board.validate_position(&new_pos, self) != ReturnState::FALSE {
                    moves.insert(0, new_pos.clone());
                }
                new_pos = position.move_to(-1, -2);
                if board.validate_position(&new_pos, self) != ReturnState::FALSE{
                    moves.insert(0, new_pos.clone());
                }
                new_pos = position.move_to(1, 2);
                if board.validate_position(&new_pos, self) != ReturnState::FALSE {
                    moves.insert(0, new_pos.clone());
                }
                new_pos = position.move_to(-1, 2);
                if board.validate_position(&new_pos, self) != ReturnState::FALSE {
                    moves.insert(0, new_pos.clone());
                }
            },
            PieceType::KING => {
                //TODO: ADD CASTLING
                let mut new_pos = position.move_to(0, 0); 
                for col in -1 .. 2 {
                   for row in -1 .. 2 {
                        if col == 0 && row == 0 { continue; }
                        new_pos = position.move_to(col, row); 
                        if board.validate_position(&new_pos, self) != ReturnState::FALSE {
                            moves.insert(0, new_pos.clone());
                        }
                   } 
                }
            }
            _ => {

            }
        }
        moves
    }

    fn get_valid_moves_in_direction<F:Fn(i32, i32) -> (i32,i32)>(&self, board : &Board, start_position : &Position, f:F) -> Vec<Position> {
        let mut curr_col = start_position.col;
        let mut curr_row = start_position.row;
        let mut moves: Vec<Position> = Vec::new();

        let mut _result = ReturnState::FALSE;
        loop {
            (curr_col, curr_row) = f(curr_col, curr_row);
            _result = board.validate_grid_position(curr_col, curr_row, self);
            if _result == ReturnState::TRUE{
                moves.insert(0, Position::from_grid(curr_col, curr_row));
            } else if _result == ReturnState::TRUE_EXIT{
                moves.insert(0, Position::from_grid(curr_col, curr_row));
                break;
            } else {
                break;
            }
        }
        moves
    }

    //HORIZONTAL MOVES
    fn get_horizontal_moves(&self, board : &Board, start_position : &Position) -> Vec<Position> {
        let mut moves = Vec::new();
        moves.append(&mut self.get_valid_moves_right(&board, start_position));
        moves.append(&mut self.get_valid_moves_left(&board, start_position));
        moves.append(&mut self.get_valid_moves_up(&board, start_position));
        moves.append(&mut self.get_valid_moves_down(&board, start_position));
        moves
    }
    fn get_valid_moves_right(&self, board : &Board, start_position : &Position) -> Vec<Position> {
        self.get_valid_moves_in_direction(&board, &start_position, |col : i32, row : i32| -> (i32,i32) {
            let c = col + 1;
            (c, row)
        })
    }

    fn get_valid_moves_left(&self, board : &Board, start_position : &Position) -> Vec<Position> {
        self.get_valid_moves_in_direction(&board, &start_position, |col : i32, row : i32| -> (i32,i32) {
            let c = col - 1;
            (c, row)
        })
    }
    fn get_valid_moves_up(&self, board : &Board, start_position : &Position) -> Vec<Position> {
        self.get_valid_moves_in_direction(&board, &start_position, |col : i32, row : i32| -> (i32,i32) {
            let r= row - 1;
            (col, r)
        })
    }
    fn get_valid_moves_down(&self, board : &Board, start_position : &Position) -> Vec<Position> {
        self.get_valid_moves_in_direction(&board, &start_position, |col : i32, row : i32| -> (i32,i32) {
            let r= row + 1;
            (col, r)
        })
    }
    //DIAGONAL MOVES
    fn get_diagonal_moves(&self, board : &Board, start_position : &Position) -> Vec<Position> {
        let mut moves = Vec::new();
        moves.append(&mut self.get_valid_moves_right_up(&board, start_position));
        moves.append(&mut self.get_valid_moves_right_down(&board, start_position));
        moves.append(&mut self.get_valid_moves_left_up(&board, start_position));
        moves.append(&mut self.get_valid_moves_left_down(&board, start_position));
        moves
    }
    fn get_valid_moves_right_down(&self, board : &Board, start_position : &Position) -> Vec<Position> {
        self.get_valid_moves_in_direction(&board, &start_position, |col : i32, row : i32| -> (i32,i32) {
            let c = col + 1;
            let r= row + 1;
            (c, r)
        })
    }
    fn get_valid_moves_right_up(&self, board : &Board, start_position : &Position) -> Vec<Position> {
        self.get_valid_moves_in_direction(&board, &start_position, |col : i32, row : i32| -> (i32,i32) {
            let c = col + 1;
            let r= row - 1;
            (c, r)
        })
    }

    fn get_valid_moves_left_down(&self, board : &Board, start_position : &Position) -> Vec<Position> {
        self.get_valid_moves_in_direction(&board, &start_position, |col : i32, row : i32| -> (i32,i32) {
            let c = col - 1;
            let r= row + 1;
            (c, r)
        })
    }
    fn get_valid_moves_left_up(&self, board : &Board, start_position : &Position) -> Vec<Position> {
        self.get_valid_moves_in_direction(&board, &start_position, |col : i32, row : i32| -> (i32,i32) {
            let c = col - 1;
            let r= row - 1;
            (c, r)
        })
    }

    fn get_empty_squares_in_direction<F:Fn(i32, i32) -> (i32,i32)>(&self, board : &Board, start_position : &Position, f:F) -> f32 {
        let mut curr_col = start_position.col;
        let mut curr_row = start_position.row;
        let mut score = 0.0;

        let mut _result = ReturnState::FALSE;
        loop {
            (curr_col, curr_row) = f(curr_col, curr_row);
            _result = board.validate_grid_position(curr_col, curr_row, self);
            if _result == ReturnState::TRUE{
                score += 0.01;
            } else {
                break;
            }
        }
        score
    }
    fn get_empty_squares_left(&self, board : &Board, start_position : &Position) -> f32 {
        self.get_empty_squares_in_direction(board, start_position, |col : i32, row : i32| -> (i32,i32) {
            (col - 1, row)
        })
    }
    fn get_empty_squares_right(&self, board : &Board, start_position : &Position) -> f32 {
        self.get_empty_squares_in_direction(board, start_position, |col : i32, row : i32| -> (i32,i32) {
            (col + 1, row)
        })
    }
    fn get_empty_squares_down(&self, board : &Board, start_position : &Position) -> f32 {
        self.get_empty_squares_in_direction(board, start_position, |col : i32, row : i32| -> (i32,i32) {
            (col, row + 1)
        })
    }
    fn get_empty_squares_up(&self, board : &Board, start_position : &Position) -> f32 {
        self.get_empty_squares_in_direction(board, start_position, |col : i32, row : i32| -> (i32,i32) {
            (col, row - 1)
        })
    }
    fn get_empty_squares_left_up(&self, board : &Board, start_position : &Position) -> f32 {
        self.get_empty_squares_in_direction(board, start_position, |col : i32, row : i32| -> (i32,i32) {
            (col - 1, row - 1)
        })
    }
    fn get_empty_squares_right_up(&self, board : &Board, start_position : &Position) -> f32 {
        self.get_empty_squares_in_direction(board, start_position, |col : i32, row : i32| -> (i32,i32) {
            (col + 1, row - 1)
        })
    }
    fn get_empty_squares_down_left(&self, board : &Board, start_position : &Position) -> f32 {
        self.get_empty_squares_in_direction(board, start_position, |col : i32, row : i32| -> (i32,i32) {
            (col - 1, row + 1)
        })
    }
    fn get_empty_squares_up_right(&self, board : &Board, start_position : &Position) -> f32 {
        self.get_empty_squares_in_direction(board, start_position, |col : i32, row : i32| -> (i32,i32) {
            (col + 1, row - 1)
        })
    }

    //EVALUATERS
    pub fn get_score(&self, board : &Board, all_moves : &[Move]) -> f32 {
        let mut score = 0.0;
        match self.type_.unwrap() {
            PieceType::PAWN => {
                score += self.score_pawn(board);
            },
            PieceType::KING => {
                score += self.score_king(board);
            },
            PieceType::KNIGHT => {
                score += self.score_knight(board);
            }
            PieceType::ROOK => {
                score += self.score_rook(board);
            }
            PieceType::BISHOP => {
                score += self.score_bishop(board);
            }
            PieceType::QUEEN => {
                score += self.score_queen(board);
            }
            kind => {
                score += kind.value() as f32;
            }
        }
        //+ 0.1 for each defender divided by the piece value. Defenders are should matter less for important pieces
        score += self.count_defenders(all_moves) / self.type_.unwrap().value() as f32;
        if !self.has_moved() {
            score -= 0.2
        }

        score
    }

    fn has_moved(&self) -> bool {
        match (self.color.unwrap(), self.type_.unwrap()) {
            (PieceColor::BLACK, PieceType::PAWN) => {
                self.row != 1
            },
            (PieceColor::WHITE, PieceType::PAWN) => {
                self.row != 6
            },
            (PieceColor::BLACK, _) => {
                self.row != 0
            },
            (PieceColor::WHITE, _) => {
                self.row != 7
            }
        }
    }

    fn count_defenders(&self, all_moves : &[Move]) -> f32 {
        //Start with minus one because this piece will be one of the end positions
        let mut defenders = -0.05;
        for mv in all_moves {
            if mv.end_position.col == self.col && mv.end_position.row == self.row {
                defenders += 0.05;
            }
        }
        defenders
    }

    fn score_pawn(&self, board : &Board) -> f32 {
        1.0 + self.distance_from_center_score(board)
    }
    fn distance_from_center_score(&self, board : &Board) -> f32 {
        let distance = ((self.col as f32 - 3.5).powi(2) + (self.row as f32 - 3.5)).powi(2).sqrt();
        //5 is maximum value for distance, the closer we are to 0 the better
        (distance - 5.0).abs() / 100.0
    }
    fn score_knight(&self, board : &Board) -> f32 {
        3.0 + self.distance_from_center_score(board)
    }

    fn score_rook(&self, board : &Board) -> f32 {

        5.0 + self.score_horizontal_vision(board)
    }
    fn score_queen(&self, board : &Board) -> f32 {
        9.0 + self.score_diagonal_vision(board) + self.score_horizontal_vision(board)
    }
    fn score_bishop(&self, board : &Board) -> f32 {
        3.0 + self.score_diagonal_vision(board)
    }
    fn score_horizontal_vision(&self, board : &Board) -> f32 {
        let mut start_position = Position::new();
        start_position.row = self.row;
        start_position.col = self.col;

        self.get_empty_squares_down(board, &start_position) +
        self.get_empty_squares_up(board, &start_position) +
        self.get_empty_squares_left(board, &start_position) + 
        self.get_empty_squares_right(board, &start_position)
    }
    fn score_diagonal_vision(&self, board : &Board) -> f32 {
        let mut start_position = Position::new();
        start_position.row = self.row;
        start_position.col = self.col;

        self.get_empty_squares_down_left(board, &start_position) +
        self.get_empty_squares_up_right(board, &start_position) +
        self.get_empty_squares_left_up(board, &start_position) + 
        self.get_empty_squares_right_up(board, &start_position)
    }

    fn score_king(&self, board : &Board) -> f32 {
        //how safe is the king
        //we want the kings lines of sight to be guarded
        let mut distance: f32= 0.0;
        if self.color.unwrap() == PieceColor::BLACK {
            distance = ((self.col as f32 - 3.5).powi(2) + (self.row as f32) - 7.0).powi(2).sqrt();
            
        } else {
            distance = ((self.col as f32 - 3.5).powi(2) + (self.row as f32)).powi(2).sqrt();
        }
        //closer to edge and closer to back row is best
        //the farther we are from the center/opposite row the better
        let score = distance / 100.0;
        100.0 + score
    }
}
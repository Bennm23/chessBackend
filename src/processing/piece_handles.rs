use protobuf::Enum;

use crate::generated::chess::*;

use crate::processing::board_handles::*;

#[derive(Clone)]
pub struct PositionPair(Position, Option<Position>);

impl PositionPair {
    pub fn new(pos: Position, secondary: Option<Position>) -> Self {
        Self {
            0: pos,
            1: secondary,
        }
    }
    pub fn primary_move(&self) -> &Position {
        &self.0
    }
    pub fn secondary_move(&self) -> &Option<Position> {
        &self.1
    }
}

impl ProtoPiece {
    pub fn can_capture(&self, other: &ProtoPiece) -> bool {
        !other.type_.unwrap().eq(&PieceType::NONE) && !self.color.unwrap().eq(&other.color.unwrap())
    }
    pub fn can_capture_position(&self, position: &Position, board: &Board) -> bool {
        self.can_capture_grid(position.col, position.row, board)
    }
    pub fn can_capture_grid(&self, col: i32, row: i32, board: &Board) -> bool {
        match board.get_piece(row, col) {
            Some(piece) => self.can_capture(piece),
            None => false,
        }
    }

    pub fn is_piece(&self, color: PieceColor, kind: PieceType) -> bool {
        self.color.unwrap() == color && self.type_.unwrap() == kind
    }

    pub fn get_valid_moves(&self, board: &Board) -> Vec<PositionPair> {
        let mut color_multiple = -1;
        if self.color.unwrap().eq(&PieceColor::BLACK) {
            color_multiple = 1;
        }

        let mut moves: Vec<PositionPair> = Vec::new();

        let mut position: Position = Position::new();
        position.row = self.row;
        position.col = self.col;
        match self.type_.unwrap() {
            PieceType::PAWN => {
                position.row = self.row + color_multiple * 1;
                position.col = self.col;
                if board.square_empty(&position) {
                    moves.push(PositionPair::new(position.clone(), None));
                }
                position.col = self.col + 1;
                if self.col != 7 && self.can_capture_position(&position, &board) {
                    moves.push(PositionPair::new(position.clone(), None));
                }
                position.col = self.col - 1;
                if self.col != 0 && self.can_capture_position(&position, &board) {
                    moves.push(PositionPair::new(position.clone(), None));
                }
                if self.row == 1 || self.row == 6 {
                    position.col = self.col;
                    position.row = self.row + color_multiple * 2;
                    let mut in_between = Position::new();
                    in_between.col = self.col;
                    in_between.row = self.row + color_multiple;
                    if board.square_empty(&position) && board.square_empty(&in_between) {
                        moves.push(PositionPair::new(position.clone(), None));
                    }
                }
                //TODO: En Passent
            }
            PieceType::ROOK => {
                moves.append(&mut self.get_horizontal_moves(&board, &position));
            }
            PieceType::BISHOP => {
                moves.append(&mut self.get_diagonal_moves(&board, &position));
            }
            PieceType::QUEEN => {
                moves.append(&mut self.get_horizontal_moves(&board, &position));
                moves.append(&mut self.get_diagonal_moves(&board, &position));
            }
            PieceType::KNIGHT => {
                let mut new_pos = position.move_to(2, -1); //up 1 right 2
                if board.validate_position(&new_pos, self) != ReturnState::False {
                    moves.push(PositionPair::new(new_pos.clone(), None));
                }
                new_pos = position.move_to(-2, -1); //up 1 left 2
                if board.validate_position(&new_pos, self) != ReturnState::False {
                    moves.push(PositionPair::new(new_pos.clone(), None));
                }
                new_pos = position.move_to(2, 1); //down 1 right 2
                if board.validate_position(&new_pos, self) != ReturnState::False {
                    moves.push(PositionPair::new(new_pos.clone(), None));
                }
                new_pos = position.move_to(-2, 1); //down 1 left 2
                if board.validate_position(&new_pos, self) != ReturnState::False {
                    moves.push(PositionPair::new(new_pos.clone(), None));
                }

                new_pos = position.move_to(1, -2);
                if board.validate_position(&new_pos, self) != ReturnState::False {
                    moves.push(PositionPair::new(new_pos.clone(), None));
                }
                new_pos = position.move_to(-1, -2);
                if board.validate_position(&new_pos, self) != ReturnState::False {
                    moves.push(PositionPair::new(new_pos.clone(), None));
                }
                new_pos = position.move_to(1, 2);
                if board.validate_position(&new_pos, self) != ReturnState::False {
                    moves.push(PositionPair::new(new_pos.clone(), None));
                }
                new_pos = position.move_to(-1, 2);
                if board.validate_position(&new_pos, self) != ReturnState::False {
                    moves.push(PositionPair::new(new_pos.clone(), None));
                }
            }
            PieceType::KING => {
                let mut new_pos: Position;
                for col in -1..=1 {
                    for row in -1..=1 {
                        if col == 0 && row == 0 {
                            continue;
                        }
                        new_pos = position.move_to(col, row);
                        if board.validate_position(&new_pos, self) != ReturnState::False {
                            // moves.insert(0, new_pos.clone());
                            moves.push(PositionPair::new(new_pos.clone(), None));
                        }
                    }
                }

                if color_multiple == 1 {
                    //BLACK Moving
                    if board.black_long_castle {
                        let mut can_do_it = true;
                        for i in 1..=3 {
                            if !board.square_empty_grid(i, 0) {
                                can_do_it = false;
                                break;
                            }
                        }
                        if can_do_it {
                            let king_pos = Position::from_grid(2, 0);
                            let long_rook = Position::from_grid(3, 0);
                            moves.push(PositionPair::new(king_pos, Some(long_rook)));
                        }
                    }
                    if board.black_castle {
                        let mut can_do_it = true;
                        for i in 5..=6 {
                            if !board.square_empty_grid(i, 0) {
                                can_do_it = false;
                                break;
                            }
                        }
                        if can_do_it {
                            let king_pos = Position::from_grid(6, 0);
                            let short_rook = Position::from_grid(5, 0);
                            moves.push(PositionPair::new(king_pos, Some(short_rook)));
                        }
                    }
                } else {
                    if board.white_long_castle {
                        let mut can_do_it = true;
                        for i in 1..=3 {
                            if !board.square_empty_grid(i, 7) {
                                can_do_it = false;
                                break;
                            }
                        }
                        if can_do_it {
                            let king_pos = Position::from_grid(2, 7);
                            let long_rook = Position::from_grid(3, 7);
                            moves.push(PositionPair::new(king_pos, Some(long_rook)));
                        }
                    }
                    if board.white_castle {
                        let mut can_do_it = true;
                        for i in 5..=6 {
                            if !board.square_empty_grid(i, 7) {
                                can_do_it = false;
                                break;
                            }
                        }
                        if can_do_it {
                            let king_pos = Position::from_grid(6, 7);
                            let short_rook = Position::from_grid(5, 7);
                            moves.push(PositionPair::new(king_pos, Some(short_rook)));
                        }
                    }
                }
            }
            _ => {}
        }
        moves
    }

    fn get_valid_moves_in_direction<F: Fn(i32, i32) -> (i32, i32)>(
        &self,
        board: &Board,
        start_position: &Position,
        f: F,
    ) -> Vec<Position> {
        let mut curr_col = start_position.col;
        let mut curr_row = start_position.row;
        let mut moves: Vec<Position> = Vec::new();

        let mut _result = ReturnState::False;
        loop {
            (curr_col, curr_row) = f(curr_col, curr_row);
            _result = board.validate_grid_position(curr_col, curr_row, self);
            if _result == ReturnState::True {
                moves.insert(0, Position::from_grid(curr_col, curr_row));
            } else if _result == ReturnState::TrueExit {
                moves.insert(0, Position::from_grid(curr_col, curr_row));
                break;
            } else {
                break;
            }
        }
        moves
    }

    fn get_horizontal_moves(&self, board: &Board, start_position: &Position) -> Vec<PositionPair> {
        let mut moves = Vec::new();
        moves.append(&mut self.get_valid_moves_right(&board, start_position));
        moves.append(&mut self.get_valid_moves_left(&board, start_position));
        moves.append(&mut self.get_valid_moves_up(&board, start_position));
        moves.append(&mut self.get_valid_moves_down(&board, start_position));
        moves
            .iter()
            .map(|m| PositionPair::new(m.clone(), None))
            .collect()
    }
    fn get_valid_moves_right(&self, board: &Board, start_position: &Position) -> Vec<Position> {
        self.get_valid_moves_in_direction(
            &board,
            &start_position,
            |col: i32, row: i32| -> (i32, i32) {
                let c = col + 1;
                (c, row)
            },
        )
    }
    fn get_valid_moves_left(&self, board: &Board, start_position: &Position) -> Vec<Position> {
        self.get_valid_moves_in_direction(
            &board,
            &start_position,
            |col: i32, row: i32| -> (i32, i32) {
                let c = col - 1;
                (c, row)
            },
        )
    }
    fn get_valid_moves_up(&self, board: &Board, start_position: &Position) -> Vec<Position> {
        self.get_valid_moves_in_direction(
            &board,
            &start_position,
            |col: i32, row: i32| -> (i32, i32) {
                let r = row - 1;
                (col, r)
            },
        )
    }
    fn get_valid_moves_down(&self, board: &Board, start_position: &Position) -> Vec<Position> {
        self.get_valid_moves_in_direction(
            &board,
            &start_position,
            |col: i32, row: i32| -> (i32, i32) {
                let r = row + 1;
                (col, r)
            },
        )
    }
    //DIAGONAL MOVES
    fn get_diagonal_moves(&self, board: &Board, start_position: &Position) -> Vec<PositionPair> {
        let mut moves = Vec::new();
        moves.append(&mut self.get_valid_moves_right_up(&board, start_position));
        moves.append(&mut self.get_valid_moves_right_down(&board, start_position));
        moves.append(&mut self.get_valid_moves_left_up(&board, start_position));
        moves.append(&mut self.get_valid_moves_left_down(&board, start_position));
        moves
            .iter()
            .map(|m| PositionPair::new(m.clone(), None))
            .collect()
    }
    fn get_valid_moves_right_down(
        &self,
        board: &Board,
        start_position: &Position,
    ) -> Vec<Position> {
        self.get_valid_moves_in_direction(
            &board,
            &start_position,
            |col: i32, row: i32| -> (i32, i32) {
                let c = col + 1;
                let r = row + 1;
                (c, r)
            },
        )
    }
    fn get_valid_moves_right_up(&self, board: &Board, start_position: &Position) -> Vec<Position> {
        self.get_valid_moves_in_direction(
            &board,
            &start_position,
            |col: i32, row: i32| -> (i32, i32) {
                let c = col + 1;
                let r = row - 1;
                (c, r)
            },
        )
    }
    fn get_valid_moves_left_down(&self, board: &Board, start_position: &Position) -> Vec<Position> {
        self.get_valid_moves_in_direction(
            &board,
            &start_position,
            |col: i32, row: i32| -> (i32, i32) {
                let c = col - 1;
                let r = row + 1;
                (c, r)
            },
        )
    }
    fn get_valid_moves_left_up(&self, board: &Board, start_position: &Position) -> Vec<Position> {
        self.get_valid_moves_in_direction(
            &board,
            &start_position,
            |col: i32, row: i32| -> (i32, i32) {
                let c = col - 1;
                let r = row - 1;
                (c, r)
            },
        )
    }

    fn get_empty_squares_in_direction<F: Fn(i32, i32) -> (i32, i32)>(
        &self,
        board: &Board,
        start_position: &Position,
        f: F,
    ) -> f32 {
        let mut curr_col = start_position.col;
        let mut curr_row = start_position.row;
        let mut score = 0.0;

        let mut _result = ReturnState::False;
        loop {
            (curr_col, curr_row) = f(curr_col, curr_row);
            _result = board.validate_grid_position(curr_col, curr_row, self);
            if _result == ReturnState::True {
                score += 0.1;
            } else {
                break;
            }
        }
        score
    }
    fn get_empty_squares_left(&self, board: &Board, start_position: &Position) -> f32 {
        self.get_empty_squares_in_direction(
            board,
            start_position,
            |col: i32, row: i32| -> (i32, i32) { (col - 1, row) },
        )
    }
    fn get_empty_squares_right(&self, board: &Board, start_position: &Position) -> f32 {
        self.get_empty_squares_in_direction(
            board,
            start_position,
            |col: i32, row: i32| -> (i32, i32) { (col + 1, row) },
        )
    }
    fn get_empty_squares_down(&self, board: &Board, start_position: &Position) -> f32 {
        self.get_empty_squares_in_direction(
            board,
            start_position,
            |col: i32, row: i32| -> (i32, i32) { (col, row + 1) },
        )
    }
    fn get_empty_squares_up(&self, board: &Board, start_position: &Position) -> f32 {
        self.get_empty_squares_in_direction(
            board,
            start_position,
            |col: i32, row: i32| -> (i32, i32) { (col, row - 1) },
        )
    }
    fn get_empty_squares_left_up(&self, board: &Board, start_position: &Position) -> f32 {
        self.get_empty_squares_in_direction(
            board,
            start_position,
            |col: i32, row: i32| -> (i32, i32) { (col - 1, row - 1) },
        )
    }
    fn get_empty_squares_right_up(&self, board: &Board, start_position: &Position) -> f32 {
        self.get_empty_squares_in_direction(
            board,
            start_position,
            |col: i32, row: i32| -> (i32, i32) { (col + 1, row - 1) },
        )
    }
    fn get_empty_squares_down_left(&self, board: &Board, start_position: &Position) -> f32 {
        self.get_empty_squares_in_direction(
            board,
            start_position,
            |col: i32, row: i32| -> (i32, i32) { (col - 1, row + 1) },
        )
    }
    fn get_empty_squares_up_right(&self, board: &Board, start_position: &Position) -> f32 {
        self.get_empty_squares_in_direction(
            board,
            start_position,
            |col: i32, row: i32| -> (i32, i32) { (col + 1, row - 1) },
        )
    }

    //EVALUATERS
    // pub fn get_score(&self, board : &Board, all_moves : &[Move]) -> f32 {
    pub fn get_score(&self, board: &Board) -> f32 {
        let mut score = 0.0;
        let piece_type = self.type_.unwrap();
        match piece_type {
            PieceType::PAWN => {
                // score += self.score_pawn(board, all_moves);
                score += self.score_pawn(board);
            }
            PieceType::KNIGHT => {
                score += self.score_knight(board);
            }
            // PieceType::ROOK => {
            //     score += self.score_rook(board);
            // }
            PieceType::BISHOP => {
                score += self.score_bishop(board);
            }
            // PieceType::QUEEN => {
            //     score += self.score_queen(board);
            // }
            PieceType::KING => {
                score += self.score_king(board);
                // score += 1000.0;
            }
            _ => {
                // score += kind.value() as f32;
                score += self.get_raw_value();
            }
        }
        //+ 0.1 for each defender divided by the piece value. Defenders are should matter less for important pieces
        // score += self.count_defenders(all_moves) / self.type_.unwrap().value() as f32;
        // let defenders = self.icount_defenders(all_moves);
        // if defenders == 0 {
        //     score -= piece_type.value() as f32 / 2.0;
        // }
        // if !self.has_moved() {
        //     score -= 0.3
        // }

        score
    }
    // fn score_pawn(&self, board : &Board, all_moves : &[Move]) -> f32 {
    fn score_pawn(&self, board: &Board) -> f32 {
        let mut score = 1.0;
        //Centrality Score
        if self.in_center() {
            score += 0.25;
        }
        //Passed pawns (no opposing pawns blocking or preventing advancement)
        //Doubled pawns, negative
        //isolation, negative
        // let pawn_defenders = self.count_pawn_defenders(all_moves);
        // if pawn_defenders == 0 {
        //     score -= 0.15;
        // }

        //Punish poor mobility
        if board.turnCount < 30 {
            if self.in_middle_cols() && self.in_original_position() {
                score -= board.turnCount as f32 * 0.01;
            }
        }

        score
        // 1.0 + self.distance_from_center_score(board)
    }
    fn score_knight(&self, board: &Board) -> f32 {
        let mut score = 3.0;
        // 3.0 + self.distance_from_center_score(board)
        //Central + 0.3
        if self.in_center() {
            score += 0.3;
        } else if self.near_center() {
            //Near Center, +0.2
            score += 0.2;
        }
        //Outpost (can't be attacked by pawns and supported by pawns) + 0.5
        //Edge penalty -0.3
        if self.on_edge() {
            score -= 0.3;
        }
        //Corner penalty -0.5
        //pawn support +0.2
        //mobility = 0.1 * moves

        //Punish poor mobility
        if board.turnCount < 30 {
            if self.in_original_position() {
                score -= board.turnCount as f32 * 0.02; //0.6
            }
        }
        score
    }
    fn score_bishop(&self, board: &Board) -> f32 {
        let mut score = 0.0;

        //Punish poor mobility
        if board.turnCount < 30 {
            if self.in_original_position() {
                score -= board.turnCount as f32 * 0.02; //0.6
            }
        }
        // if board.turnCount > 20 {
        score += self.score_diagonal_vision(board); //0.05 * tiles visible. Max = 0.7
                                                    // }

        3.0 + score
    }
    fn score_rook(&self, board: &Board) -> f32 {
        let mut score = 0.0;
        //TODO: File control

        //Punish poor mobility
        // if board.turnCount < 30 {
        //     if self.in_original_position() {
        //         score -= board.turnCount as f32 * 0.02; //0.6
        //     }
        // } else {
        //     let mut score = self.score_horizontal_vision(board); //0.05 * tiles visible. Max = 0.7
        // }
        if board.turnCount > 30 {
            score += self.score_horizontal_vision(board);
        }
        5.0 + score
    }
    fn score_queen(&self, board: &Board) -> f32 {
        let mut score = 0.0;
        if board.turnCount > 30 {
            score += self.score_horizontal_vision(board); //0.05 * tiles visible. Max = 0.7
            score += self.score_diagonal_vision(board);
        } else {
            if self.in_original_position() {
                score += 1.0;
            }
        }
        9.0 + score
    }
    fn score_king(&self, board: &Board) -> f32 {
        //how safe is the king
        let mut score = 0.0;

        //we want the kings lines of sight to be guarded, ideally by pawns.
        //if another piece is guarding in front, that is not ideal because it is pinned
        let mut tgt_row = self.row - 1;
        if self.color.unwrap() == PieceColor::BLACK {
            tgt_row = self.row + 1;
        }
        for i in self.col - 1..self.col + 1 {
            if i < 0 || i > 7 {
                continue;
            }
            if let Some(piece) = board.get_piece(tgt_row, i) {
                let raw_val = piece.get_raw_value();
                if raw_val == 1.0 {
                    score += 0.2;
                } else {
                    score -= raw_val / 5.0;
                }
            }
        }

        //King is better in the corner. TODO: Endgame
        if self.color.unwrap() == PieceColor::BLACK {
            if self.row == 0 && (self.col == 0 || self.col == 1 || self.col == 6 || self.col == 7) {
                score += 1.0;
            }
        } else {
            if self.row == 7 && (self.col == 0 || self.col == 1 || self.col == 6 || self.col == 7) {
                score += 1.0;
            }
        }

        1000.0 + score
    }

    fn icount_defenders(&self, all_moves: &[Move]) -> i32 {
        //Start with minus one because this piece will be one of the end positions
        let mut defenders = -1;
        for mv in all_moves {
            if mv.end_position.col == self.col && mv.end_position.row == self.row {
                defenders += 1;
            }
        }
        defenders
    }
    fn count_defenders(&self, all_moves: &[Move]) -> f32 {
        //Start with minus one because this piece will be one of the end positions
        let mut defenders = -0.05;
        for mv in all_moves {
            if mv.end_position.col == self.col && mv.end_position.row == self.row {
                defenders += 0.05;
            }
        }
        defenders
    }
    fn count_pawn_defenders(&self, all_moves: &[Move]) -> i32 {
        //Start with minus one because this piece will be one of the end positions
        let mut defenders = -1;
        for mv in all_moves.iter().filter(|p| p.get_type() == PieceType::PAWN) {
            if mv.end_position.col == self.col && mv.end_position.row == self.row {
                defenders += 1;
            }
        }
        defenders
    }

    #[inline(always)]
    fn in_center(&self) -> bool {
        (self.row == 3 || self.row == 4) && (self.col == 3 || self.col == 4)
    }
    #[inline(always)]
    fn near_center(&self) -> bool {
        // (self.row == 3 || self.row == 4) && (self.col == 3 || self.col == 4)
        ((self.row == 2 || self.row == 5) && (self.col >= 2 && self.col <= 5))
            || ((self.col == 2 || self.col == 5) && (self.row >= 2 && self.row <= 5))
    }
    #[inline(always)]
    fn in_middle_cols(&self) -> bool {
        self.col == 2 || self.col == 3 || self.col == 4 || self.col == 5
    }
    #[inline(always)]
    fn on_edge(&self) -> bool {
        self.col == 0 || self.col == 7
    }
    #[inline(always)]
    fn distance_from_center_score(&self) -> f32 {
        let distance = ((self.col as f32 - 3.5).powi(2) + (self.row as f32 - 3.5).powi(2)).sqrt();
        (distance - 5.0).abs() / 10.0
    }

    fn in_original_position(&self) -> bool {
        match (self.type_.unwrap(), self.color.unwrap()) {
            (PieceType::PAWN, PieceColor::BLACK) => self.row == 1,
            (PieceType::PAWN, PieceColor::WHITE) => self.row == 6,
            (PieceType::KNIGHT, PieceColor::BLACK) => {
                self.row == 0 && (self.col == 1 || self.col == 6)
            }
            (PieceType::KNIGHT, PieceColor::WHITE) => {
                self.row == 7 && (self.col == 1 || self.col == 6)
            }
            (PieceType::BISHOP, PieceColor::BLACK) => {
                self.row == 0 && (self.col == 2 || self.col == 5)
            }
            (PieceType::BISHOP, PieceColor::WHITE) => {
                self.row == 7 && (self.col == 2 || self.col == 5)
            }
            (PieceType::ROOK, PieceColor::BLACK) => {
                self.row == 0 && (self.col == 0 || self.col == 7)
            }
            (PieceType::ROOK, PieceColor::WHITE) => {
                self.row == 7 && (self.col == 0 || self.col == 7)
            }
            (PieceType::QUEEN, PieceColor::BLACK) => self.row == 0 && self.col == 3,
            (PieceType::QUEEN, PieceColor::WHITE) => self.row == 7 && self.col == 3,
            (PieceType::KING, PieceColor::BLACK) => self.row == 0 && self.col == 4,
            (PieceType::KING, PieceColor::WHITE) => self.row == 7 && self.col == 4,
            (_, _) => false,
        }
    }
    pub fn piece_index(&self) -> usize {
        match (self.type_.unwrap(), self.color.unwrap()) {
            //TODO: Just use proto int val. Ensure doesn't mess up any type eval
            (PieceType::PAWN, PieceColor::BLACK) => 0,
            (PieceType::PAWN, PieceColor::WHITE) => 1,
            (PieceType::KNIGHT, PieceColor::BLACK) => 2,
            (PieceType::KNIGHT, PieceColor::WHITE) => 3,
            (PieceType::BISHOP, PieceColor::BLACK) => 4,
            (PieceType::BISHOP, PieceColor::WHITE) => 5,
            (PieceType::ROOK, PieceColor::BLACK) => 6,
            (PieceType::ROOK, PieceColor::WHITE) => 7,
            (PieceType::QUEEN, PieceColor::BLACK) => 8,
            (PieceType::QUEEN, PieceColor::WHITE) => 9,
            (PieceType::KING, PieceColor::BLACK) => 10,
            (PieceType::KING, PieceColor::WHITE) => 11,
            (_, _) => 0,
        }
    }
    fn score_horizontal_vision(&self, board: &Board) -> f32 {
        let mut start_position = Position::new();
        start_position.row = self.row;
        start_position.col = self.col;

        self.get_empty_squares_down(board, &start_position)
            + self.get_empty_squares_up(board, &start_position)
            + self.get_empty_squares_left(board, &start_position)
            + self.get_empty_squares_right(board, &start_position)
    }
    fn score_diagonal_vision(&self, board: &Board) -> f32 {
        let mut start_position = Position::new();
        start_position.row = self.row;
        start_position.col = self.col;

        self.get_empty_squares_down_left(board, &start_position)
            + self.get_empty_squares_up_right(board, &start_position)
            + self.get_empty_squares_left_up(board, &start_position)
            + self.get_empty_squares_right_up(board, &start_position)
    }
    fn get_raw_value(&self) -> f32 {
        match self.type_.unwrap() {
            PieceType::PAWN => 1.0,
            PieceType::BISHOP => 3.0,
            PieceType::KNIGHT => 3.0,
            PieceType::ROOK => 5.0,
            PieceType::QUEEN => 9.0,
            PieceType::KING => 1000.0,
            _ => 0.0,
        }
    }
}

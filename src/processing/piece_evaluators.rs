use chess::{get_adjacent_files, get_bishop_rays, get_file, get_rook_rays, BitBoard, Board, Color, Piece, Square, EMPTY};

pub fn raw_piece_val(piece: Piece) -> f32 {
    match piece {
        Piece::Pawn => 1.0,
        Piece::Knight => 3.0,
        Piece::Bishop => 3.0,
        Piece::Rook => 5.0,
        Piece::Queen => 9.0,
        Piece::King => 1000.0,
    }
}

pub fn in_original_position(square: &Square, piece: Piece, color: Color) -> bool {
    let row = square.to_index() / 8;
    let col = square.to_index() % 8;
    match (piece, color) {
        (Piece::Pawn, Color::White) => row == 1,
        (Piece::Pawn, Color::Black) => row == 6,
        (Piece::Knight, Color::White) => row == 0 && (col == 1 || col == 6),
        (Piece::Knight, Color::Black) => row == 7 && (col == 1 || col == 6),
        (Piece::Bishop, Color::White) => row == 0 && (col == 2 || col == 5),
        (Piece::Bishop, Color::Black) => row == 7 && (col == 2 || col == 5),
        (Piece::Rook, Color::White) => row == 0 && (col == 0 || col == 7),
        (Piece::Rook, Color::Black) => row == 7 && (col == 0 || col == 7),
        (Piece::Queen, Color::White) => row == 0 && col == 3,
        (Piece::Queen, Color::Black) => row == 7 && col == 3,
        (Piece::King, Color::White) => row == 0 && col == 4,
        (Piece::King, Color::Black) => row == 7 && col == 4,
    }
}

pub fn square_in_center(square: &Square) -> bool {
    let row = square.to_index() / 8;
    let col = square.to_index() % 8;
    (row == 3 || row == 4) && (col == 3 || col == 4)
}
pub fn square_near_center(square: &Square) -> bool {
    let row = square.to_index() / 8;
    let col = square.to_index() % 8;
    ((row == 2 || row == 5) && (col >= 2 && col <= 5))
        || ((col == 2 || col == 5) && (row >= 2 && row <= 5))
}
pub fn square_on_edge(square: &Square) -> bool {
    let row = square.to_index() / 8;
    let col = square.to_index() % 8;
    col == 0 || col == 7 || row == 0 || row == 7
}
pub fn square_in_corner(square: &Square) -> bool {
    let row = square.to_index() / 8;
    let col = square.to_index() % 8;
    (row == 0 || row == 7) && (col == 0 || col == 7)
}
pub fn square_in_middle_cols(square: &Square) -> bool {
    let col = square.to_index() % 8;
    col == 4 || col == 5
}

const SINGLE: BitBoard = BitBoard(1);

pub fn score_pawn(board: &Board, square: Square) -> f32 {
    let mut positional_score = 0.0;

    if square_in_center(&square) {
        positional_score += 0.3;
    } else if square_near_center(&square) {
        positional_score += 0.2;
    }

    let my_combined = board.color_combined(board.side_to_move());
    let my_pawns = board.pieces(Piece::Pawn) & my_combined;

    let my_file = get_file(square.get_file());
    
    //Punish doubled pawns
    if my_pawns & my_file > SINGLE {
        positional_score -= 0.1;
    }

    let adjacent_files = get_adjacent_files(square.get_file());
    //Punish isolated pawns
    if my_pawns & adjacent_files == EMPTY {
        positional_score -= 0.1;
    }

    if square_in_middle_cols(&square) && in_original_position(&square, Piece::Pawn, board.side_to_move()) {
        positional_score -= 0.1;
    }

    // let left_defender = square.ubackward(board.side_to_move()).uleft();
    // if left_defender.get_file() != File::H
    //     && my_pawns & BitBoard::from_square(left_defender) != EMPTY
    // {
    //     positional_score += 0.05;
    // }
    // let right_defender = square.ubackward(board.side_to_move()).uright();
    // //If we have a defender behind and right, and didn't wrap
    // if right_defender.get_file() != File::A
    //     && my_pawns & BitBoard::from_square(right_defender) != EMPTY
    // {
    //     positional_score += 0.05;
    // }

    1.0 + positional_score
}

pub fn score_bishop(_board: &Board, square: Square, color: Color) -> f32 {
    let mut positional_score = 0.0;

    let bishop_vision = get_bishop_rays(square).popcnt() as f32;
    positional_score += bishop_vision * 0.05; //Max of 14 * 0.05 = 0.7

    if in_original_position(&square, Piece::Bishop, color) {
        positional_score -= 0.1;
    }

    3.0 + positional_score
}
pub fn score_knight(_board: &Board, square: Square, color: Color) -> f32 {
    let mut positional_score = 0.0;
    if square_in_center(&square) {
        positional_score += 0.25;
    } else if square_near_center(&square) {
        positional_score += 0.20;
    }
    if square_in_corner(&square) {
        positional_score -= 0.5;
    } else if square_on_edge(&square) {
        positional_score -= 0.25;
    }

    if in_original_position(&square, Piece::Knight, color) {
        positional_score -= 0.1;
    }

    // positional_score += get_knight_moves(square).popcnt() as f32 * 0.1;

    3.0 + positional_score
}
pub fn score_rook(_board: &Board, square: Square) -> f32 {
    let mut positional_score = 0.0;

    let rook_vision = get_rook_rays(square).popcnt() as f32;
    positional_score += rook_vision * 0.05; //Max of 14 * 0.05 = 0.7

    5.0 + positional_score
}
pub fn score_queen(board: &Board, square: Square) -> f32 {
    let mut positional_score = 0.0;

    //If I still have a lot of pieces remaining, encourage queen patience
    // if board.color_combined(board.side_to_move()).popcnt() > 10 {
    //     if in_original_position(&square, Piece::Queen, board.side_to_move()) {
    //         positional_score += 0.3;
    //     }
    // } else {
    //     //Otherwise, lets encourage a mobile queen
    //     let bishop_vision = get_bishop_rays(square).popcnt() as f32;
    //     let rook_vision = get_rook_rays(square).popcnt() as f32;
    //     positional_score += (rook_vision + bishop_vision) * 0.05; //Max of 28 * 0.05 = 1.4
    // }

    9.0 + positional_score
}
pub fn score_king(board: &Board, square: Square) -> f32 {
    let mut positional_score = 0.0;

    for pinned in *board.pinned() {
        if let Some(piece) = board.piece_on(pinned) {
            
            if piece == Piece::Queen {
                positional_score -= 3.0;
            } else if piece == Piece::Rook {
                positional_score -= 1.5;
            }
        }
    }

    //If there are more than 15 pieces left, we should value king safety much more highly
    if board.combined().popcnt() > 15 && square_in_corner(&square) {
        positional_score += 2.0;
    }

    1000.0 + positional_score
}

pub fn score_board(board: &Board) -> f32 {
    let mut black_score = 0.0;
    let mut white_score = 0.0;

    for square in *board.pieces(Piece::Pawn) {
        if let Some(_pawn) = board.piece_on(square) {
            if board.color_on(square).expect("Piece Without Color") == Color::Black {
                black_score += score_pawn(board, square)
            } else {
                white_score += score_pawn(board, square)
            }
        }
    }
    for square in *board.pieces(Piece::Knight) {
        if let Some(_knight) = board.piece_on(square) {
            if board.color_on(square).expect("Piece Without Color") == Color::Black {
                black_score += score_knight(board, square, Color::Black)
            } else {
                white_score += score_knight(board, square, Color::White)
            }
        }
    }
    for square in *board.pieces(Piece::Bishop) {
        if let Some(_bishop) = board.piece_on(square) {
            if board.color_on(square).expect("Piece Without Color") == Color::Black {
                black_score += score_bishop(board, square, Color::Black)
            } else {
                white_score += score_bishop(board, square, Color::White)
            }
        }
    }
    for square in *board.pieces(Piece::Rook) {
        if let Some(_rook) = board.piece_on(square) {
            if board.color_on(square).expect("Piece Without Color") == Color::Black {
                black_score += score_rook(board, square)
            } else {
                white_score += score_rook(board, square)
            }
        }
    }
    for square in *board.pieces(Piece::Queen) {
        if let Some(_queen) = board.piece_on(square) {
            if board.color_on(square).expect("Piece Without Color") == Color::Black {
                black_score += score_queen(board, square)
            } else {
                white_score += score_queen(board, square)
            }
        }
    }
    for square in *board.pieces(Piece::King) {
        if let Some(_king) = board.piece_on(square) {
            if board.color_on(square).expect("Piece Without Color") == Color::Black {
                black_score += score_king(board, square)
            } else {
                white_score += score_king(board, square)
            }
        }
    }

    white_score - black_score
}

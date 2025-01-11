use std::{
    cmp::Ordering,
    str::FromStr,
    sync::{
        atomic::{self, AtomicI64},
        Arc, RwLock,
    },
    time::Instant,
};

use chess::{
    get_bishop_rays, get_king_moves, get_knight_moves, get_pawn_attacks, get_rook_rays, BitBoard,
    Board, BoardBuilder, CacheTable, ChessMove, Color, File, MoveGen, Piece, Rank, Square, EMPTY,
};
use rayon::{iter::ParallelIterator, slice::ParallelSlice};

use crate::{Move, NUM_THREADS};

#[derive(Clone, Copy, PartialEq, PartialOrd)]
enum Flag {
    Exact,
    Lower,
    Upper,
}
#[derive(Clone, Copy, PartialEq, PartialOrd)]
struct TranspositionEntry {
    score: f32,
    depth: i8,
    flag: Flag,
}
impl TranspositionEntry {
    fn new(score: f32, depth: i8, flag: Flag) -> Self {
        Self { score, depth, flag }
    }
}
impl Default for TranspositionEntry {
    fn default() -> Self {
        Self {
            score: Default::default(),
            depth: Default::default(),
            flag: Flag::Exact,
        }
    }
}

type TranspositionTable = CacheTable<TranspositionEntry>;
fn raw_piece_val(piece: Piece) -> f32 {
    match piece {
        Piece::Pawn => 1.0,
        Piece::Knight => 3.0,
        Piece::Bishop => 3.0,
        Piece::Rook => 5.0,
        Piece::Queen => 9.0,
        Piece::King => 1000.0,
    }
}

fn in_original_position(square: &Square, piece: Piece, color: Color) -> bool {
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

fn square_in_center(square: &Square) -> bool {
    let row = square.to_index() / 8;
    let col = square.to_index() % 8;
    (row == 3 || row == 4) && (col == 3 || col == 4)
}
fn square_near_center(square: &Square) -> bool {
    let row = square.to_index() / 8;
    let col = square.to_index() % 8;
    ((row == 2 || row == 5) && (col >= 2 && col <= 5))
        || ((col == 2 || col == 5) && (row >= 2 && row <= 5))
}
fn square_on_edge(square: &Square) -> bool {
    let row = square.to_index() / 8;
    let col = square.to_index() % 8;
    col == 0 || col == 7 || row == 0 || row == 7
}
fn square_in_corner(square: &Square) -> bool {
    let row = square.to_index() / 8;
    let col = square.to_index() % 8;
    (row == 0 || row == 7) && (col == 0 || col == 7)
}
fn square_in_middle_cols(square: &Square) -> bool {
    let col = square.to_index() % 8;
    col == 4 || col == 5
}

fn score_pawn(board: &Board, square: Square) -> f32 {
    let mut positional_score = 0.0;

    if square_in_center(&square) {
        positional_score += 0.25;
    } else if square_near_center(&square) {
        positional_score += 0.15;
    }

    let my_combined = board.color_combined(board.side_to_move());
    let my_pawns = board.pieces(Piece::Pawn);

    let left_defender = square.ubackward(board.side_to_move()).uleft();
    if left_defender.get_file() != File::H
        && my_combined & my_pawns & BitBoard::from_square(left_defender) != EMPTY
    {
        positional_score += 0.05;
    }
    let right_defender = square.ubackward(board.side_to_move()).uright();
    //If we have a defender behind and right, and didn't wrap
    if right_defender.get_file() != File::A
        && my_combined & my_pawns & BitBoard::from_square(right_defender) != EMPTY
    {
        positional_score += 0.05;
    }

    1.0 + positional_score
}

fn score_bishop(_board: &Board, square: Square, color: Color) -> f32 {
    let mut positional_score = 0.0;

    let bishop_vision = get_bishop_rays(square).popcnt() as f32;
    positional_score += bishop_vision * 0.05; //Max of 14 * 0.05 = 0.7

    if in_original_position(&square, Piece::Bishop, color) {
        positional_score -= 0.1;
    }

    3.0 + positional_score
}
fn score_knight(_board: &Board, square: Square, color: Color) -> f32 {
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
fn score_rook(_board: &Board, square: Square) -> f32 {
    let mut positional_score = 0.0;

    let rook_vision = get_rook_rays(square).popcnt() as f32;
    positional_score += rook_vision * 0.05; //Max of 14 * 0.05 = 0.7

    5.0 + positional_score
}
fn score_queen(_board: &Board, square: Square) -> f32 {
    let mut positional_score = 0.0;

    // let bishop_vision = get_bishop_rays(square).popcnt() as f32;
    // let rook_vision = get_rook_rays(square).popcnt() as f32;
    // positional_score += (rook_vision + bishop_vision) * 0.05; //Max of 28 * 0.05 = 1.4

    9.0 + positional_score
}
fn score_king(board: &Board, square: Square) -> f32 {
    let mut positional_score = 0.0;

    for pinned in *board.pinned() {
        if let Some(piece) = board.piece_on(pinned) {
            positional_score -= raw_piece_val(piece) / 5.0
        }
    }

    //If there are more than 15 pieces left, we should value king safety much more highly
    if board.combined().popcnt() > 15 && square_in_corner(&square) {
        positional_score += 2.0;
    }

    1000.0 + positional_score
}

fn score_board(board: &Board) -> f32 {
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

fn alpha_beta(
    board: Board,
    depth: i8,
    mut alpha: f32,
    mut beta: f32,
    maximizer: bool,
    transpositions: &mut TranspositionTable,
) -> f32 {
    let board_hash = board.get_hash();

    if let Some(entry) = transpositions.get(board_hash) {
        if entry.depth >= depth {
            match entry.flag {
                Flag::Exact => return entry.score,
                Flag::Lower => alpha = alpha.max(entry.score),
                Flag::Upper => beta = beta.min(entry.score),
            }

            if alpha >= beta {
                return entry.score;
            }
        }
    }

    if depth == 0 {
        let eval = score_board(&board);

        transpositions.add(
            board_hash,
            TranspositionEntry::new(eval, depth, Flag::Exact),
        );
        return eval;
    }

    let mut best_eval = if maximizer {
        f32::NEG_INFINITY
    } else {
        f32::INFINITY
    };

    let mut move_iter = MoveGen::new_legal(&board);
    // let moves: Vec<ChessMove> = move_iter.collect();
    let targets = board.color_combined(!board.side_to_move());
    move_iter.set_iterator_mask(*targets); //Use to get all attackers for castle ssquares?

    let mut moves: Vec<ChessMove> = Vec::new();

    for capture in &mut move_iter {
        moves.push(capture);
    }

    move_iter.set_iterator_mask(!EMPTY);

    for regular in &mut move_iter {
        moves.push(regular);
    }

    let mut flag = Flag::Exact;

    for mv in moves {
        let new_board = board.make_move_new(mv);
        let eval = alpha_beta(
            new_board,
            depth - 1,
            alpha,
            beta,
            !maximizer,
            transpositions,
        );

        if maximizer {
            best_eval = best_eval.max(eval);
            alpha = alpha.max(eval);
            if alpha >= beta {
                flag = Flag::Lower;
                break;
            }
        } else {
            best_eval = best_eval.min(eval);
            beta = beta.min(eval);
            if beta <= alpha {
                flag = Flag::Upper;
                break;
            }
        }
    }

    transpositions.add(board_hash, TranspositionEntry::new(best_eval, depth, flag));

    best_eval
}

pub fn find_best_move(board: Board, depth: i8) -> ChessMove {
    let now = Instant::now();

    let mut best_move: ChessMove = ChessMove::default();
    let mut best_score = f32::NEG_INFINITY;

    let white_pawns = board.pieces(chess::Piece::Pawn) & board.color_combined(chess::Color::White);
    for p in white_pawns {
        println!("White Pawn on Square = {p}");
    }

    let mut move_iter = MoveGen::new_legal(&board);

    let targets = board.color_combined(!board.side_to_move());
    move_iter.set_iterator_mask(*targets); //Use to get all attackers for castle ssquares?

    let mut moves: Vec<ChessMove> = Vec::new();

    //MAYBE do checkers too
    for capture in &mut move_iter {
        moves.push(capture);
    }

    move_iter.set_iterator_mask(!EMPTY);

    for regular in &mut move_iter {
        moves.push(regular);
    }

    let mut transposition_table: TranspositionTable =
        CacheTable::new(2usize.pow(24), TranspositionEntry::default());

    for mv in moves {
        println!("Evaluating Move = {mv}");
        let new_board = board.make_move_new(mv);

        let eval = alpha_beta(
            new_board,
            depth - 1,
            f32::NEG_INFINITY,
            f32::INFINITY,
            false, //TODO replace with player==white
            &mut transposition_table,
        );
        if eval > best_score {
            best_score = eval;
            println!("Best Move Opt = {mv}");
            best_move = mv.clone();
        }
    }

    let elapsed = now.elapsed();

    best_move
}
struct ScorePair(ChessMove, f32);

pub fn find_best_move_chunks(board: Board, depth: i8) -> ChessMove {
    let now = Instant::now();

    // let mut best_move: ChessMove = ChessMove::default();
    // let mut best_score = f32::NEG_INFINITY;
    let mut move_iter = MoveGen::new_legal(&board);

    let targets = board.color_combined(!board.side_to_move());
    move_iter.set_iterator_mask(*targets); //Use to get all attackers for castle ssquares?

    let mut moves: Vec<ChessMove> = Vec::new();

    //MAYBE do checkers too
    for capture in &mut move_iter {
        moves.push(capture);
    }

    move_iter.set_iterator_mask(!EMPTY);

    for regular in &mut move_iter {
        moves.push(regular);
    }

    let chunk_size = (moves.len() + NUM_THREADS - 1) / NUM_THREADS;
    // let mut transposition_table: Arc<RwLock<TranspositionTable>> =
    //     Arc::new(
    //         RwLock::new(
    //             CacheTable::new(2usize.pow(24), TranspositionEntry::default())
    //         )
    //     );

    let bests: Vec<ScorePair> = moves
        .par_chunks(chunk_size)
        .map(|chunk| {
            let mut best_move: ChessMove = ChessMove::default();
            let mut best_score = f32::NEG_INFINITY;
            let mut transposition_table =
                TranspositionTable::new(2usize.pow(22), TranspositionEntry::default());

            for mv in chunk {
                let new_board = board.make_move_new(*mv);

                let eval = alpha_beta(
                    new_board,
                    depth - 1,
                    f32::NEG_INFINITY,
                    f32::INFINITY,
                    false, //TODO replace with player==white
                    &mut transposition_table,
                );
                if eval > best_score {
                    best_score = eval;
                    println!("Best Move Opt = {mv}");
                    best_move = mv.clone();
                }
            }
            println!("Found a best score = {}", best_score);
            ScorePair {
                0: best_move,
                1: best_score,
            }
        })
        .collect();

    let best_move = bests
        .iter()
        .max_by(|a, b| -> Ordering { a.1.total_cmp(&b.1) })
        .expect("Best Move missing in Bets");

    let elapsed = now.elapsed();

    println!("Search Took {} ms", elapsed.as_millis());

    best_move.0
}

// fn alpha_beta_p(
//     board: Board,
//     depth: i8,
//     mut alpha: f32,
//     mut beta: f32,
//     maximizer: bool,
//     transpositions: Arc<RwLock<TranspositionTable>>,
// ) -> f32 {
//     let board_hash = board.get_hash();

//     {
//         if let Some(entry) = transpositions.read().unwrap().get(board_hash) {
//             if entry.depth >= depth {
//                 match entry.flag {
//                     Flag::Exact => return entry.score,
//                     Flag::Lower => alpha = alpha.max(entry.score),
//                     Flag::Upper => beta = beta.min(entry.score),
//                 }

//                 if alpha >= beta {
//                     return entry.score;
//                 }
//             }
//         }
//     }

//     if depth == 0 {
//         let eval = score_board(&board);

//         transpositions.write().unwrap().add(
//             board_hash,
//             TranspositionEntry::new(eval, depth, Flag::Exact),
//         );
//         return eval;
//     }

//     let mut best_eval = if maximizer {
//         f32::NEG_INFINITY
//     } else {
//         f32::INFINITY
//     };

//     let mut move_iter  = MoveGen::new_legal(&board);
//     // let moves: Vec<ChessMove> = move_iter.collect();
//     let targets = board.color_combined(!board.side_to_move());
//     move_iter.set_iterator_mask(*targets);//Use to get all attackers for castle ssquares?

//     let mut moves: Vec<ChessMove> = Vec::new();

//     for capture in &mut move_iter {
//         moves.push(capture);
//     }

//     move_iter.set_iterator_mask(!EMPTY);

//     for regular in &mut move_iter {
//         moves.push(regular);
//     }

//     let mut flag = Flag::Exact;

//     for mv in moves {
//         let new_board = board.make_move_new(mv);
//         let eval = alpha_beta_p(new_board, depth - 1, alpha, beta, !maximizer, transpositions.clone());

//         if maximizer {
//             best_eval = best_eval.max(eval);
//             alpha = alpha.max(eval);
//             if alpha >= beta {
//                 flag = Flag::Lower;
//                 break;
//             }
//         } else {
//             best_eval = best_eval.min(eval);
//             beta = beta.min(eval);
//             if beta <= alpha {
//                 flag = Flag::Upper;
//                 break;
//             }
//         }
//     }

//     {
//         transpositions.write().unwrap().add(
//             board_hash,
//             TranspositionEntry::new(best_eval, depth, flag),
//         );
//     }

//     best_eval
// }

pub fn fix_castle_rights(board: &Board, fen: String) -> Board {
    if board.checkers().popcnt() != 0 {
        return *board;
    }
    let other_board = board.null_move().unwrap();

    let mut builder = BoardBuilder::from_str(&fen).expect("Fen String Invalid");

    adjust_castle_rights_for_board(board, &mut builder);
    adjust_castle_rights_for_board(&other_board, &mut builder);

    let res: Result<Board, _> = builder.try_into();

    if res.is_err() {
        return *board;
    } else {
        return res.unwrap();
    }
}

fn adjust_castle_rights_for_board(board: &Board, builder: &mut BoardBuilder) {
    let combined = board.combined();

    let my_kingside_clear = combined
        & board
            .my_castle_rights()
            .kingside_squares(board.side_to_move())
        == EMPTY;
    let my_queenside_clear = combined
        & board
            .my_castle_rights()
            .queenside_squares(board.side_to_move())
        == EMPTY;

    println!("Eval Rights For Color = {:?}", board.side_to_move());

    let ksq = board.king_square(board.side_to_move());

    if ksq.get_file() != File::E {
        return;
    }

    if my_kingside_clear && my_queenside_clear {
        println!("adding both castle rights");
        builder.castle_rights(board.side_to_move(), chess::CastleRights::Both);
    } else if my_kingside_clear {
        println!("adding kingside castle rights");
        builder.castle_rights(board.side_to_move(), chess::CastleRights::KingSide);
    } else if my_queenside_clear {
        println!("adding queenside castle rights");
        builder.castle_rights(board.side_to_move(), chess::CastleRights::QueenSide);
    } else {
        println!("no castle rights");
        builder.castle_rights(board.side_to_move(), chess::CastleRights::NoRights);
    }
}

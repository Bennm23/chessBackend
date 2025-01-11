use std::{
    cmp::Ordering, str::FromStr, sync::
        Arc
    , time::{Duration, Instant}
};

use chess::{
    BitBoard, Board, BoardBuilder, CacheTable, ChessMove, File, MoveGen, EMPTY
};
use dashmap::DashMap;
use rayon::{iter::ParallelIterator, slice::ParallelSlice};

use crate::NUM_THREADS;

use super::piece_evaluators::score_board;

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub enum Flag {
    Exact,
    Lower,
    Upper,
}
#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct TranspositionEntry {
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

    let move_iter = MoveGen::new_legal(&board);

    let mut moves: Vec<ChessMove> = Vec::new();

    //SNAPSHOT: BEAT Charles Bot (2000), Effective Rating 1700-2050
    let targets = board.color_combined(!board.side_to_move());
    for mv in move_iter {
        let new_board = board.make_move_new(mv);

        let checkers = new_board.checkers();

        let dsq = BitBoard::from_square(mv.get_dest());

        //Prioritize checks and captures
        if checkers & dsq != EMPTY {
            moves.insert(0, mv);
        } else if targets & dsq != EMPTY {
            moves.insert(0, mv);
        } else {
            moves.push(mv);
        }
        
    }
    // let moves: Vec<ChessMove> = move_iter.collect();
    // let targets = board.color_combined(!board.side_to_move());
    // move_iter.set_iterator_mask(*targets); //Use to get all attackers for castle ssquares?


    // for capture in &mut move_iter {
    //     moves.push(capture);
    // }

    // move_iter.set_iterator_mask(!EMPTY);

    // for regular in &mut move_iter {
    //     moves.push(regular);
    // }

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

pub fn find_best_move(board: Board, depth: i8, transposition_table: &mut TranspositionTable, prev_bests: &Vec<ScorePair>) -> Vec<ScorePair> {

    let mut best_move: ChessMove = ChessMove::default();
    let mut best_score = f32::NEG_INFINITY;

    let mut move_iter = MoveGen::new_legal(&board);

    let targets = board.color_combined(!board.side_to_move());
    move_iter.set_iterator_mask(*targets); //Use to get all attackers for castle ssquares?

    let mut moves: Vec<ChessMove> = Vec::new();
    for mv in prev_bests {
        moves.push(mv.0);
    }

    //MAYBE do checkers too
    for capture in &mut move_iter {
        if moves.contains(&capture) {
            continue;
        }
        moves.push(capture);
    }

    move_iter.set_iterator_mask(!EMPTY);

    for regular in &mut move_iter {
        if moves.contains(&regular) {
            continue;
        }
        moves.push(regular);
    }

    // //MAYBE do checkers too
    // for capture in &mut move_iter {
    //     moves.push(capture);
    // }

    // move_iter.set_iterator_mask(!EMPTY);

    // for regular in &mut move_iter {
    //     moves.push(regular);
    // }

    // let mut transposition_table: TranspositionTable =
    //     CacheTable::new(2usize.pow(24), TranspositionEntry::default());

    let mut best_moves: Vec<ScorePair> = Vec::new();

    for mv in moves {
        let new_board = board.make_move_new(mv);

        let eval = alpha_beta(
            new_board,
            depth - 1,
            f32::NEG_INFINITY,
            f32::INFINITY,
            false, //TODO replace with player==white
            transposition_table,
        );
        if eval > best_score {
            best_score = eval;
            best_move = mv.clone();
            best_moves.insert(0, ScorePair{0: best_move, 1: best_score});
        }
    }

    // best_move
    best_moves
}
pub struct ScorePair(ChessMove, f32);

const MAX_SEARCH_TIME: Duration = Duration::from_secs(3);

pub fn find_best_move_iterable(board: Board, max_depth: i8) -> ChessMove {
    let now = Instant::now();

    // let mut transposition_table: TranspositionTable =
    //     CacheTable::new(2usize.pow(24), TranspositionEntry::default());

    let mut latest_bests: Vec<ScorePair> = Vec::new();
    // let transposition_table: SafeTransTable = Arc::new(DashMap::with_capacity(2usize.pow(25)));
    
    for depth in 1..max_depth {
        // let these_bests = find_best_move(board, depth, &mut transposition_table, &latest_bests);
        //no parallel, with using previous best moves
        // Got to depth 1 in 94 ms
        // Got to depth 2 in 94 ms
        // Got to depth 3 in 96 ms
        // Got to depth 4 in 108 ms
        // Got to depth 5 in 207 ms
        // Got to depth 6 in 741 ms
        // Got to depth 7 in 5398 ms

        let these_bests = find_best_move_chunks(board, depth, &now); //No point in passing bests because we split the chunks anyway
        //Prioritize checks and captures + parallel
        //Got to depth 1 in 41 ms
        // Got to depth 2 in 83 ms
        // Got to depth 3 in 124 ms
        // Got to depth 4 in 172 ms
        // Got to depth 5 in 252 ms
        // Got to depth 6 in 511 ms
        // Got to depth 7 in 2572 ms
        // Got to depth 8 in 4012 ms


    // best_move.0
        //no shared transposition TODO: HOW CAN THIS BE BETTER???
        // Got to depth 1 in 42 ms
        // Got to depth 2 in 85 ms
        // Got to depth 3 in 127 ms
        // Got to depth 4 in 176 ms
        // Got to depth 5 in 255 ms
        // Got to depth 6 in 520 ms
        // Got to depth 7 in 2586 ms
        // Got to depth 8 in 12787 ms

        // best = find_best_move_chunks_p(board, depth, transposition_table.clone());
        // Concurrent, size = 2^24
        // Got to depth 1 in 14 ms
        // Got to depth 2 in 15 ms
        // Got to depth 3 in 18 ms
        // Got to depth 4 in 48 ms
        // Got to depth 5 in 181 ms
        // Got to depth 6 in 507 ms
        // Got to depth 7 in 2911 ms

        if now.elapsed() > MAX_SEARCH_TIME {
            println!("Aborted at depth {depth} after {} ms", now.elapsed().as_millis());
            break;
        } else {
            println!("Got to depth {depth} in {} ms", now.elapsed().as_millis());
            latest_bests = these_bests;
        }
    }
    let best_move = latest_bests
        .iter()
        .max_by(|a, b| -> Ordering { a.1.total_cmp(&b.1) })
        .expect("Best Move missing in Bets");

    println!("Best Move Board Eval = {}", best_move.1);

    best_move.0
}
pub fn find_best_move_chunks(board: Board, depth: i8, starter: &Instant) -> Vec<ScorePair> {
    let move_iter = MoveGen::new_legal(&board);

    let moves: Vec<ChessMove> = move_iter.collect();


    //TODO: Spread out checks/captures throughout the chunks?

    // let targets = board.color_combined(!board.side_to_move());
    // move_iter.set_iterator_mask(*targets); //Use to get all attackers for castle ssquares?

    // let mut moves: Vec<ChessMove> = Vec::new();

    // //MAYBE do checkers too
    // for capture in &mut move_iter {
    //     moves.push(capture);
    // }

    // move_iter.set_iterator_mask(!EMPTY);

    // for regular in &mut move_iter {
    //     moves.push(regular);
    // }

    let chunk_size = (moves.len() + NUM_THREADS - 1) / NUM_THREADS;

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
                    best_move = mv.clone();
                }

                if starter.elapsed() > MAX_SEARCH_TIME {
                    break;
                }
            }
            ScorePair {
                0: best_move,
                1: best_score,
            }
        })
        .collect();

    bests
}

type SafeTransTable = Arc<DashMap<u64, TranspositionEntry>>;
fn alpha_beta_p(
    board: Board,
    depth: i8,
    mut alpha: f32,
    mut beta: f32,
    maximizer: bool,
    transpositions: SafeTransTable,
) -> f32 {
    let board_hash = board.get_hash();

    if let Some(entry) = transpositions.get(&board_hash) {
    // if let Some(entry) = transpositions.read().get(board_hash) {
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

        transpositions.insert(
        // transpositions.write().add(
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
        let eval = alpha_beta_p(
            new_board,
            depth - 1,
            alpha,
            beta,
            !maximizer,
            transpositions.clone(),
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

    transpositions.insert(board_hash, TranspositionEntry::new(best_eval, depth, flag));
    // transpositions.write().add(board_hash, TranspositionEntry::new(best_eval, depth, flag));

    best_eval
}
pub fn find_best_move_chunks_p(board: Board, depth: i8, transposition_table: SafeTransTable) -> ChessMove {
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

    // let transposition_table: Arc<DashMap<u64, TranspositionEntry>> = Arc::new(DashMap::with_capacity(2usize.pow(24)));

    let bests: Vec<ScorePair> = moves
        .par_chunks(chunk_size)
        .map(|chunk| {
            let mut best_move: ChessMove = ChessMove::default();
            let mut best_score = f32::NEG_INFINITY;
            // let mut transposition_table =
            //     TranspositionTable::new(2usize.pow(22), TranspositionEntry::default());

            for mv in chunk {
                let new_board = board.make_move_new(*mv);

                let eval = alpha_beta_p(
                    new_board,
                    depth - 1,
                    f32::NEG_INFINITY,
                    f32::INFINITY,
                    false, //TODO replace with player==white
                    transposition_table.clone(),
                    // &mut transposition_table,
                );
                if eval > best_score {
                    best_score = eval;
                    best_move = mv.clone();
                }
            }
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


    best_move.0
}

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

    let ksq = board.king_square(board.side_to_move());

    if ksq.get_file() != File::E {
        return;
    }

    if my_kingside_clear && my_queenside_clear {
        builder.castle_rights(board.side_to_move(), chess::CastleRights::Both);
    } else if my_kingside_clear {
        builder.castle_rights(board.side_to_move(), chess::CastleRights::KingSide);
    } else if my_queenside_clear {
        builder.castle_rights(board.side_to_move(), chess::CastleRights::QueenSide);
    } else {
        builder.castle_rights(board.side_to_move(), chess::CastleRights::NoRights);
    }
}

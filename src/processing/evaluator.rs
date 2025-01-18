use pleco::{board::castle_rights::Castling, core::{masks::{CASTLING_PATH_BLACK_K_SIDE, RANK_1, RANK_BB}, mono_traits::{BlackType, PlayerTrait, WhiteType}, sq, ALL_FILES}, helper::prelude::{adjacent_file, ring_distance}, BitBoard, Board, PieceType, Player, Rank};

use super::consts::{*};

pub fn eval_board(board: &Board) -> MyVal {
    
    if board.turn() == Player::White {
        score_player::<WhiteType>(board) - score_player::<BlackType>(board)
    } else {
        score_player::<BlackType>(board) - score_player::<WhiteType>(board)
    }

}

#[inline(always)]
fn count_pieces<P: PlayerTrait>(board: &Board, ptype: PieceType) -> MyVal {
    board.count_piece(P::player(), ptype) as MyVal
}

fn score_player<P: PlayerTrait>(board: &Board) -> MyVal {

    let mut sum = 0;

    sum += count_pieces::<P>(board, PieceType::P) * PAWN_VALUE;
    sum += score_pawns::<P>(board);
    
    sum += count_pieces::<P>(board, PieceType::N) * KNIGHT_VALUE;
    sum += score_knights::<P>(board);
    
    sum += count_pieces::<P>(board, PieceType::B) * BISHOP_VALUE;
    sum += score_bishops::<P>(board);

    sum += count_pieces::<P>(board, PieceType::R) * ROOK_VALUE;
    sum += score_rooks::<P>(board);

    sum += count_pieces::<P>(board, PieceType::Q) * QUEEN_VALUE;

    sum += count_pieces::<P>(board, PieceType::K) * KING_VALUE;
    sum += score_king::<P>(board);

    sum
}


fn score_pawns<P: PlayerTrait>(board: &Board) -> MyVal {

    let mut score = 0;
    let my_pawns = board.piece_bb(P::player(), PieceType::P);
    let enemy_pawns = board.piece_bb(P::player().other_player(), PieceType::P);

    // let mut sqs_defended = BitBoard(0);

    let count_other = board.non_pawn_material(P::player().other_player());

    for square in my_pawns {
        if count_other < 4 {
            score += PAWN_LATE_POS[P::player() as usize][square.0 as usize];
        } else {
            score += PAWN_EARLY_POS[P::player() as usize][square.0 as usize];
        }
        // sqs_defended |= board.magic_helper.pawn_attacks_from(square, P::player());

        let neighbors = my_pawns & adjacent_file(square.file());
        let supported = neighbors & P::down(square).rank_bb();

        if supported.is_not_empty() {
            score += supported.count_bits() as MyVal * SUPPORTED_PAWN;
        } else if neighbors.is_not_empty() {
            score += neighbors.count_bits() as MyVal * NEIGHBOR_PAWN;
        } else {
            score -= PAWN_ISOLATION;
        }
        // if neighbors.is_not_empty() {
        //     score += neighbors.count_bits() as MyVal * NEIGHBOR_PAWN;
        //     score += supported.count_bits() as MyVal * SUPPORTED_PAWN;
        //     if neighbors.is_not_empty() {
        //     } else {
        //     }
        // }
    }

    for file in ALL_FILES {

        let my_pawns = file.bb() & my_pawns;

        let file_cnt = my_pawns.count_bits() as MyVal;
        //Doubled pawn penalty
        if file_cnt > 1 {
            score -= file_cnt * 3;
        }

        let enemy_pawns = file.bb() & enemy_pawns;
        if file_cnt > 0 && enemy_pawns.count_bits() == 0 {
            score += PASSED_PAWN;
        }
    }

    //Get pawns defended by other pawns
    // sqs_defended &= my_pawns;
    // score += 2 * sqs_defended.count_bits() as MyVal;


    score
}

fn score_knights<P: PlayerTrait>(board: &Board) -> MyVal {

    let mut score = 0;

    let my_knights = board.piece_bb(P::player(), PieceType::N);

    for square in my_knights {
        //positional bonus
        score += KNIGHT_POS[square.0 as usize] / 2;

        //mobility bonus
        let attacks = board.attacks_from(PieceType::N, square, P::player());
        score += 2 * attacks.count_bits() as MyVal;
    }

    score
}

fn score_bishops<P: PlayerTrait>(board: &Board) -> MyVal {

    let mut score = 0;

    let my_bishops = board.piece_bb(P::player(), PieceType::B);

    if my_bishops.count_bits() == 2 {
        score += TWO_BISHOPS;
    }

    for square in my_bishops {
        //mobility bonus
        let attacks = board.attacks_from(PieceType::B, square, P::player());
        score += attacks.count_bits() as MyVal;
    }

    score
}


fn score_rooks<P: PlayerTrait>(board: &Board) -> MyVal {

    let mut score = 0;

    let my_rooks = board.piece_bb(P::player(), PieceType::R);

    // let enemy_pieces = board.piece_bb(P::player().other_player(), PieceType::All);


    for square in my_rooks {
        //mobility bonus, maybe only late?
        let attacks = board.attacks_from(PieceType::R, square, P::player());
        score += attacks.count_bits() as MyVal;

        let my_file = square.file_bb();

        //Encourage open files
        if my_file.count_bits() == 1 {
            score += ROOK_OPEN_FILE;
        } else {
            //Penalize obstructions
            score -= (my_file.count_bits() as MyVal - 1) * 2;
        }

        //Encourage rooks attacking
        if P::player() == Player::White {
            if square.rank() == Rank::R7 || square.rank() == Rank::R8 {
                score += ROOK_MATING;
            }
        } else {
            if square.rank() == Rank::R1 || square.rank() == Rank::R2 {
                score += ROOK_MATING;
                
            }
        }
    }

    score
}

fn score_king<P: PlayerTrait>(board: &Board) -> MyVal {

    let mut score = 0;


    let ksq = board.king_sq(P::player());
    let my_pawns = board.piece_bb(P::player(), PieceType::P);
    let non_pawns = board.piece_bb_both_players(PieceType::All) ^ board.piece_bb_both_players(PieceType::P);

    if board.player_can_castle(P::player()).bits() != 0 {
        score += CASTLE_ABILITY;
    }

    if non_pawns.count_bits() > 6 {//includes 2 for kings
        //Encourage castled king early
        score += KING_EARLY_POS[P::player() as usize][ksq.0 as usize];
    } else {
        //Encourage king activity when there are less mobile pieces remaining
        score += KING_LATE_POS[P::player() as usize][ksq.0 as usize];
    }

    let mut min_king_distance = 0;
    if !my_pawns.is_empty() {
        while (ring_distance(ksq, min_king_distance as u8) & my_pawns).is_empty() {
            min_king_distance += 1;
        }
    }

    //Penalize king isolation
    score += KING_ISOLATION_PENALTY * min_king_distance;

    score
}
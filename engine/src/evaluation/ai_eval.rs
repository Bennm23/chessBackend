use pleco::{
    core::{
        mono_traits::{BlackType, PlayerTrait, WhiteType},
        score::Score,
    },
    BitBoard, Board, PieceType, Player, SQ,
};

use crate::{
    consts::{
        EvalVal, MyVal, BISHOP_EG, BISHOP_MG, DOUBLE_PAWN_PENALTY, KNIGHT_EG, KNIGHT_MG,
        MAX_PHASE, MOBILITY_BONUS, PASSED_PAWN_BONUS, PAWN_ADVANCEMENT_SCORES,
        PAWN_EG, PAWN_MG, QUEEN_EG, QUEEN_MG, ROOK_EG, ROOK_MG, WING_ADVANCE_SCORES,
    },
    debug::{EvalDebugger, EvalPasses, Tracing},
    tables::{material::Material, pawn_table::PawnTable},
};

const MIDGAME_PHASE_LIMIT: EvalVal = 15258;
const ENDGAME_PHASE_LIMIT: EvalVal = 3915;
fn phase(board: &Board) -> EvalVal {
    // Based on non-pawn material.
    let mut npm = board.non_pawn_material_all();
    npm = ENDGAME_PHASE_LIMIT.max(npm.min(MIDGAME_PHASE_LIMIT));
    (((npm - ENDGAME_PHASE_LIMIT) * MAX_PHASE as i32) / (MIDGAME_PHASE_LIMIT - ENDGAME_PHASE_LIMIT)) as EvalVal
}

// --- Extra tuning constants (you can tweak these) ---

const BISHOP_PAIR_MG: EvalVal = 30;
const BISHOP_PAIR_EG: EvalVal = 40;

const ROOK_OPEN_FILE_MG: EvalVal = 15;
const ROOK_OPEN_FILE_EG: EvalVal = 10;
const ROOK_SEMI_OPEN_FILE_MG: EvalVal = 8;
const ROOK_SEMI_OPEN_FILE_EG: EvalVal = 5;

const KING_SHIELD_PAWN_MG: EvalVal = 10;
const KING_SHIELD_PAWN_EG: EvalVal = 3;

// Simple knight centralization bonus
const KNIGHT_CENTER_MG: EvalVal = 5;
const BISHOP_MOBILITY_MG: EvalVal = 2;

pub struct BasicEvaluator<'a, T: Tracing<EvalDebugger>> {
    board: &'a Board,
    phase: EvalVal,
    tracer: T,
    all_bb: BitBoard,
    board_pieces: [[PieceType; 64]; 2],
}

impl<'a, T: Tracing<EvalDebugger>> BasicEvaluator<'a, T> {
    pub fn new(
        board: &'a Board,
        trace: T,
        _pawn_table: &'a mut PawnTable,
        _material: &'a mut Material,
    ) -> Self {
        let mut board_pieces = [[PieceType::None; 64]; 2];

        for (sq, p) in board.get_piece_locations() {
            if p.type_of() == PieceType::None {
                continue;
            }
            board_pieces[p.player_lossy() as usize][sq.0 as usize] = p.type_of();
        }

        Self {
            board,
            phase: phase(board),
            tracer: trace,
            all_bb: board.piece_bb_both_players(PieceType::All),
            board_pieces,
        }
    }

    #[allow(unused)]
    pub fn debug_piece_locations(&self) {
        for p in 0..2 {
            println!("-----------");
            println!("Player = {p}");
            for sq in 0..64 {
                if self.board_pieces[p][sq] == PieceType::None {
                    continue;
                }
                println!("Piece At {} = {}", SQ(sq as u8), self.board_pieces[p][sq]);
            }
        }
    }

    /// White-centric evaluation: positive is good for White.
    pub fn white_score(&mut self) -> EvalVal {
        let mut score = Score::ZERO;

        let white_score = self.score_player_new::<WhiteType>();
        let black_score = self.score_player_new::<BlackType>();

        score += white_score - black_score;

        if let Some(dbg) = self.tracer.trace() {
            let player = Player::White;
            dbg.set_eval(EvalPasses::Total, player, score);
            println!("{dbg}");
        }

        // Tapered eval: combine midgame and endgame based on phase.
        let mg = score.mg();
        let eg = score.eg();
        let raw_score =
            (mg * self.phase + eg * (MAX_PHASE - self.phase)) / MAX_PHASE;

        raw_score
    }

    /// Score all aspects for one side.
    fn score_player_new<P: PlayerTrait>(&mut self) -> Score {
        let mut score = Score::ZERO;

        let piece_eval = self.score_raw_pieces::<P>();
        score += piece_eval;

        let pawn_structure = self.score_pawn_structure::<P>();
        score += pawn_structure;

        let king_safety = self.score_king_safety::<P>();
        score += king_safety;

        let mobility = self.score_piece_mobility::<P>();
        score += mobility;

        let misc = self.score_misc_piece_features::<P>();
        score += misc;

        if let Some(dbg) = self.tracer.trace() {
            let player = P::player();
            dbg.set_eval(EvalPasses::Material, player, piece_eval);
            dbg.set_eval(EvalPasses::PawnStructure, player, pawn_structure);
            dbg.set_eval(EvalPasses::King, player, king_safety);
            dbg.set_eval(EvalPasses::Mobility, player, mobility);
            dbg.set_eval(EvalPasses::Misc, player, misc);
        }

        score
    }

    /// Pawn structure: passed pawns, doubled pawns, advancement, wing pawns.
    fn score_pawn_structure<P: PlayerTrait>(&self) -> Score {
        let mut score = Score::ZERO;
        let player = P::player();
        let enemy_pawns = self.board.piece_bb(player.other_player(), PieceType::P);
        let my_pawns = self.board.piece_bb(player, PieceType::P);

        let mut mpb = my_pawns;
        while let Some((pawn_sq, _bb)) = mpb.pop_some_lsb_and_bit() {
            // Passed pawns: no enemy pawn on same file ahead.
            let enemy_blocking = enemy_pawns & pawn_sq.file_bb();
            if enemy_blocking.count_bits() == 0 {
                score += PASSED_PAWN_BONUS;
            }

            // Doubled pawns: more than one friendly pawn on same file.
            let friendly_blocking = my_pawns & pawn_sq.file_bb();
            if friendly_blocking.count_bits() > 1 {
                score -= DOUBLE_PAWN_PENALTY;
            }

            // Pawn advancement.
            let rel_rank = player.relative_rank_of_sq(pawn_sq) as usize;
            score += PAWN_ADVANCEMENT_SCORES[rel_rank];

            // Wing pawn advanced.
            if rel_rank >= 4 {
                score += WING_ADVANCE_SCORES[pawn_sq.file() as usize];
            }
        }

        score
    }

    /// Very simple king safety: pawn shield in front of king.
    fn score_king_safety<P: PlayerTrait>(&self) -> Score {
        let player = P::player();
        let king_sq = self.board.king_sq(player);

        let my_pawns_bb = self.board.piece_bb(player, PieceType::P);

        let k_file = king_sq.file() as i32;
        let k_rank = king_sq.rank() as i32;

        let mut shield_pawns = 0;

        // Iterate our pawns and check which ones are in the "shield region".
        let mut pawns = my_pawns_bb;
        while let Some((pawn_sq, _)) = pawns.pop_some_lsb_and_bit() {
            let p_file = pawn_sq.file() as i32;
            let p_rank = pawn_sq.rank() as i32;

            if (p_file - k_file).abs() > 1 {
                continue;
            }

            match player {
                Player::White => {
                    // Pawns in front of white king, up to two ranks forward.
                    if p_rank > k_rank && p_rank <= k_rank + 2 {
                        shield_pawns += 1;
                    }
                }
                Player::Black => {
                    // Pawns in front of black king (towards rank 0).
                    if p_rank < k_rank && p_rank >= k_rank - 2 {
                        shield_pawns += 1;
                    }
                }
            }
        }

        if shield_pawns == 0 {
            return Score::ZERO;
        }

        let mg = KING_SHIELD_PAWN_MG * shield_pawns;
        let eg = KING_SHIELD_PAWN_EG * shield_pawns;
        Score::new(mg, eg)
    }

    /// Mobility: how many empty squares our pieces attack.
    fn score_piece_mobility<P: PlayerTrait>(&self) -> Score {
        let mut total = Score::ZERO;
        let player = P::player();

        let empty_squares = !self.all_bb;

        for (sq, piece) in self.board.get_piece_locations() {
            if piece.player_lossy() != player {
                continue;
            }
            let attacks = self.board.attacks_from(piece.type_of(), sq, player) & empty_squares;
            let cnt = attacks.count_bits() as usize;
            total += MOBILITY_BONUS[piece.type_of() as usize][cnt];
        }
        total
    }

    /// Base piece values (MG/EG), *not* pre-scaled by phase.
    #[inline(always)]
    fn get_raw_piece_val(&self, piece: PieceType) -> Score {
        match piece {
            PieceType::P => Score::new(PAWN_MG, PAWN_EG),
            PieceType::N => Score::new(KNIGHT_MG, KNIGHT_EG),
            PieceType::B => Score::new(BISHOP_MG, BISHOP_EG),
            PieceType::R => Score::new(ROOK_MG, ROOK_EG),
            PieceType::Q => Score::new(QUEEN_MG, QUEEN_EG),
            _ => Score::ZERO,
        }
    }

    /// Pure material + some simple piece-square-ish tweaks (centralization).
    fn score_raw_pieces<P: PlayerTrait>(&mut self) -> Score {
        let mut score = Score::ZERO;
        let player = P::player();

        let pawn_score =
            self.get_raw_piece_val(PieceType::P) * self.board.count_piece(player, PieceType::P);
        score += pawn_score;

        let knight_score =
            self.get_raw_piece_val(PieceType::N) * self.board.count_piece(player, PieceType::N);
        score += knight_score;

        let bishop_score =
            self.get_raw_piece_val(PieceType::B) * self.board.count_piece(player, PieceType::B);
        score += bishop_score;

        let rook_score =
            self.get_raw_piece_val(PieceType::R) * self.board.count_piece(player, PieceType::R);
        score += rook_score;

        let queen_score =
            self.get_raw_piece_val(PieceType::Q) * self.board.count_piece(player, PieceType::Q);
        score += queen_score;

        // Add a small centralization bonus for knights and bishop activity.
        let center_bonus = self.score_piece_centralization::<P>();
        score += center_bonus;

        if let Some(dbg) = self.tracer.trace() {
            dbg.set_eval(EvalPasses::Pawn, player, pawn_score);
            dbg.set_eval(EvalPasses::Knight, player, knight_score);
            dbg.set_eval(EvalPasses::Bishop, player, bishop_score);
            dbg.set_eval(EvalPasses::Rook, player, rook_score);
            dbg.set_eval(EvalPasses::Queen, player, queen_score);
            dbg.set_eval(EvalPasses::PawnStructure, player, center_bonus);
        }
        score
    }

    /// Simple centralization / activity bonus for knights and bishops.
    fn score_piece_centralization<P: PlayerTrait>(&self) -> Score {
        let mut mg: EvalVal = 0;
        let player = P::player();

        for (sq, piece) in self.board.get_piece_locations() {
            if piece.player_lossy() != player {
                continue;
            }
            let f = sq.file() as i32; // 0..7
            let r = sq.rank() as i32; // 0..7
            let df = (f - 3).abs();   // distance from center files d/e
            let dr = (r - 3).abs();   // distance from center ranks 4/5-ish
            let dist = df + dr;

            match piece.type_of() {
                PieceType::N => {
                    // Encourage central knights.
                    let bonus = (6 - dist).max(0) as EvalVal * KNIGHT_CENTER_MG;
                    mg += bonus;
                }
                PieceType::B => {
                    // Bishops: mild bonus for central-ish and active.
                    let bonus = (4 - df).max(0) as EvalVal * BISHOP_MOBILITY_MG;
                    mg += bonus;
                }
                _ => {}
            }
        }

        Score::new(mg, 0)
    }

    /// Misc piece features: bishop pair, rooks on open/semi-open files.
    fn score_misc_piece_features<P: PlayerTrait>(&self) -> Score {
        let player = P::player();
        let enemy = player.other_player();

        let mut score = Score::ZERO;

        // Bishop pair
        let bishop_count = self.board.count_piece(player, PieceType::B);
        if bishop_count >= 2 {
            score += Score::new(BISHOP_PAIR_MG, BISHOP_PAIR_EG);
        }

        // Rooks on open/semi-open files.
        let my_pawns = self.board.piece_bb(player, PieceType::P);
        let enemy_pawns = self.board.piece_bb(enemy, PieceType::P);

        for (sq, piece) in self.board.get_piece_locations() {
            if piece.player_lossy() != player {
                continue;
            }
            if piece.type_of() != PieceType::R {
                continue;
            }

            let file_bb = sq.file_bb();
            let my_on_file = (my_pawns & file_bb).count_bits();
            let enemy_on_file = (enemy_pawns & file_bb).count_bits();

            if my_on_file == 0 && enemy_on_file == 0 {
                score += Score::new(ROOK_OPEN_FILE_MG, ROOK_OPEN_FILE_EG);
            } else if my_on_file == 0 && enemy_on_file > 0 {
                score += Score::new(ROOK_SEMI_OPEN_FILE_MG, ROOK_SEMI_OPEN_FILE_EG);
            }
        }

        score
    }

    #[inline(always)]
    fn get_piece_on_sq(&self, sq: SQ, player: Player) -> PieceType {
        self.board_pieces[player as usize][sq.0 as usize]
    }

    #[inline(always)]
    #[allow(unused)]
    fn get_piece_val_on_sq(&self, sq: SQ, player: Player) -> Score {
        self.get_raw_piece_val(self.get_piece_on_sq(sq, player))
    }
}

#[inline(always)]
#[allow(unused)]
fn score_magnitude(s: &Score) -> EvalVal {
    s.mg() + s.eg()
}

#[inline(always)]
#[allow(unused)]
fn div_score(a: Score, b: Score) -> Score {
    Score(a.mg() / b.mg().max(1), a.eg() / b.eg().max(1))
}

#[inline(always)]
#[allow(unused)]
fn div_u8(a: Score, b: u8) -> Score {
    Score(a.mg() / b as EvalVal, a.eg() / b as EvalVal)
}

#[inline(always)]
#[allow(unused)]
fn count_pieces<P: PlayerTrait>(board: &Board, ptype: PieceType) -> MyVal {
    board.count_piece(P::player(), ptype) as MyVal
}

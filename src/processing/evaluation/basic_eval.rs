
use pleco::{core::{mono_traits::{BlackType, PlayerTrait, WhiteType}, score::Score}, Board, PieceType, Player, SQ};


use crate::processing::{consts::{EvalVal, MyVal, BISHOP_EG, BISHOP_MG, KNIGHT_EG, KNIGHT_MG, MAX_PHASE, MOBILITY_BONUS, PAWN_EG, PAWN_MG, QUEEN_EG, QUEEN_MG, ROOK_EG, ROOK_MG}, debug::{EvalDebugger, EvalPasses, Tracing}, tables::{material::Material, pawn_table::PawnTable}};


fn phase(board: &Board) -> EvalVal {
    let midgame_limit = 15258;
    let endgame_limit  = 3915;
    let mut npm = board.non_pawn_material_all();
    npm = endgame_limit.max(npm.min(midgame_limit));
    ((((npm - endgame_limit) * MAX_PHASE as i32) / (midgame_limit - endgame_limit)) as EvalVal) << 0
}

pub struct BasicEvaluator <'a, T: Tracing<EvalDebugger>> {
    board: &'a Board,
    phase: EvalVal,
    tracer: T,
    // pawn_entry: &'a mut PawnEntry,
    // material_entry: &'a mut MaterialEntry,
    board_pieces: [[PieceType; 64]; 2],
}

impl <'a, T: Tracing<EvalDebugger>> BasicEvaluator <'a, T> {
    pub fn new(
        board: &'a Board,
        trace: T, 
        _pawn_table: &'a mut PawnTable,
        _material: &'a mut Material,
    ) -> Self {
        // let pawn_entry = { pawn_table.probe(&board) };
        // let material_entry = { material.probe(&board) };

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
            // pawn_entry,
            // material_entry,
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

    pub fn white_score(&mut self) -> EvalVal {
        let mut score = Score::ZERO;

        let player_score = 
            self.score_player_new::<WhiteType>() -
            self.score_player_new::<BlackType>();

        score += player_score;


        if let Some(dbg) = self.tracer.trace() {
            let player = Player::White;
        
            // dbg.set_eval(EvalPasses::Material, player, self.board.psq());
            // dbg.set_eval(EvalPasses::Imbalance, player, self.material_entry.score());
            // dbg.set_two_eval(
            //     EvalPasses::Pawn, 
            //     self.pawn_entry.pawns_score(Player::White), 
            //     self.pawn_entry.pawns_score(Player::Black),
            // );
            dbg.set_eval(EvalPasses::Total, player, score);
            println!("{dbg}");
        }

        let raw_score = (score.mg() * self.phase + score.eg() * (MAX_PHASE - self.phase)) / MAX_PHASE;

        raw_score
    }

    fn score_player_new<P: PlayerTrait>(&mut self) -> Score {

        let mut score = Score::ZERO;

        let piece_eval = self.score_raw_pieces::<P>();
        score += piece_eval;

        let king_safety = self.score_king_safety::<P>();
        score += king_safety;

        let mobility = self.score_piece_mobility::<P>();
        score += mobility;

        // let pawn_worth = (PAWN_MG * self.phase + PAWN_EG * (MAX_PHASE - self.phase)) / MAX_PHASE;
        if let Some(dbg) = self.tracer.trace() {
            let player = P::player();
            dbg.set_eval(EvalPasses::Material, player, piece_eval);
            dbg.set_eval(EvalPasses::King, player, king_safety);
            dbg.set_eval(EvalPasses::Mobility, player, mobility);
        }
        score
    }

    fn score_king_safety<P: PlayerTrait>(
        &self
    ) -> Score {
        Score::ZERO
    }
    fn score_piece_mobility<P: PlayerTrait>(
        &self
    ) -> Score {

        let mut total = Score::ZERO;
        let player = P::player();

        let empty_squares = !self.board.piece_bb_both_players(PieceType::All);

        // println!("Evaluating Piece Mobility for {}", P::player());
        for (sq, piece) in self.board.get_piece_locations() {
           if piece.player_lossy() != player {
            continue;
           }
           let num_attacks = self.board.attacks_from(piece.type_of(), sq, player) & empty_squares;
           total += MOBILITY_BONUS[piece.type_of() as usize][num_attacks.count_bits() as usize];
        //    let mob = MOBILITY_BONUS[piece.type_of() as usize][num_attacks.count_bits() as usize];
        //    total += mob;
        //    println!("Piece = {piece} Attacks {} Mobility Score = {}", num_attacks.count_bits(), score_str(mob));
        }
        total
    }

    fn get_raw_piece_val(&self, piece: PieceType) -> Score {

        let res = match piece {
            PieceType::P => {
                Score::new(PAWN_MG * self.phase, (MAX_PHASE - self.phase) * PAWN_EG)
            }
            PieceType::N => {
                Score::new(KNIGHT_MG * self.phase, (MAX_PHASE - self.phase) * KNIGHT_EG)
            }
            PieceType::B => {
                Score::new(BISHOP_MG * self.phase, (MAX_PHASE - self.phase) * BISHOP_EG)
            }
            PieceType::R => {
                Score::new(ROOK_MG * self.phase, (MAX_PHASE - self.phase) * ROOK_EG)
            }
            PieceType::Q => {
                Score::new(QUEEN_MG * self.phase, (MAX_PHASE - self.phase) * QUEEN_EG)
            }
            _ => { return Score::ZERO }
        };
    
        div_u8(res, MAX_PHASE as u8)
    }

    fn score_raw_pieces<P: PlayerTrait>(
        &mut self
    ) -> Score {

        let mut score = Score::ZERO;
        let player = P::player();

        //Each raw score is the value of that piece relative to the game phase.
        //So a rook with only 20 pieces left is worth more like 560 then 500
        let pawn_score = 
            self.get_raw_piece_val(PieceType::P) *
            self.board.count_piece(player, PieceType::P);
        score += pawn_score;

        let knight_score = 
            self.get_raw_piece_val(PieceType::N) *
            self.board.count_piece(player, PieceType::N);
        score += knight_score;

        let bishop_score = 
            self.get_raw_piece_val(PieceType::B) *
            self.board.count_piece(player, PieceType::B);
        score += bishop_score;

        let rook_score = 
            self.get_raw_piece_val(PieceType::R) *
            self.board.count_piece(player, PieceType::R);
        score += rook_score;

        let queen_score = 
            self.get_raw_piece_val(PieceType::Q) *
            self.board.count_piece(player, PieceType::Q);
        score += queen_score;

        if let Some(dbg) = self.tracer.trace() {
            dbg.set_eval(EvalPasses::Pawn, player, pawn_score);
            dbg.set_eval(EvalPasses::Knight, player, knight_score);
            dbg.set_eval(EvalPasses::Bishop, player, bishop_score);
            dbg.set_eval(EvalPasses::Rook, player, rook_score);
            dbg.set_eval(EvalPasses::Queen, player, queen_score);
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
fn div_u8(a: Score, b: u8) -> Score {
    Score(a.mg() / b as EvalVal, a.eg() / b as EvalVal)
}
#[inline(always)]
#[allow(unused)]
fn count_pieces<P: PlayerTrait>(board: &Board, ptype: PieceType) -> MyVal {
    board.count_piece(P::player(), ptype) as MyVal
}

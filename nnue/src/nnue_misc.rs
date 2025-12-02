use pleco::{Board, Piece, PieceType, Player, SQ};

use crate::{constants::LAYER_STACKS, nnue_utils::{format_cp_aligned_dot, to_cp}};

pub struct EvalTrace {
    pub selected_bucket: usize,
    pub side_to_move: Player,
    pub psqt: [i32; LAYER_STACKS],
    pub positional: [i32; LAYER_STACKS],
}
impl EvalTrace {
    pub fn new() -> Self {
        Self {
            selected_bucket: 0,
            side_to_move: Player::White,
            psqt: [0; LAYER_STACKS],
            positional: [0; LAYER_STACKS],
        }
    }
    pub fn print(&self, board: &Board) {
        println!("EvalTrace");
        println!("NNUE Network Contributions ({} to move)", self.side_to_move);
        println!("FEN: {}", board.fen());

        let spacing = 13;

        let linspace: &str = &"-".repeat(spacing);

        println!("+{}+{}+{}+{}+", linspace, linspace, linspace, linspace);
        println!(
            "|{:^spacing$}|{:^spacing$}|{:^spacing$}|{:^spacing$}|",
            "Bucket", "Material", "Positional", "Total"
        );
        println!(
            "|{:^spacing$}|{:^spacing$}|{:^spacing$}|{:^spacing$}|",
            "", "(PSQT)", "(Layers)", ""
        );
        println!("+{}+{}+{}+{}+", linspace, linspace, linspace, linspace);

        for bucket in 0..LAYER_STACKS {
            let total = self.psqt[bucket] + self.positional[bucket];
            println!(
                "|{:^spacing$}|{:^spacing$}|{:^spacing$}|{:^spacing$}|{}",
                bucket,
                format_cp_aligned_dot(self.psqt[bucket], board),
                format_cp_aligned_dot(self.positional[bucket], board),
                format_cp_aligned_dot(total, board),
                if bucket == self.selected_bucket {
                    " <-- selected"
                } else {
                    ""
                },
            );
        }
        println!("+{}+{}+{}+{}+", linspace, linspace, linspace, linspace);

        let mut nnue_eval = self.psqt[self.selected_bucket] + self.positional[self.selected_bucket];
        if board.turn() == Player::Black {
            nnue_eval = -nnue_eval;
        }
        println!("NNUE Evaluation            {} (white side)", 0.01 * to_cp(nnue_eval, &board));
    }
}


/// DirtyPiece is the “what changed” record passed to the NNUE updater. It captures up to three piece changes from a move:
/// dirty_num: how many entries are valid.
/// piece[3]: which piece was involved (one per change).
/// from[3], to[3]: origin and destination squares for each change (may be SQ_NONE when a piece is created/removed).
/// Typical cases:
/// Normal move: 1 entry (moved piece from→to).
/// Capture: 2 entries (mover, captured piece to SQ_NONE).
/// Promotion with capture: up to 3 entries (pawn removed from, captured piece removed, promoted piece added). This lets the NNUE accumulator update incrementally without rebuilding.
#[derive(Clone, Debug)]
pub struct DirtyPiece {
    // fill with your move deltas, from/to, piece types, etc.
    // e.g., pub from: [Option<Square>; 2], pub to: [Option<Square>; 2], pub piece: [Piece; 2]
    pub dirty_num: usize,
    pub piece: [pleco::Piece; 3],
    pub from: [pleco::SQ; 3],
    pub to: [pleco::SQ; 3],
}

impl Default for DirtyPiece {
    fn default() -> Self {
        Self {
            dirty_num: 0,
            piece: [pleco::Piece::None; 3],
            from: [pleco::SQ::NONE; 3],
            to: [pleco::SQ::NONE; 3],
        }
    }
}

impl DirtyPiece {
    pub fn from_move(board: &Board, mv: pleco::BitMove) -> Self {
        let mut dp = DirtyPiece::default();
// DirtyPiece (types.h (line 276)) tracks up to three board changes per move: dirty_num says how many entries are valid, piece[i] is the piece type+color, and from[i]/to[i] are its source/destination (either may be SQ_NONE for creation/removal).
// Ordinary moves (non-capture, non-promotion, non-castle) fill only entry 0 in do_move: piece[0]=moving piece, from[0]=from square, to[0]=to square, dirty_num=1 (position.cpp (line 820)).
// Captures (including en passant) add entry 1: piece[1]=captured piece, from[1]=capture square, to[1]=SQ_NONE, and dirty_num becomes 2 (position.cpp (lines 771-779)).
// Promotions replace the pawn and add the promoted piece: the pawn entry’s to[0] is set to SQ_NONE, and a new entry piece[dirty_num]=promotion piece, from=SQ_NONE, to=promotion square is appended before bumping dirty_num (position.cpp (lines 838-851)). If the promotion also captured something, the capture entry remains as above, so you can have 3 entries (moved pawn, captured piece, new promoted piece).
// Castling skips the generic block and instead do_castling<true> writes two entries: king move and rook move, with dirty_num=2 (position.cpp (lines 909-920)).
// These records are then consumed by NNUE to know exactly which features to update incrementally.


        let us = board.turn();
        let them = !us;

        let from_sq = mv.get_src();
        let to_sq = mv.get_dest();

        let pc = board.piece_at_sq(from_sq);
        let captured_pc = if mv.is_en_passant() {
            Piece::make_lossy(them, PieceType::P)
        } else {
            board.piece_at_sq(to_sq)
        };

        dp.dirty_num = 1;

        if mv.is_castle() {
            
            let king_side = to_sq > from_sq;
            let rfrom = to_sq; // Castling is encoded as "king captures friendly rook"
            let rto = relative_square(us, if king_side { SQ::F1 } else { SQ::D1 });
            let to = relative_square(us, if king_side { SQ::G1 } else { SQ::C1 });

            dp.piece[0] = Piece::make_lossy(us, PieceType::K);
            dp.from[0] = from_sq;
            dp.to[0] = to;
            dp.piece[1] = Piece::make_lossy(us, PieceType::R);
            dp.from[1] = rfrom;
            dp.to[1] = rto;
            dp.dirty_num = 2;
        } 
        // Pleco treats castling as a king move that "captures" the rook, so we need to handle that case separately from normal capture
        else if captured_pc != Piece::None {
            let mut capsq = to_sq;

            if captured_pc.type_of() == PieceType::P {
                if mv.is_en_passant() {
                    capsq = SQ((capsq.0 as i8 - pawn_push(us)) as u8);
                    assert!(pc == Piece::make_lossy(us, PieceType::P));
                    assert!(to_sq == board.ep_square());
                    assert!(board.piece_at_sq(to_sq) == Piece::None);
                    assert!(board.piece_at_sq(capsq) == Piece::make_lossy(them, PieceType::P));
                }
            }

            dp.dirty_num = 2;  // 1 piece moved, 1 piece captured
            dp.piece[1]  = captured_pc;
            dp.from[1]   = capsq;
            dp.to[1]     = SQ::NONE;
        }

        if !mv.is_castle() {
            dp.piece[0] = pc;
            dp.from[0] = from_sq;
            dp.to[0] = to_sq;
        }

        if pc.type_of() == PieceType::P && mv.is_promo() {
            let promo_type = mv.promo_piece();
            let promo_pc = Piece::make_lossy(us, promo_type);

            dp.to[0] = SQ::NONE; // Pawn is removed
            dp.piece[dp.dirty_num] = promo_pc;
            dp.from[dp.dirty_num] = SQ::NONE;
            dp.to[dp.dirty_num] = to_sq;
            dp.dirty_num += 1;
        }
        dp
    }
}

fn relative_square(player: Player, sq: pleco::SQ) -> pleco::SQ {
    SQ(sq.0 ^ (player as u8 * 56))
}

fn pawn_push(player: Player) -> i8 {
    match player {
        Player::White => 8,
        Player::Black => -8,
    }
}

#[cfg(test)]
mod tests {
    use pleco::{Piece, PieceType, Player, SQ};

    #[test]
    fn test_quiet_move() {

        let board = pleco::Board::start_pos();
        let mv = board.generate_moves_of_type(pleco::core::GenTypes::Quiets);

        for m in mv {
            let dp = super::DirtyPiece::from_move(&board, m);
        
            assert!(dp.dirty_num == 1);
            assert!(dp.piece[0] == board.piece_at_sq(m.get_src()));
            assert!(dp.from[0] == m.get_src());
            assert!(dp.to[0] == m.get_dest());
        
            for i in 1..3 {
                assert!(dp.piece[i] == Piece::None);
                assert!(dp.from[i] == SQ::NONE);
                assert!(dp.to[i] == SQ::NONE);
            }
        }
    }

    #[test]
    fn test_white_capture_move() {

        // All white piece types can capture something in this position
        let board = pleco::Board::from_fen("rnbqk2r/2pppppp/7n/Rp3b1Q/2P1PKB1/6N1/1P1P1PPP/1NB4R w kq - 0 1").unwrap();
        let mv = board.generate_moves_of_type(pleco::core::GenTypes::Captures);

        for m in mv {
            let dp = super::DirtyPiece::from_move(&board, m);
        
            assert!(dp.dirty_num == 2);
            assert!(dp.piece[0] == board.piece_at_sq(m.get_src()));
            assert!(dp.from[0] == m.get_src());
            assert!(dp.to[0] == m.get_dest());

            assert!(dp.piece[1] == board.piece_at_sq(m.get_dest()));
            assert!(dp.from[1] == m.get_dest());
            assert!(dp.to[1] == SQ::NONE);

        
            assert!(dp.piece[2] == Piece::None);
            assert!(dp.from[2] == SQ::NONE);
            assert!(dp.to[2] == SQ::NONE);
        }
    }
    #[test]
    fn test_black_capture_move() {

        // All black piece types can capture something in this position
        let board = pleco::Board::from_fen("rn3b2/ppp1p1pp/2bp1n2/4kp2/N1q1P2r/8/PPPP1PPP/R1BQKBNR b KQ - 0 1").unwrap();
        let mv = board.generate_moves_of_type(pleco::core::GenTypes::Captures);

        for m in mv {
            let dp = super::DirtyPiece::from_move(&board, m);
        
            assert!(dp.dirty_num == 2);
            assert!(dp.piece[0] == board.piece_at_sq(m.get_src()));
            assert!(dp.from[0] == m.get_src());
            assert!(dp.to[0] == m.get_dest());

            assert!(dp.piece[1] == board.piece_at_sq(m.get_dest()));
            assert!(dp.from[1] == m.get_dest());
            assert!(dp.to[1] == SQ::NONE);

        
            assert!(dp.piece[2] == Piece::None);
            assert!(dp.from[2] == SQ::NONE);
            assert!(dp.to[2] == SQ::NONE);
        }
    }

    #[test]
    fn test_normal_promo() {
        let fens = vec![
            (Player::White, "3k4/6P1/8/8/8/8/8/5K2 w - - 0 1"),
            (Player::Black, "3k1r2/6P1/8/8/8/8/2p3K1/8 b - - 0 1")
        ];
        
        for (player, fen) in fens {
            let board = pleco::Board::from_fen(fen).unwrap();
            let mv = board.generate_moves_of_type(pleco::core::GenTypes::All);

            for m in mv {
                if !m.is_promo() {
                    continue
                }
                let dp = super::DirtyPiece::from_move(&board, m);
            
                assert!(dp.dirty_num == 2);
                assert!(dp.piece[0] == board.piece_at_sq(m.get_src()));
                assert!(dp.from[0] == m.get_src());
                assert!(dp.to[0] == SQ::NONE);

                assert!(dp.piece[1] == Piece::make_lossy(player, m.promo_piece()));
                assert!(dp.from[1] == SQ::NONE);
                assert!(dp.to[1] == m.get_dest());

            
                assert!(dp.piece[2] == Piece::None);
                assert!(dp.from[2] == SQ::NONE);
                assert!(dp.to[2] == SQ::NONE);
            }
        }

    }

    #[test]
    fn test_capture_promo() {

        //white ""
        let fens = vec![
            (Player::White, "3k1r2/6P1/8/8/8/8/6K1/8 w - - 0 1"),
            (Player::Black, "3k1r2/6P1/8/8/8/8/3p2K1/4N3 b - - 0 1")
        ];
        
        for (player, fen) in fens {
            let board = pleco::Board::from_fen(fen).unwrap();
            let mv = board.generate_moves_of_type(pleco::core::GenTypes::All);

            for m in mv {
                if !m.is_promo() || !m.is_capture() {
                    continue
                }
                let dp = super::DirtyPiece::from_move(&board, m);
                assert!(dp.dirty_num == 3);
                // The pawn captures dest
                assert!(dp.piece[0] == board.piece_at_sq(m.get_src()));
                assert!(dp.from[0] == m.get_src());
                assert!(dp.to[0] == SQ::NONE);

                // The captured piece is removed
                assert!(dp.piece[1] == board.piece_at_sq(m.get_dest()));
                assert!(dp.from[1] == m.get_dest());
                assert!(dp.to[1] == SQ::NONE);
            
                // The promoted piece is added
                assert!(dp.piece[2] == Piece::make_lossy(player, m.promo_piece()));
                assert!(dp.from[2] == SQ::NONE);
                assert!(dp.to[2] == m.get_dest());
            }
        }

    }
    #[test]
    fn test_en_passent() {
        let fens = vec![
            (Player::White, "4k3/4p3/8/3PPp2/8/8/8/4K3 w - f6 1 1"),
            (Player::Black, "5k2/8/8/8/3ppP2/8/4P3/5K2 b - f3 0 1")
        ];
            for (player, fen) in fens {
            let board = pleco::Board::from_fen(fen).unwrap();
            let mv = board.generate_moves_of_type(pleco::core::GenTypes::All);

            for m in mv {
                if !m.is_en_passant() {
                    continue
                }
                println!("Testing move: {}", m);
                println!("DEST SQ: {}", m.get_dest());
                let dp = super::DirtyPiece::from_move(&board, m);
                assert!(dp.dirty_num == 2);
                // The pawn moves to dest
                assert!(dp.piece[0] == board.piece_at_sq(m.get_src()));
                assert!(dp.from[0] == m.get_src());
                assert!(dp.to[0] == m.get_dest());

                let ep_square = if player == Player::White {
                    SQ(m.get_dest().0 - 8)
                } else {
                    SQ(m.get_dest().0 + 8)
                };

                // The captured pawn is removed
                assert!(dp.piece[1] == Piece::make_lossy(!player, PieceType::P));
                assert!(dp.from[1] == ep_square);
                assert!(dp.to[1] == SQ::NONE);
            
                assert!(dp.piece[2] == Piece::None);
                assert!(dp.from[2] == SQ::NONE);
                assert!(dp.to[2] == SQ::NONE);
            }
        }

}

    #[test]
    fn test_white_castling() {

        let board = pleco::Board::from_fen("r3k3/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1").unwrap();
        let mv = board.generate_moves();

        for m in mv {
            if !m.is_castle() {
                continue;
            }
            let dp = super::DirtyPiece::from_move(&board, m);
        
            assert!(dp.dirty_num == 2);
            assert!(dp.piece[0] == board.piece_at_sq(m.get_src()));
            assert!(dp.piece[1] == Piece::make_lossy(pleco::Player::White, pleco::PieceType::R));
            
            assert!(dp.from[0] == SQ::E1);
            
            if m.is_queen_castle() {
                assert!(dp.to[0] == SQ::C1);
                assert!(dp.from[1] == SQ::A1);
                assert!(dp.to[1] == SQ::D1);
            } else {
                assert!(dp.to[0] == SQ::G1);
                assert!(dp.from[1] == SQ::H1);
                assert!(dp.to[1] == SQ::F1);
            }

            assert!(dp.piece[2] == Piece::None);
            assert!(dp.from[2] == SQ::NONE);
            assert!(dp.to[2] == SQ::NONE);
        }
    }
    #[test]
    fn test_black_castling() {

        let board = pleco::Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1").unwrap();
        let mv = board.generate_moves();

        for m in mv {
            if !m.is_castle() {
                continue;
            }
            let dp = super::DirtyPiece::from_move(&board, m);

            assert!(dp.dirty_num == 2);
            assert!(dp.piece[0] == board.piece_at_sq(m.get_src()));
            assert!(dp.piece[1] == Piece::make_lossy(pleco::Player::Black, pleco::PieceType::R));
            
            assert!(dp.from[0] == SQ::E8);
            
            if m.is_queen_castle() {
                assert!(dp.to[0] == SQ::C8);
                assert!(dp.from[1] == SQ::A8);
                assert!(dp.to[1] == SQ::D8);
            } else {
                assert!(dp.to[0] == SQ::G8);
                assert!(dp.from[1] == SQ::H8);
                assert!(dp.to[1] == SQ::F8);
            }

            assert!(dp.piece[2] == Piece::None);
            assert!(dp.from[2] == SQ::NONE);
            assert!(dp.to[2] == SQ::NONE);
        }
    }
}

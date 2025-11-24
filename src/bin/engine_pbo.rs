// src/bin/engine_pgo.rs

use std::time::Instant;

use engine::{debug::{NoTrace, Tracing}, final_search::MySearcher};
use pleco::{BitMove, Board};

/// PGO depths — tuned for "realistic but not insane" workloads.
const OPENING_DEPTH: u8 = 8;
const MIDGAME_DEPTH: u8 = 9;
const ENDGAME_DEPTH: u8 = 12;
const TACTICAL_DEPTH: u8 = 10;
const SELFPLAY_DEPTHS: [u8; 3] = [6, 8, 10];

/// How many times to search each FEN (per depth).
const REPEATS_PER_FEN: usize = 1;

/// Self-play parameters.
const SELFPLAY_GAMES: usize = 3;
const SELFPLAY_MAX_PLIES: usize = 40;

/// A small starter set of FENs.
/// - First N: opening-ish positions
/// - Last M: more middlegame-ish
///
/// For serious PGO, you want hundreds or thousands of positions.
/// Expand this array (or load from a file) with real FENs from an
/// opening book / database.
const FENS_OPENING: &[&str] = &[
    // ---- STARTING + EARLY OPENINGS ----
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
    "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq - 0 1",
    "rnbqkbnr/pppppppp/8/8/2P5/8/PPP2PPP/RNBQKBNR b KQkq - 0 1",
    "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 0 1",

    // ---- RUY LOPEZ ----
    "r1bqkbnr/pppp1ppp/2n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 2 4",
    "r1bq1rk1/pppp1ppp/2n5/1Bb1p3/4P3/5N2/PPPP1PPP/RNBQ1RK1 w - - 6 6",
    "r2q1rk1/ppp2ppp/2n1bn2/1B1pp3/3PP3/2N2N2/PPP2PPP/R1BQ1RK1 w - - 5 8",

    // ---- ITALIAN / GIUOCO PIANO ----
    "r1bqkbnr/pppp1ppp/2n5/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
    "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/3P1N2/PPP2PPP/RNBQ1RK1 b kq - 5 6",

    // ---- SICILIAN ----
    "rnbqkbnr/1pp1pppp/p7/1B1p4/3P4/5N2/PPP1PPPP/RNBQK2R b KQkq - 1 4",
    "rnbqkb1r/pp2pppp/5n2/2pp4/3P4/2N1PN2/PPP2PPP/R1BQKB1R w KQkq - 3 5",
    "r1bq1rk1/pp1n1ppp/2pbpn2/3p4/3P4/2NBPN2/PPQ2PPP/R3K2R w KQ - 4 10",
    "r1bq1rk1/1p1n1ppp/p1pbpn2/3p4/3P4/1PNBPN2/P1Q2PPP/R3K2R w KQ - 6 11",
    "r2q1rk1/1b1nbppp/p2ppn2/1pp5/3PP3/1BN1BN1P/PP3PP1/R2Q1RK1 w - - 5 11",

    // ---- SICILIAN Sveshnikov ----
    "r1bqkb1r/pp1n1ppp/2p1pn2/8/3NP3/5N2/PPP2PPP/R1BQKB1R w KQkq - 5 6",
    "r2q1rk1/pp1n1ppp/2pbpn2/3p4/3P4/2NBPN2/PPQ2PPP/R3K2R w KQ - 7 11",

    // ---- FRENCH DEFENSE ----
    "rnbqkbnr/ppp1pppp/3p4/8/3PP3/2N5/PPP2PPP/R1BQKBNR b KQkq - 2 3",
    "rnbqk2r/ppp1bppp/3p1n2/4p3/3PP3/2N2N2/PPP2PPP/R1BQKB1R w KQkq - 6 5",
    "r1bq1rk1/ppp1bppp/3p1n2/4p3/2BPP3/2N2N2/PPP2PPP/R1BQ1RK1 w - - 7 7",

    // ---- CARO-KANN ----
    "rnbqkb1r/pp2pppp/2p2n2/3p4/3P4/3B1N2/PPP1PPPP/RNBQK2R w KQkq - 4 6",
    "r1bqkb1r/pp1n1ppp/2p1pn2/3p4/3P4/3BPN2/PP3PPP/RNBQ1RK1 w kq - 7 8",

    // ---- PIRC / MODERN ----
    "rnbqkb1r/pppppp1p/5np1/8/3PP3/2N2N2/PPP2PPP/R1BQKB1R b KQkq - 3 3",
    "rnbqkb1r/1ppppp1p/p4np1/8/3PP3/2N2N2/PPP2PPP/R1BQKB1R w KQkq - 2 4",

    // ---- SLAV ----
    "rnbqkbnr/pp1ppppp/8/2p5/3PP3/5N2/PPP2PPP/RNBQKB1R b KQkq - 1 2",
    "rnbqkb1r/pp1ppppp/5n2/2p5/3PP3/2N5/PPP2PPP/R1BQKB1R w KQkq - 2 3",
    "rnbqkb1r/1p1ppppp/p4n2/2p5/3PP3/2N5/PPP2PPP/R1BQKB1R w KQkq - 0 4",

    // ---- SEMI-SLAV ----
    "rnbqkb1r/1p1ppppp/p4n2/2p5/3PP3/2N2N2/PPP2PPP/R1BQKB1R b KQkq - 1 4",
    "rnbqkb1r/1p1npppp/p1p2n2/2p5/3PP3/2N2N2/PPP2PPP/R1BQKB1R w KQkq - 2 5",
    "r1bqkb1r/1p1npppp/p1p2n2/2P5/3P4/2N2N2/PP3PPP/R1BQKB1R b KQkq - 0 6",

    // ---- KING’S INDIAN ----
    "rnbqkb1r/pppppp1p/5np1/8/3PP3/2N2N2/PPP2PPP/R1BQKB1R w KQkq - 2 3",
    "rnbq1rk1/pp2ppbp/3p1np1/2p5/3PP3/2N2N2/PP2BPPP/R1BQ1RK1 w - - 4 7",
    "rnbq1rk1/pp2ppbp/3p1np1/8/3PP3/2N1BN2/PP3PPP/R2Q1RK1 b - - 6 8",

    // ---- GRÜNFELD ----
    "rnbqkbnr/ppp1pppp/8/3p4/3PP3/2N5/PPP2PPP/R1BQKBNR b KQkq - 1 2",
    "rnbqk2r/pp2ppbp/3p1np1/2p5/3PP3/2N2N2/PPQ2PPP/R1B1KB1R w KQkq - 4 6",
    "r2qk2r/pp1nppbp/3p1np1/2p5/3PP3/2N2N2/PPQ2PPP/R1B1K2R b KQkq - 5 7",

    // ---- NIMZO-INDIAN ----
    "rnbqk2r/pppp1ppp/4pn2/2b5/3P4/2N2N2/PPP1PPPP/R1BQKB1R w KQkq - 2 4",
    "r1bqk2r/pppp1ppp/2n1pn2/2b5/3P4/2N2N2/PPP1PPPP/R1BQKB1R w KQkq - 4 5",
    "r1bqk2r/pp3ppp/2n1pn2/2bp4/3P4/2NB1N2/PPP1PPPP/R1BQK2R w KQkq - 6 7",

    // ---- QUEEN’S GAMBIT DECLINED ----
    "rnbqkbnr/ppp1pppp/3p4/8/2PP4/5N2/PP2PPPP/RNBQKB1R b KQkq - 0 2",
    "rnbqkb1r/pp3ppp/3ppn2/2p5/2PPP3/2N2N2/PP3PPP/R1BQKB1R w KQkq - 1 5",
    "r1bqkb1r/pp3ppp/2nppn2/2p5/2PPP3/2N2N2/PP2BPPP/R1BQK2R w KQkq - 2 7",

    // ---- QUEEN’S INDIAN ----
    "rnbqkb1r/ppp1pp1p/3p1np1/8/2PPP3/5N2/PP3PPP/RNBQKB1R w KQkq - 3 4",
    "rnbqk2r/ppp1pp1p/3p1np1/8/2PPP3/2N2N2/PP3PPP/R1BQKB1R b KQkq - 5 5",
    "rnbq1rk1/ppp1pp1p/3p1np1/8/2PPP3/2N2N2/PP3PPP/R1BQR1K1 w - - 6 7",

    // ---- ENGLISH OPENING ----
    "rnbqkbnr/pp1ppppp/8/2p5/8/2N5/PPPPPPPP/R1BQKBNR w KQkq - 2 2",
    "rnbqkb1r/pp1ppppp/5n2/2p5/2P5/2N2N2/PP1PPPPP/R1BQKB1R w KQkq - 3 3",
    "r1bqkb1r/pp1ppppp/2n2n2/2p5/2P5/2N2N2/PP1PPPPP/R1BQKB1R w KQkq - 4 4",
];

#[allow(unused)]
pub const EARLY_BALANCED_FENS: &[&str] = &[
    "rnbqkbnr/p1pppppp/8/1p6/8/2N3P1/PPPPPP1P/R1BQKBNR b - - 0 2",
    "rnbqkbnr/ppppp1pp/5p2/8/7P/7N/PPPPPPP1/RNBQKB1R b - - 0 2",
    "1rbqkbnr/ppppppp1/n7/7p/8/P2BP3/1PPP1PPP/RNBQK1NR w - - 0 4",
    "rnbqkbnr/pppppppp/8/8/3P4/5P2/PP2P1PP/RNBQKBNR b - - 0 2",
    "rnbqkb1r/pppppppp/5n2/8/2PP2P1/8/PP2PP1P/RNBQKBNR b - - 0 2",
    "rnbqkb1r/pppp1ppp/4pn2/8/2PP2P1/8/PP2PP1P/RNBQKBNR w - - 0 2",
    "rnbqk1nr/ppppppbp/6p1/8/P2P4/6P1/1P2PP1P/RNBQKBNR w - - 0 2",
    "rnbqkbnr/ppp1pppp/3p4/8/2P2P2/8/PP1PP1PP/RNBQKBNR w - - 0 2",
    "rnbqk1nr/pp1pppbp/2p3p1/8/3P4/5PP1/PPP1P2P/RNBQKBNR w - - 0 3",
    "rnbqkbnr/ppp1pppp/8/3p4/1PP5/P4P2/3P2PP/RNBQKBNR b - - 0 2",
    "rnbqkbnr/pp1ppppp/2p5/8/1PP5/P4P2/3P2PP/RNBQKBNR w - - 0 2",
    "rnbqkbnr/pppp1ppp/8/4p3/1PP5/P4PP1/3P3P/RNBQKBNR w - - 0 2",
    "r1bqkbnr/pppn2pp/3p4/4p3/1PPPP3/P4NP1/7P/RNBQKB1R w - - 0 5",
    "rnbqkbnr/1ppppppp/p7/8/P1P5/7P/1P1PPPP1/RNBQKBNR b - - 0 2",
    "rnbqkbnr/1pp1pppp/p2p4/8/P1P5/7P/1P1PPPP1/RNBQKBNR w - - 0 2",
    "rnbqk1nr/1pp1ppbp/p2p2p1/8/PPPP4/7P/1P3PP1/RNBQKBNR b - - 0 3",
    "rnbqk1nr/1pp2pbp/p2pp1p1/8/PPPP4/7P/1P3PP1/RNBQKBNR w - - 0 4",
    "rnbqk1nr/1pp2pbp/p2pp1p1/8/PPPPP3/7P/1P4P1/RNBQKBNR b - - 0 4",
    "rnbqkbnr/ppp1pppp/3p4/8/3PP3/7P/PPP2PP1/RNBQKBNR b - - 0 2",
    "rnbqkbnr/ppp1pppp/3p4/8/3PP3/7P/PPPN1PP1/R1BQKBNR b - - 0 2",
    "rnbqkb1r/ppp1pppp/5n2/8/3PP3/7P/PPPN1PP1/R1BQKBNR w - - 0 3",
    "rnbqkbnr/ppp1pppp/3p4/8/4PPP1/8/PPPP3P/RNBQKBNR b - - 0 2",
    "rnbqk1nr/ppppppbp/6p1/8/4PPP1/8/PPPP3P/RNBQKBNR w - - 0 2",
    "rnbqk1nr/ppppppbp/6p1/8/4PPP1/7P/PPPP4/RNBQKBNR b - - 0 2",
    "r1bqk1nr/ppppppbp/2n3p1/8/4PPP1/7P/PPPP4/RNBQKBNR w - - 0 3",
    "r1bqk1nr/ppppppbp/2n3p1/8/4PPPP/8/PPPP4/RNBQKBNR b - - 0 3",
    "rnbqk1nr/pp1pppbp/2p3p1/4P3/3P2P1/5P2/PPP4P/RNBQKBNR b - - 0 3",
    "rnbqkbnr/pp2pppp/2p5/3p4/3PP1P1/8/PPP2P1P/RNBQKBNR w - - 0 3",
    "rnbqkbnr/pp2pppp/2p5/3p4/2PPP1P1/8/PP3P1P/RNBQKBNR b - - 0 3",
    "rnb1kbnr/pp2pppp/2pq4/3p4/2PPP1P1/8/PP3P1P/RNBQKBNR w - - 0 4",
    "rnb1kbnr/pp2pppp/2pq4/3pP3/2PP2P1/8/PP3P1P/RNBQKBNR b - - 0 4",
    "rnb1kbnr/pp3ppp/2pq4/3pp3/2PP2P1/8/PP3P1P/RNBQKBNR w - - 0 5",
    "rnb1kbnr/pp3ppp/2pq4/3pp3/2PP2P1/1P6/P4P1P/RNBQKBNR b - - 0 5",
    "rnbqkbnr/1ppppppp/8/p7/8/4PP2/PPPP2PP/RNBQKBNR b - - 0 2",
    "r1bqkbnr/pppppppp/2n5/1B6/4P3/8/PPPP1PPP/RNBQK1NR b KQkq - 2 2",
    "rnbqkbnr/pppppp1p/6p1/8/3P4/6P1/PPP1PP1P/RNBQKBNR b KQkq - 0 2",
    "rnbqkb1r/pppppp1p/5np1/6B1/3P4/6P1/PPP1PP1P/RN1QKBNR b KQkq - 2 3",
];

const FENS_MIDGAME: &[&str] = &[
    // Open center
    "r2q1rk1/pp1nbppp/2n1p3/2bpP3/3P1B2/2N2N2/PPQ2PPP/R3KB1R w KQ - 4 11",
    // IQP position
    "r1bq1rk1/pp3ppp/2n1pn2/2bp4/3P4/2PBPN2/PP3PPP/R1BQ1RK1 w - - 4 9",
    // K-side attack forming
    "r2q1rk1/1b1nbppp/p2ppn2/1pp5/3PP3/1BN1BN1P/PP3PP1/R2Q1RK1 w - - 4 11",
    // Semi-open file pressure
    "r2q1rk1/pp1n1ppp/2pbpn2/3p4/3P4/2NBPN2/PPQ2PPP/R3K2R w KQ - 6 11",
    // Hedgehog structure
    "r1bq1rk1/1p1n1ppp/p1p1pn2/1bp5/3P4/1PNBPN2/PBQ2PPP/R3K2R w KQ - 5 10",
    // French closed center
    "r1bq1rk1/pppn1ppp/3bpn2/3p4/1PPPP3/2N2N2/P2BBPPP/R2Q1RK1 w - - 4 9",
    // Sicilian open
    "r1bq1rk1/pp1nbppp/2n1p3/2bpP3/3P4/2NB1N2/PP3PPP/R2Q1RK1 w - - 6 10",
    // Sicilian Scheveningen-ish
    "r1bq1rk1/pp3ppp/2np1n2/2p1p3/2B1P3/1PN1BP2/P1PQ2PP/R3K2R w KQ - 5 12",
    // Maroczy bind
    "r1bq1rk1/pp3ppp/2np1n2/2p1p3/2BPP3/1PN2P2/P1PQ2PP/R3K2R b KQ - 3 11",
    // Semi-open c-file
    "r2q1rk1/pp1n1ppp/2p1pn2/2bp4/3P4/2NBPN2/PPQ2PPP/2KR3R w - - 6 12",
    // Grünfeld midgame
    "r2q1rk1/pp1nppbp/3p1np1/8/2PP4/2N1PN2/PP3PPP/R1BQ1RK1 w - - 4 8",
    // Nimzo midgame
    "r1bq1rk1/pp3ppp/2n1pn2/2bp4/3P4/2NBPN2/PP2BPPP/R2Q1RK1 w - - 7 8",
    // QGD Tartakower
    // "r1bq1rk1/pp1n1ppp/2pbpn2/3p4/3P4/2NBPN2/PPQ2PPP/R3K2R w KQ - 5 9",
    // // Opposite-side castling attack
    // "r2qk2r/1b1nbppp/p2ppn2/1p6/3PP3/1BN1BN1P/PP3PP1/2RQ1RK1 w kq - 2 12",
    // // Benoni-like
    // "r2q1rk1/1p1n1pbp/p2p2p1/2pPp3/2P1P3/1PN2N2/P2B1PPP/R2Q1RK1 w - - 4 11",
    // // IQP symmetric
    // "r2q1rk1/pp1nbppp/3ppn2/2p5/2PP4/2N1PN2/PP3PPP/R1BQ1RK1 w - - 4 9",
    // // Carlsbad (minority attack structure)
    // "r1bq1rk1/2pn1ppp/p3pn2/1p1p4/1P1P4/P1N1PN2/2P2PPP/R1BQ1RK1 w - - 3 9",
    // // Kingside expansion
    // "r1bq1rk1/pp2ppbp/2np1np1/8/2PPP3/2N1BN2/PP3PPP/R1BQ1RK1 w - - 5 9",
    // // Dynamic center tension
    // "r2q1rk1/pp1n1ppp/2p1pn2/2bp4/3P4/2NBPN2/PPQ2PPP/R3K2R w KQ - 8 12",
    // // Slightly open center with tactics possible
    // "r1bq1rk1/pp2ppbp/2n3p1/2ppP3/3P4/2N2N2/PPP2PPP/R1BQ1RK1 w - - 7 9",
    // "r1bq1rk1/pp3ppp/2n1pn2/2bp4/3P4/2PBPN2/PP3PPP/R1BQ1RK1 w - - 4 9",
    // "r1bq1rk1/2pn1ppp/p3pn2/1p1p4/1P1P4/P1N1PN2/2P2PPP/R1BQ1RK1 w - - 3 9",
    // "r2q1rk1/pp1n1ppp/2p1pn2/2bp4/3P4/2NBPN2/PPQ2PPP/2KR1B1R w - - 6 12",
    // "r1bq1rk1/pp3ppp/2np1n2/2p1p3/2BPP3/1PN2P2/P1PQ2PP/R3K2R b KQ - 3 11",
];
// 8 basic endgames (good for pruning / eval paths)
const FENS_ENDGAME: &[&str] = &[
    // K+P vs K
    "8/8/8/3k4/3P4/8/3K4/8 w - - 0 1",
    // KR vs K
    "8/8/8/3k4/2R5/8/3K4/8 w - - 0 1",
    // KQ vs K
    "8/8/8/3k4/2Q5/8/3K4/8 b - - 0 1",
    // Lucena-like rook endgame
    "8/3k4/8/3P1R2/8/8/3K4/8 w - - 0 1",
    // Philidor-type defensive setup
    "8/8/3k4/8/3P4/8/3KR3/8 b - - 0 1",
    // Opposite-colour bishops with passed pawn
    "8/8/3k4/3P4/3B4/8/3K4/8 w - - 0 1",
    // Knight vs pawn race
    "8/8/3k4/3P4/8/2N5/3K4/8 w - - 0 1",
    // R+P vs R
    "8/3k4/3P4/8/8/3R4/3K4/8 w - - 0 1",
];

// 8 tactical-ish middlegames
const FENS_TACTICAL: &[&str] = &[
    // Typical kingside attack setup
    "r1bq1rk1/ppp2ppp/2np1n2/4p1B1/2B1P3/2NP1N2/PPP2PPP/R2Q1RK1 w - - 6 8",
    // Bxh7+ themes possible
    "r1bq1rk1/ppp2ppp/2np1n2/4p3/2B1P1B1/2NP1N2/PPP2PPP/R2Q1RK1 w - - 8 9",
    // Back-rank / pin motifs
    "r2q1rk1/pp3ppp/2p1bn2/3p4/3P4/2N1PN2/PPQ2PPP/2R2RK1 w - - 4 11",
    // Central pins & piece pressure
    "r1bq1rk1/pp2bppp/2np1n2/2p1p3/2P1P3/2NP1NP1/PP2BPBP/R1BQ1RK1 w - - 5 9",
    // Long diagonal pressure
    "r1bq1rk1/ppp2ppp/2np1n2/4p3/2BPP3/2N2N2/PP3PPP/R1BQ1RK1 w - - 4 8",
    // Exposed king in the centre/queenside
    "r3r1k1/pp3ppp/2p2n2/3q4/3P4/2N1P1P1/PPQ2PBP/2R2RK1 w - - 3 16",
    // Direct attack on h7
    "r1bq1rk1/ppp2ppp/2np1n2/4p3/2B1P1B1/2NP1N2/PPP2PPP/R2Q1RK1 b - - 7 8",
    // Heavy piece tactics on the queenside
    "r2q1rk1/1b1nbppp/p2ppn2/1p6/3NP3/1BN1B3/PPP1QPPP/2KR3R w - - 4 11",

    "r2q1rk1/pp2bppp/2n1pn2/2bp4/3P4/2NBPN2/PPQ2PPP/2KR1B1R w - - 5 10",
    "r1bq1rk1/ppp2ppp/2np1n2/4p3/2BPP3/2N2N2/PP3PPP/R1BQ1RK1 w - - 4 8",
    "r3r1k1/pp3ppp/2p2n2/3q4/3P4/2N1P1P1/PPQ2PBP/2R2RK1 w - - 3 16",
    "r2q1rk1/1b1nbppp/p2ppn2/1p6/3NP3/1BN1B1BP/PPP1QPP1/R3K2R w KQ - 4 11",
    "r1bq1rk1/ppp2ppp/2np1n2/4p1B1/2B1P3/2NP1N2/PPP2PPP/R2Q1RK1 b - - 7 8",
];

fn main() {
    eprintln!("PGO training run starting…");

    let start = Instant::now();
    for fen in FENS_OPENING {
        let b = Board::from_fen(fen).expect("Invalid FEN in PGO set");
    }
    for fen in FENS_MIDGAME {
        let b = Board::from_fen(fen).expect("Invalid FEN in PGO set");
    }
    for fen in FENS_ENDGAME {
        let b = Board::from_fen(fen).expect("Invalid FEN in PGO set");
    }
    for fen in FENS_TACTICAL {
        let b = Board::from_fen(fen).expect("Invalid FEN in PGO set");
    }
    // for fen in EARLY_BALANCED_FENS {
    //     let b = Board::from_fen(fen).expect("Invalid FEN in PGO set");
    // }

    run_position_training();

    run_self_play();

    eprintln!(
        "PGO training run finished in {:.2?}",
        start.elapsed()
    );
}

/// Run searches on a mix of opening and middlegame positions
/// to exercise movegen + search + eval in realistic branches.
fn run_position_training() {
    eprintln!("Running position training…");
    let start = Instant::now();

    // Opening positions
    for &fen in FENS_OPENING {
        train_on_fen(fen, OPENING_DEPTH);
    }
    println!("Opening Took {} seconds", start.elapsed().as_secs_f32());

    println!("Starting Midgame");
    // Middlegame positions
    for &fen in FENS_MIDGAME {
        train_on_fen(fen, MIDGAME_DEPTH);
    }
    println!("Midgame Took {} seconds", start.elapsed().as_secs_f32());
    println!("Starting Endgame");
    // Endgame positions
    for &fen in FENS_ENDGAME {
        train_on_fen(fen, ENDGAME_DEPTH);
    }
    println!("Endgame Took {} seconds", start.elapsed().as_secs_f32());
    println!("Starting Tactical");
    // Tactical positions
    for &fen in FENS_TACTICAL {
        train_on_fen(fen, TACTICAL_DEPTH);
    }
    println!("Tactical Took {} seconds", start.elapsed().as_secs_f32());
}

/// For one FEN, run several searches to give LLVM stable profiles.
fn train_on_fen(fen: &str, depth: u8) {
    let board = Board::from_fen(fen)
        .unwrap_or_else(|_| panic!("Invalid FEN in PGO set: {fen}"));


    for _ in 0..REPEATS_PER_FEN {
        let mut board = Board::from_fen(fen)
            .unwrap_or_else(|_| panic!("Invalid FEN in PGO set: {fen}"));
        // let mut board = board.clone();
        // New searcher every time, like your real API
        let trace = NoTrace::new();
        let mut searcher = MySearcher::new(trace, None);

        let _best: BitMove = searcher.find_best_move(&mut board, depth);
    }
    {
        let mut b = board.clone();
        let mut searcher = MySearcher::new(NoTrace::new(), Some(40)); // 40ms
        let _ = searcher.find_best_move(&mut b, 31);
    }

}

/// Self-play: exercise deeper game flow, TT, repetition logic, etc.
fn run_self_play() {
    eprintln!(
        "Running {SELFPLAY_GAMES} self-play games (depths {:?}, max plies {})…",
        SELFPLAY_DEPTHS,
        SELFPLAY_MAX_PLIES
    );
    let start = Instant::now();

    println!("Playing Self From Start Position");
    for game_idx in 0..SELFPLAY_GAMES {
        let mut board = Board::start_pos();
        self_play_single_game(game_idx, &mut board);
    }
    println!("Start Position Took {} seconds", start.elapsed().as_secs_f32());

    // println!("Playing Self from Balanced Openings");
    // for (game_idx, &fen) in EARLY_BALANCED_FENS.iter().enumerate() {
    //     let mut board = Board::from_fen(fen)
    //         .unwrap_or_else(|_| panic!("Invalid FEN in PGO set: {fen}"));
    //     self_play_single_game(game_idx, &mut board);
    // }
    // println!("Balanced Openings Took {} seconds", start.elapsed().as_secs_f32());

    println!("Playing Self from Tactical Positions");
    for (game_idx, &fen) in FENS_TACTICAL.iter().take(SELFPLAY_GAMES).enumerate() {
        let mut board = Board::from_fen(fen)
            .unwrap_or_else(|_| panic!("Invalid FEN in PGO set: {fen}"));
        self_play_single_game(game_idx, &mut board);
    }
    println!("Tactical Positions Took {} seconds", start.elapsed().as_secs_f32());
    // println!("Playing Self from Midgame Positions");
    // for (game_idx, &fen) in FENS_MIDGAME.iter().enumerate() {
    //     let mut board = Board::from_fen(fen)
    //         .unwrap_or_else(|_| panic!("Invalid FEN in PGO set: {fen}"));
    //     self_play_single_game(game_idx, &mut board);
    // }
    // println!("Midgame Positions Took {} seconds", start.elapsed().as_secs_f32());
    // println!("Playing Self from Endgame Positions");
    // for (game_idx, &fen) in FENS_ENDGAME.iter().enumerate() {
    //     let mut board = Board::from_fen(fen)
    //         .unwrap_or_else(|_| panic!("Invalid FEN in PGO set: {fen}"));
    //     self_play_single_game(game_idx, &mut board);
    // }
    // println!("Endgame Positions Took {} seconds", start.elapsed().as_secs_f32());
}

fn self_play_single_game(game_idx: usize, board: &mut Board) {

    for ply in 0..SELFPLAY_MAX_PLIES {
        if board.checkmate() || board.stalemate() {
            // You can add more draw detection here if you like,
            // but for PGO, this is already decent.
            break;
        }

        let trace = NoTrace::new();
        let mut searcher = MySearcher::new(trace, None);

        let best: BitMove = searcher.find_best_move(board, SELFPLAY_DEPTHS[ply % SELFPLAY_DEPTHS.len()]);

        if best == BitMove::null() {
            // No legal moves for some reason; bail out.
            break;
        }

        board.apply_move(best);

        // Optional: a little progress logging
        if ply == 0 {
            eprintln!("Self-play game #{game_idx} started");
        }
    }
}

// constants for square ids (A1 = 0, H1 = 7, A8 = 56, H8 = 63)
const SQ_A1: u8 = 0;
const SQ_H1: u8 = 7;
const SQ_A8: u8 = 56;
const SQ_H8: u8 = 63;
const SQUARE_NB: usize = 64;
const COLOR_NB: usize = 2;
const PIECE_NB: usize = 16;

// PieceSquareIndex[perspective][piece]
const PIECE_SQUARE_INDEX: [[u32; PIECE_NB]; COLOR_NB] = [
    // perspective = white
    [0, 0, 64 * 2, 64 * 4, 64 * 6, 64 * 8, 64 * 10, 0, 0, 64, 64 * 3, 64 * 5, 64 * 7, 64 * 9, 64 * 10, 0],
    // perspective = black
    [0, 64, 64 * 3, 64 * 5, 64 * 7, 64 * 9, 64 * 10, 0, 0, 0, 64 * 2, 64 * 4, 64 * 6, 64 * 8, 64 * 10, 0],
];

// KingBuckets[perspective][ksq]
const KING_BUCKETS: [[u32; SQUARE_NB]; COLOR_NB] = {
    // helper macro like B(v) = v * PS_NB = v * 64 * 11, but we inline values
    const PS_NB: u32 = 64 * 11;
    const fn b(v: u32) -> u32 { v * PS_NB }
    [
        [
            b(28), b(29), b(30), b(31), b(31), b(30), b(29), b(28),
            b(24), b(25), b(26), b(27), b(27), b(26), b(25), b(24),
            b(20), b(21), b(22), b(23), b(23), b(22), b(21), b(20),
            b(16), b(17), b(18), b(19), b(19), b(18), b(17), b(16),
            b(12), b(13), b(14), b(15), b(15), b(14), b(13), b(12),
            b(8),  b(9),  b(10), b(11), b(11), b(10), b(9),  b(8),
            b(4),  b(5),  b(6),  b(7),  b(7),  b(6),  b(5),  b(4),
            b(0),  b(1),  b(2),  b(3),  b(3),  b(2),  b(1),  b(0),
        ],
        [
            b(0),  b(1),  b(2),  b(3),  b(3),  b(2),  b(1),  b(0),
            b(4),  b(5),  b(6),  b(7),  b(7),  b(6),  b(5),  b(4),
            b(8),  b(9),  b(10), b(11), b(11), b(10), b(9),  b(8),
            b(12), b(13), b(14), b(15), b(15), b(14), b(13), b(12),
            b(16), b(17), b(18), b(19), b(19), b(18), b(17), b(16),
            b(20), b(21), b(22), b(23), b(23), b(22), b(21), b(20),
            b(24), b(25), b(26), b(27), b(27), b(26), b(25), b(24),
            b(28), b(29), b(30), b(31), b(31), b(30), b(29), b(28),
        ],
    ]
};

// const ORIENT_TBL: [[u8; SQUARE_NB]; COLOR_NB] = [
//     [SQ_H1; 32].map(|_| SQ_H1).iter().copied().chain([SQ_A1; 32]).collect::<Vec<u8>>().try_into().unwrap(), // simplified: we’ll inline the table below
//     [SQ_H8; 32].map(|_| SQ_H8).iter().copied().chain([SQ_A8; 32]).collect::<Vec<u8>>().try_into().unwrap(),
// ];
// Inline explicit table to avoid allocation:
const ORIENT_TBL_WHITE: [u8; SQUARE_NB] = [
    SQ_H1, SQ_H1, SQ_H1, SQ_H1, SQ_A1, SQ_A1, SQ_A1, SQ_A1,
    SQ_H1, SQ_H1, SQ_H1, SQ_H1, SQ_A1, SQ_A1, SQ_A1, SQ_A1,
    SQ_H1, SQ_H1, SQ_H1, SQ_H1, SQ_A1, SQ_A1, SQ_A1, SQ_A1,
    SQ_H1, SQ_H1, SQ_H1, SQ_H1, SQ_A1, SQ_A1, SQ_A1, SQ_A1,
    SQ_H1, SQ_H1, SQ_H1, SQ_H1, SQ_A1, SQ_A1, SQ_A1, SQ_A1,
    SQ_H1, SQ_H1, SQ_H1, SQ_H1, SQ_A1, SQ_A1, SQ_A1, SQ_A1,
    SQ_H1, SQ_H1, SQ_H1, SQ_H1, SQ_A1, SQ_A1, SQ_A1, SQ_A1,
    SQ_H1, SQ_H1, SQ_H1, SQ_H1, SQ_A1, SQ_A1, SQ_A1, SQ_A1,
];
const ORIENT_TBL_BLACK: [u8; SQUARE_NB] = [
    SQ_H8, SQ_H8, SQ_H8, SQ_H8, SQ_A8, SQ_A8, SQ_A8, SQ_A8,
    SQ_H8, SQ_H8, SQ_H8, SQ_H8, SQ_A8, SQ_A8, SQ_A8, SQ_A8,
    SQ_H8, SQ_H8, SQ_H8, SQ_H8, SQ_A8, SQ_A8, SQ_A8, SQ_A8,
    SQ_H8, SQ_H8, SQ_H8, SQ_H8, SQ_A8, SQ_A8, SQ_A8, SQ_A8,
    SQ_H8, SQ_H8, SQ_H8, SQ_H8, SQ_A8, SQ_A8, SQ_A8, SQ_A8,
    SQ_H8, SQ_H8, SQ_H8, SQ_H8, SQ_A8, SQ_A8, SQ_A8, SQ_A8,
    SQ_H8, SQ_H8, SQ_H8, SQ_H8, SQ_A8, SQ_A8, SQ_A8, SQ_A8,
    SQ_H8, SQ_H8, SQ_H8, SQ_H8, SQ_A8, SQ_A8, SQ_A8, SQ_A8,
];
// OrientTBL[perspective][ksq]
const ORIENT_TBL: [[u8; SQUARE_NB]; COLOR_NB] = [ORIENT_TBL_WHITE, ORIENT_TBL_BLACK];

#[inline]
pub fn make_index(perspective: usize, square: u8, piece: usize, king_sq: u8) -> usize {
    ((u32::from(square) ^ u32::from(ORIENT_TBL[perspective][king_sq as usize]))
        + PIECE_SQUARE_INDEX[perspective][piece]
        + KING_BUCKETS[perspective][king_sq as usize] ) as usize
}

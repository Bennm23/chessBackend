use std::array::from_fn;

use crate::constants::SQUARES;

pub const COLORS: usize = 2; // 0=White,1=Black
pub const PSQT_BUCKETS: usize = 8;

#[derive(Clone)]
pub struct Accumulator<const DIM: usize> {
    // accumulation[color][dim]
    pub accumulation: [[i16; DIM]; COLORS],
    // psqtAccumulation[color][bucket]
    pub psqt_accum: [[i32; PSQT_BUCKETS]; COLORS],
    pub computed: [bool; COLORS],
}

impl<const DIM: usize> Accumulator<DIM> {
    pub fn new() -> Self {
        Self {
            accumulation: [[0; DIM]; COLORS],
            psqt_accum: [[0; PSQT_BUCKETS]; COLORS],
            computed: [false; COLORS],
        }
    }
}

#[derive(Clone)]
pub struct AccumulatorState<const BIG: usize, const SMALL: usize> {
    pub big: Accumulator<BIG>,
    // store your DirtyPiece equivalent here
    pub dirty_piece: DirtyPiece,
}

/// DirtyPiece is the “what changed” record passed to the NNUE updater. It captures up to three piece changes from a move:
/// dirty_num: how many entries are valid.
/// piece[3]: which piece was involved (one per change).
/// from[3], to[3]: origin and destination squares for each change (may be SQ_NONE when a piece is created/removed).
/// Typical cases:
/// Normal move: 1 entry (moved piece from→to).
/// Capture: 2 entries (mover, captured piece to SQ_NONE).
/// Promotion with capture: up to 3 entries (pawn removed from, captured piece removed, promoted piece added). This lets the NNUE accumulator update incrementally without rebuilding.
#[derive(Clone, Default)]
pub struct DirtyPiece {
    // fill with your move deltas, from/to, piece types, etc.
    // e.g., pub from: [Option<Square>; 2], pub to: [Option<Square>; 2], pub piece: [Piece; 2]
}

impl<const BIG: usize, const SMALL: usize> AccumulatorState<BIG, SMALL> {
    pub fn reset(&mut self, dp: DirtyPiece) {
        self.dirty_piece = dp;
        self.big.computed = [false; COLORS];
        // self.small.computed = [false; COLORS];
    }
}

// Finny-table style cache keyed by king square and color
pub struct AccumulatorCache<const DIM: usize> {
    pub entries: [[CacheEntry<DIM>; COLORS]; 64],
}

pub struct CacheEntry<const DIM: usize> {
    pub accumulation: [i16; DIM],
    pub psqt_accum: [i32; PSQT_BUCKETS],
    pub by_color_bb: [u64; COLORS],      // bitboards if you need them
    pub by_type_bb: [u64; 6],            // adjust size to your piece types
}

impl<const DIM: usize> CacheEntry<DIM> {
    pub fn clear(&mut self, biases: &[i16; DIM]) {
        self.accumulation.copy_from_slice(biases);
        self.psqt_accum = [0; PSQT_BUCKETS];
        self.by_color_bb = [0; COLORS];
        self.by_type_bb = [0; 6];
    }
}

impl<const DIM: usize> AccumulatorCache<DIM> {
    pub fn new() -> Self {
        Self {
            entries: from_fn(|_| from_fn(|_| CacheEntry {
                accumulation: [0; DIM],
                psqt_accum: [0; PSQT_BUCKETS],
                by_color_bb: [0; COLORS],
                by_type_bb: [0; 6],
            })),
        }
    }

    pub fn clear_with_biases(&mut self, biases: &[i16; DIM]) {
        for sq in 0..SQUARES {
            for c in 0..COLORS {
                self.entries[sq][c].clear(biases);
            }
        }
    }
}

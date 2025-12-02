use core::panic;
use std::array::from_fn;

use pleco::{BitBoard, Board, Piece, Player};

use crate::{
    constants::{
        COLOR_OPS, COLORS, MAX_PLY, PAWN_THROUGH_KING, PIECE_TYPE_NB, PSQT_BUCKETS, SQUARES,
        TRANSFORMED_FEATURE_DIM_BIG,
    },
    feature_transformer::FeatureTransformer,
    feature_sets::{
        self, IndexList, MAX_ACTIVE_DIMENSIONS, append_changed_indices, requires_refresh,
    },
    nnue,
    nnue_misc::DirtyPiece,
};

/// NNUE per-color accumulator holding the fully materialized feature sums.
///
/// * `accumulation[color][dim]` stores the bias plus sum of active feature weights for each
///   neuron in the first fully connected layer (dimension `DIM`), one slice per side to move.
/// * `psqt_accum[color][bucket]` carries the piece-square table term for every PSQT bucket,
///   letting us combine PSQT and feature-transform outputs without recomputing them.
/// * `computed[color]` flags whether the cached data is up-to-date so incremental updates can
///   skip rebuilding untouched sides.
#[derive(Clone)]
#[repr(align(64))]
pub struct Accumulator<const DIM: usize> {
    pub accumulation: [[i16; DIM]; COLORS],
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

impl<const DIM: usize> Default for Accumulator<DIM> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Default)]
pub struct AccumulatorState {
    big: Accumulator<TRANSFORMED_FEATURE_DIM_BIG>,
    // small: Accumulator<TRANSFORMED_FEATURE_DIM_SMALL>,
    // store your DirtyPiece equivalent here
    pub dirty_piece: DirtyPiece,
}

impl AccumulatorState {
    pub fn reset(&mut self, dp: DirtyPiece) {
        self.dirty_piece = dp;
        self.big.computed = [false; COLORS];
        // self.small.computed = [false; COLORS];
    }

    pub fn get_accumulator_mut<const DIM: usize>(&mut self) -> &mut Accumulator<DIM> {
        if DIM == TRANSFORMED_FEATURE_DIM_BIG {
            unsafe {
                &mut *(&mut self.big as *mut Accumulator<TRANSFORMED_FEATURE_DIM_BIG>
                    as *mut Accumulator<DIM>)
            }
        } else {
            panic!("Unsupported dimension for accumulator retrieval");
        }
    }
    pub fn is_computed<const DIM: usize>(&self, perspective: Player) -> bool {
        if DIM == TRANSFORMED_FEATURE_DIM_BIG {
            self.big.computed[perspective as usize]
        } else {
            panic!("Unsupported dimension for accumulator retrieval");
        }
    }
    pub fn get_accumulator<const DIM: usize>(&self) -> &Accumulator<DIM> {
        if DIM == TRANSFORMED_FEATURE_DIM_BIG {
            unsafe {
                &*(&self.big as *const Accumulator<TRANSFORMED_FEATURE_DIM_BIG>
                    as *const Accumulator<DIM>)
            }
        } else {
            panic!("Unsupported dimension for accumulator retrieval");
        }
    }
}

fn update_accumulator_refresh_cache<const DIM: usize>(
    perspective: Player,
    ft: &FeatureTransformer<DIM>,
    board: &Board,
    accum: &mut Accumulator<DIM>,

    cache: &mut AccumulatorCache<DIM>,
) {
    let king_sq = board.king_sq(perspective);
    let cache_entry = &mut cache.entries[king_sq.to_index()][perspective as usize];

    let mut removed_features = IndexList::with_capacity(MAX_ACTIVE_DIMENSIONS);
    let mut added_features = IndexList::with_capacity(MAX_ACTIVE_DIMENSIONS);

    for c in [Player::White, Player::Black] {
        for pt in PAWN_THROUGH_KING {
            let piece = Piece::make_lossy(c, pt);
            let old_bb = cache_entry.by_color_bb[c as usize] & cache_entry.by_type_bb[pt as usize];
            let new_bb = board.piece_bb(c, pt);

            let mut to_remove = old_bb & !new_bb;
            let mut to_add = new_bb & !old_bb;

            while to_remove.is_not_empty() {
                let sq = to_remove.pop_lsb();
                removed_features.push(feature_sets::make_index(
                    perspective as usize,
                    sq.0,
                    piece as usize,
                    king_sq.0,
                ));
            }

            while to_add.is_not_empty() {
                let sq = to_add.pop_lsb();
                added_features.push(feature_sets::make_index(
                    perspective as usize,
                    sq.0,
                    piece as usize,
                    king_sq.0,
                ));
            }
        }
    }
    accum.computed[perspective as usize] = true;

    // TODO: Vector Ops ?

    for index in &removed_features {
        let offset = DIM * index;
        for j in 0..DIM {
            cache_entry.accumulation[j] -= ft.weights[offset + j];
        }
        for k in 0..PSQT_BUCKETS {
            cache_entry.psqt_accum[k] -= ft.psqt_weights[index * PSQT_BUCKETS + k];
        }
    }
    for index in &added_features {
        let offset = DIM * index;
        for j in 0..DIM {
            cache_entry.accumulation[j] += ft.weights[offset + j];
        }
        for k in 0..PSQT_BUCKETS {
            cache_entry.psqt_accum[k] += ft.psqt_weights[index * PSQT_BUCKETS + k];
        }
    }

    // Cache Entry Accum is updated, copy to Accumulator
    accum.accumulation[perspective as usize].copy_from_slice(&cache_entry.accumulation);
    accum.psqt_accum[perspective as usize].copy_from_slice(&cache_entry.psqt_accum);

    // End vector ops block

    // Store the bit boards for next pass and detecting change
    for c in COLOR_OPS {
        cache_entry.by_color_bb[c as usize] = board.get_occupied_player(c);
    }
    for pt in PAWN_THROUGH_KING {
        cache_entry.by_type_bb[pt as usize] = board.piece_bb_both_players(pt);
    }
}

#[derive(PartialEq, Eq)]
enum Direction {
    Forward,
    Backward,
}

fn update_accumulator_incremental<const DIM: usize>(
    perspective: Player,
    direction: Direction,
    ft: &FeatureTransformer<DIM>,
    ksq: pleco::SQ,
    accumulators: &mut [AccumulatorState],
    target_index: usize,
    current_index: usize,
) {
    // Split the references to avoid multiple borrows
    let (current_state, target_state) = if direction == Direction::Forward {
        assert!(target_index == current_index + 1);
        // [0, current_index] and [target_index, end]
        let split = accumulators.split_at_mut(target_index);
        (&split.0[current_index], &mut split.1[0])
    } else {
        assert!(target_index == current_index - 1);
        // [0, target_index] and [current_index, end]
        let split = accumulators.split_at_mut(current_index);
        (&split.1[0], &mut split.0[target_index])
    };

    let mut removed = IndexList::with_capacity(MAX_ACTIVE_DIMENSIONS);
    let mut added = IndexList::with_capacity(MAX_ACTIVE_DIMENSIONS);
    if direction == Direction::Forward {
        append_changed_indices(
            perspective,
            ksq.0,
            &target_state.dirty_piece,
            &mut removed,
            &mut added,
        );
    } else {
        append_changed_indices(
            perspective,
            ksq.0,
            &current_state.dirty_piece,
            &mut added,
            &mut removed,
        );
    }

    //TODO: I have no idea how to do this properly. handling the different dims makes no sense
    let current_accum = current_state.get_accumulator::<DIM>();
    let target_accum = target_state.get_accumulator_mut::<DIM>();

    assert!(current_accum.computed[perspective as usize]);
    assert!(!target_accum.computed[perspective as usize]);

    if removed.is_empty() && added.is_empty() {
        // No changes, just copy over
        target_accum.accumulation[perspective as usize]
            .copy_from_slice(&current_accum.accumulation[perspective as usize]);
        target_accum.psqt_accum[perspective as usize]
            .copy_from_slice(&current_accum.psqt_accum[perspective as usize]);
        target_accum.computed[perspective as usize] = true;
    } else {
        assert!(added.len() == 1 || added.len() == 2);
        assert!(removed.len() == 1 || removed.len() == 2);
        if direction == Direction::Forward {
            assert!(added.len() <= removed.len())
        } else {
            assert!(removed.len() <= added.len())
        }

        //TODO: Vector Ops ?
        // Start from current accumulator
        target_accum.accumulation[perspective as usize]
            .copy_from_slice(&current_accum.accumulation[perspective as usize]);
        target_accum.psqt_accum[perspective as usize]
            .copy_from_slice(&current_accum.psqt_accum[perspective as usize]);

        for index in removed {
            let offset = DIM * index;
            for i in 0..DIM {
                target_accum.accumulation[perspective as usize][i] -= 
                    ft.weights[offset + i];
            }
            for i in 0..PSQT_BUCKETS {
                target_accum.psqt_accum[perspective as usize][i] -=
                    ft.psqt_weights[index * PSQT_BUCKETS + i];
            }
        }

        for index in added {
            let offset = DIM * index;
            for i in 0..DIM {
                target_accum.accumulation[perspective as usize][i] +=
                    ft.weights[offset + i];
            }
            for i in 0..PSQT_BUCKETS {
                target_accum.psqt_accum[perspective as usize][i] +=
                    ft.psqt_weights[index * PSQT_BUCKETS + i];
            }
        }
    }

    target_accum.computed[perspective as usize] = true;
}

pub struct AccumulatorStack {
    //TODO: Should this be cache aligned? Should I use box?
    accumulators: Box<[AccumulatorState; 32]>, // MAX_PLY
    current_index: usize,
}

impl AccumulatorStack {
    pub fn new() -> Self {
        Self {
            accumulators: Box::new(from_fn(|_| AccumulatorState::default())),
            current_index: 0,
        }
    }

    pub fn reset(&mut self, board: &Board, nnue: &nnue::Nnue, caches: &mut AccumulatorCaches) {
        self.current_index = 1;

        update_accumulator_refresh_cache::<TRANSFORMED_FEATURE_DIM_BIG>(
            Player::White,
            &nnue.ft,
            board,
            self.accumulators[0].get_accumulator_mut::<TRANSFORMED_FEATURE_DIM_BIG>(),
            &mut caches.big,
        );
        update_accumulator_refresh_cache::<TRANSFORMED_FEATURE_DIM_BIG>(
            Player::Black,
            &nnue.ft,
            board,
            self.accumulators[0].get_accumulator_mut::<TRANSFORMED_FEATURE_DIM_BIG>(),
            &mut caches.big,
        );
        // Optionally, initialize the base accumulator from the board position here.
    }

    pub fn find_last_usable_accumulator(&self, perspective: Player) -> usize {
        for i in (1..self.current_index).rev() {
            //TODO: Check small, stockfish does this by passing around an accumulator pointer
            if self.accumulators[i].big.computed[perspective as usize] {
                return i;
            }

            if requires_refresh(&self.accumulators[i].dirty_piece, perspective) {
                return i;
            }
        }
        0 // Fallback to the base accumulator
    }
    pub fn evaluate<const DIM: usize>(
        &mut self,
        board: &Board,
        ft: &FeatureTransformer<DIM>,
        cache: &mut AccumulatorCache<DIM>,
    ) {
        self.evaluate_side(Player::White, board, ft, cache);
        self.evaluate_side(Player::Black, board, ft, cache);
    }
    pub fn evaluate_side<const DIM: usize>(
        &mut self,
        perspective: Player,
        board: &Board,
        ft: &FeatureTransformer<DIM>,
        cache: &mut AccumulatorCache<DIM>,
    ) {
        let last_usable_accum = self.find_last_usable_accumulator(perspective);

        if self.accumulators[last_usable_accum].big.computed[perspective as usize] {
            self.forward_update_incremental(perspective, board, ft, last_usable_accum);
        } else {
            update_accumulator_refresh_cache::<DIM>(
                perspective,
                ft,
                board,
                self.accumulators[last_usable_accum].get_accumulator_mut::<DIM>(),
                cache,
            );

            self.backward_update_incremental::<DIM>(perspective, board, ft, last_usable_accum);
        }
    }

    fn forward_update_incremental<const DIM: usize>(
        &mut self,
        perspective: Player,
        board: &Board,
        ft: &FeatureTransformer<DIM>,
        start_index: usize,
    ) {
        assert!(start_index < self.accumulators.len());
        assert!(
            self.accumulators[start_index]
                .get_accumulator::<DIM>()
                .computed[perspective as usize]
        );

        let ksq = board.king_sq(perspective);

        for next in (start_index + 1)..self.current_index {
            update_accumulator_incremental::<DIM>(
                perspective,
                Direction::Forward,
                ft,
                ksq,
                &mut self.accumulators.as_mut_slice(),
                next,
                next - 1,
            );
        }

        assert!(self.current().is_computed::<DIM>(perspective));
    }
    fn backward_update_incremental<const DIM: usize>(
        &mut self,
        perspective: Player,
        board: &Board,
        ft: &FeatureTransformer<DIM>,
        end_index: usize,
    ) {
        assert!(end_index < self.accumulators.len());
        assert!(end_index < self.current_index);
        assert!(self.current().is_computed::<DIM>(perspective));

        let ksq = board.king_sq(perspective);

        for next in (end_index..self.current_index - 1).rev() {
            update_accumulator_incremental::<DIM>(
                perspective,
                Direction::Backward,
                ft,
                ksq,
                &mut self.accumulators.as_mut_slice(),
                next,
                next + 1,
            );
        }

        assert!(self.current().is_computed::<DIM>(perspective));
    }

    pub fn push(&mut self, dirty_piece: DirtyPiece) {
        assert!(self.current_index < MAX_PLY);
        self.accumulators[self.current_index].reset(dirty_piece);
        self.current_index += 1;
    }

    pub fn pop(&mut self) {
        assert!(self.current_index > 1);
        self.current_index -= 1;
    }

    pub fn current(&self) -> &AccumulatorState {
        &self.accumulators[self.current_index - 1]
    }

    pub fn current_mut(&mut self) -> &mut AccumulatorState {
        &mut self.accumulators[self.current_index - 1]
    }
}

pub struct AccumulatorCaches {
    pub big: AccumulatorCache<TRANSFORMED_FEATURE_DIM_BIG>,
    // pub small: AccumulatorCache<TRANSFORMED_FEATURE_DIM_SMALL>,
}
impl AccumulatorCaches {
    pub fn new(biases: &Vec<i16>) -> Self {
        let mut big = AccumulatorCache::new();
        big.clear_with_biases(biases);
        Self {
            big,
            // small: AccumulatorCache::new(),
        }
    }
}
// Finny-table style cache keyed by king square and color
#[repr(align(64))]
pub struct AccumulatorCache<const DIM: usize> {
    pub entries: [[CacheEntry<DIM>; COLORS]; 64],
}

#[repr(align(64))]
pub struct CacheEntry<const DIM: usize> {
    pub accumulation: [i16; DIM],
    pub psqt_accum: [i32; PSQT_BUCKETS],
    /// Bitboards of pieces by color
    pub by_color_bb: [BitBoard; COLORS],
    /// Bitboards of pieces by type for both colors
    pub by_type_bb: [BitBoard; PIECE_TYPE_NB],
}

impl<const DIM: usize> CacheEntry<DIM> {
    pub fn clear(&mut self, biases: &Vec<i16>) {
        assert!(biases.len() == DIM);
        self.accumulation.copy_from_slice(biases);
        self.psqt_accum = [0; PSQT_BUCKETS];
        self.by_color_bb = [BitBoard(0); COLORS];
        self.by_type_bb = [BitBoard(0); PIECE_TYPE_NB];
    }
}

impl<const DIM: usize> AccumulatorCache<DIM> {
    pub fn new() -> Self {
        Self {
            entries: from_fn(|_| {
                from_fn(|_| CacheEntry {
                    accumulation: [0; DIM],
                    psqt_accum: [0; PSQT_BUCKETS],
                    by_color_bb: [BitBoard(0); COLORS],
                    by_type_bb: [BitBoard(0); PIECE_TYPE_NB],
                })
            }),
        }
    }

    pub fn clear_with_biases(&mut self, biases: &Vec<i16>) {
        assert!(biases.len() == DIM);
        for sq in 0..SQUARES {
            for c in 0..COLORS {
                self.entries[sq][c].clear(biases);
            }
        }
    }
}

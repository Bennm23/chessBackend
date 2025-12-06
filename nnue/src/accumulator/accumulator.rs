use core::panic;
use std::array::from_fn;

use aligned_vec::AVec;
use pleco::{BitBoard, Board, Piece, Player};

use crate::{
    constants::{
        COLOR_OPS, COLORS, MAX_PLY, PAWN_THROUGH_KING, PIECE_TYPE_NB, PSQT_BUCKETS, PsqtWeightType, SQUARES, TRANSFORMED_FEATURE_DIM_BIG, USE_AVX2, VectorAlignment, WeightType
    }, feature_sets::{
        self, IndexList, MAX_ACTIVE_DIMENSIONS, append_changed_indices, requires_refresh,
    }, feature_transformer::FeatureTransformer, nnue, nnue_misc::DirtyPiece, vectors::{NUM_PSQT_REGS, NUM_REGS_BIG, PSQT_TILE_HEIGHT, PsqtVecT, TILE_HEIGHT_BIG, VecT, 
        to_const_vec_ptr, to_mut_vec_ptr, vec_add_16, vec_add_32, vec_store_si256, vec_sub_16, vec_sub_32, vec_zero}
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

    if USE_AVX2 {

        let combine_last_3 = (removed_features.len() as i32 - added_features.len() as i32).abs() == 1 &&
            (removed_features.len() + added_features.len()) > 2;

        let num_regs = NUM_REGS_BIG;
        let tile_height = TILE_HEIGHT_BIG;

        // Just going to use big dim automatically, no idea how to make it dynamic for consts
        let mut acc: [VecT; NUM_REGS_BIG] = [vec_zero(); NUM_REGS_BIG]; 
        let mut psqt: [PsqtVecT; NUM_PSQT_REGS] = [vec_zero(); NUM_PSQT_REGS];

        for j in 0 .. DIM / tile_height {

            let acc_tile: *mut VecT = unsafe {
                accum
                    .accumulation[perspective as usize]
                    .as_mut_ptr()
                    .add(j * tile_height)
                    .cast()
            };

            // let entry_tile: *mut VecT = to_mut_vec_ptr(
            //     &mut cache_entry.accumulation, 
            //     j * tile_height
            // );
            let entry_tile: *mut VecT = unsafe {
                cache_entry
                    .accumulation
                    .as_mut_ptr()
                    .add(j * tile_height)
                    .cast()
            };

            // Store off accumulation
            for k in 0 .. num_regs {
                acc[k] = unsafe { *entry_tile.add(k) };
            }

            let mut i: usize = 0;
            while i < removed_features.len().min(added_features.len()) - combine_last_3 as usize {
                
                let index_r = removed_features[i];
                let offset_r = DIM * index_r + j * tile_height;
                let column_r: *const VecT = unsafe {
                    ft.weights
                        .as_ptr()
                        .add(offset_r)
                        .cast()
                };
                let index_a = added_features[i];
                let offset_a = DIM * index_a + j * tile_height;
                let column_a: *const VecT = unsafe {
                    ft.weights
                        .as_ptr()
                        .add(offset_a)
                        .cast()
                };

                for k in 0 .. num_regs {
                    acc[k] = vec_add_16(
                        acc[k],
                        unsafe {
                            vec_sub_16(
                                *column_a.add(k),
                                *column_r.add(k)
                            )
                        }
                    );
                }
                i += 1;
            }

            if combine_last_3 {
                let index_r = removed_features[i];
                let offset_r = DIM * index_r + j * tile_height;
                let column_r: *const VecT = to_const_vec_ptr(&ft.weights, offset_r);
                // let column_r: *const VecT = unsafe {
                //     ft.weights
                //         .as_ptr()
                //         .add(offset_r)
                //         .cast()
                // };
                let index_a = added_features[i];
                let offset_a = DIM * index_a + j * tile_height;
                let column_a: *const VecT = to_const_vec_ptr(&ft.weights, offset_a);
                // let column_a: *const VecT = unsafe {
                //     ft.weights
                //         .as_ptr()
                //         .add(offset_a)
                //         .cast()
                // };

                if removed_features.len() > added_features.len() {
                    let index_r2 = removed_features[i + 1];
                    let offset_r2 = DIM * index_r2 + j * tile_height;
                    let column_r2: *const VecT = to_const_vec_ptr(&ft.weights, offset_r2);
                    // let column_r2: *const VecT = unsafe {
                    //     ft.weights
                    //         .as_ptr()
                    //         .add(offset_r2)
                    //         .cast()
                    // };

                    for k in 0 .. num_regs {
                        acc[k] = vec_sub_16(
                            vec_add_16(acc[k], unsafe { *column_a.add(k) }), 
                            vec_add_16(unsafe { *column_r.add(k) }, unsafe { *column_r2.add(k) })
                        )
                    }
                } else {
                    let index_a2 = added_features[i + 1];
                    let offset_a2 = DIM * index_a2 + j * tile_height;
                    let column_a2: *const VecT = to_const_vec_ptr(&ft.weights, offset_a2);
                    // let column_a2: *const VecT = unsafe {
                    //     ft.weights
                    //         .as_ptr()
                    //         .add(offset_a2)
                    //         .cast()
                    // };

                    for k in 0 .. num_regs {
                        acc[k] = vec_add_16(
                            vec_sub_16(acc[k], unsafe { *column_r.add(k) }), 
                            vec_add_16(unsafe { *column_a.add(k) }, unsafe { *column_a2.add(k) })
                        )
                    }
                }
            } else {
                while i < removed_features.len() {
                    let index = removed_features[i];
                    let offset = DIM * index + j * tile_height;
                    let column: *const VecT = to_const_vec_ptr(&ft.weights, offset);

                    for k in 0 .. num_regs {
                        acc[k] = vec_sub_16(
                            acc[k],
                            unsafe { *column.add(k) }
                        );
                    }
                    i += 1;
                }
                while i < added_features.len() {
                    let index = added_features[i];
                    let offset = DIM * index + j * tile_height;
                    let column: *const VecT = to_const_vec_ptr(&ft.weights, offset);

                    for k in 0 .. num_regs {
                        acc[k] = vec_add_16(
                            acc[k],
                            unsafe { *column.add(k) }
                        );
                    }
                    i += 1;
                }
            }

            // Write out values
            for k in 0 .. num_regs {
                vec_store_si256(unsafe { entry_tile.add(k) }, acc[k]);
            }
            for k in 0 .. num_regs {
                vec_store_si256(unsafe { acc_tile.add(k) }, acc[k]);
            }

        }

        // PSQT Update
        for j in 0 .. PSQT_BUCKETS / PSQT_TILE_HEIGHT {
            let acc_tile_psqt: *mut VecT = to_mut_vec_ptr(
                &mut accum.psqt_accum[perspective as usize], 
                j * PSQT_TILE_HEIGHT
            );

            let entry_tile_psqt: *mut VecT = to_mut_vec_ptr(
                &mut cache_entry.psqt_accum, 
                j * PSQT_TILE_HEIGHT
            );

            for k in 0 .. NUM_PSQT_REGS {
                psqt[k] = unsafe { *entry_tile_psqt.add(k) };
            }

            for i in 0 .. removed_features.len() {
                let index = removed_features[i];
                let offset = PSQT_BUCKETS * index + j * PSQT_TILE_HEIGHT;
                let column_psqt: *const PsqtVecT = to_const_vec_ptr(&ft.psqt_weights, offset);

                for k in 0 .. NUM_PSQT_REGS {
                    psqt[k] = vec_sub_32(
                        psqt[k],
                        unsafe { *column_psqt.add(k) }
                    );
                }
            }

            for i in 0 .. added_features.len() {
                let index = added_features[i];
                let offset = PSQT_BUCKETS * index + j * PSQT_TILE_HEIGHT;
                let column_psqt: *const PsqtVecT = to_const_vec_ptr(&ft.psqt_weights, offset);

                for k in 0 .. NUM_PSQT_REGS {
                    psqt[k] = vec_add_32(
                        psqt[k],
                        unsafe { *column_psqt.add(k) }
                    );
                }
            }

            // Write out values
            for k in 0 .. NUM_PSQT_REGS {
                vec_store_si256(unsafe { entry_tile_psqt.add(k) }, psqt[k]);
            }
            for k in 0 .. NUM_PSQT_REGS {
                vec_store_si256(unsafe { acc_tile_psqt.add(k) }, psqt[k]);
            }
        }

    } else {

        // Normal
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
    }

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

        if USE_AVX2 {

            let acc_in: *const VecT = to_const_vec_ptr(
                &current_accum.accumulation[perspective as usize], 
                0
            );
            let acc_out: *mut VecT = to_mut_vec_ptr(
                &mut target_accum.accumulation[perspective as usize], 
                0
            );

            let offset_a0 = DIM * added[0];
            let column_a0: *const VecT = to_const_vec_ptr(&ft.weights, offset_a0);
            let offset_r0 = DIM * removed[0];
            let column_r0: *const VecT = to_const_vec_ptr(&ft.weights, offset_r0);

            if (direction == Direction::Forward && removed.len() == 1) ||
                (direction == Direction::Backward && added.len() == 1) {
                    assert!(added.len() == 1 && removed.len() == 1);
                    for i in 0 .. DIM * size_of::<WeightType>() / size_of::<VecT>() {

                        unsafe {
                            acc_out.add(i).write(
                                vec_add_16(
                                    vec_sub_16(*acc_in.add(i), *column_r0.add(i)), 
                                    *column_a0.add(i)
                                )
                            );
                        }

                    }
            } else if direction == Direction::Forward && added.len() == 1 {
                assert!(removed.len() == 2);
                let offset_r1 = DIM * removed[1];
                let column_r1: *const VecT = to_const_vec_ptr(&ft.weights, offset_r1);

                for i in 0 .. DIM * size_of::<WeightType>() / size_of::<VecT>() {

                    unsafe {
                        acc_out.add(i).write(
                            vec_sub_16(
                                vec_add_16(*acc_in.add(i), *column_a0.add(i)), 
                                vec_add_16(*column_r0.add(i), *column_r1.add(i))
                            )
                        );
                    }

                }
            } else if direction == Direction::Backward && removed.len() == 1 {
                assert!(added.len() == 2);
                let offset_a1 = DIM * added[1];
                let column_a1: *const VecT = to_const_vec_ptr(&ft.weights, offset_a1);

                for i in 0 .. DIM * size_of::<WeightType>() / size_of::<VecT>() {

                    unsafe {
                        acc_out.add(i).write(
                            vec_add_16(
                                vec_add_16(*acc_in.add(i), *column_a0.add(i)), 
                                vec_sub_16(*column_a1.add(i), *column_r0.add(i))
                            )
                        );
                    }

                }
            } else {
                assert!(added.len() == 2 && removed.len() == 2);
                let offset_a1 = DIM * added[1];
                let column_a1: *const VecT = to_const_vec_ptr(&ft.weights, offset_a1);
                let offset_r1 = DIM * removed[1];
                let column_r1: *const VecT = to_const_vec_ptr(&ft.weights, offset_r1);

                for i in 0 .. DIM * size_of::<WeightType>() / size_of::<VecT>() {

                    unsafe {
                        acc_out.add(i).write(
                            vec_add_16(
                                *acc_in.add(i), 
                                vec_sub_16(
                                    vec_add_16(*column_a0.add(i), *column_a1.add(i)), 
                                    vec_add_16(*column_r0.add(i), *column_r1.add(i))
                                )
                            )
                        );
                    }
                }
            }

            // PSQT Update
            let acc_psqt_in: *const PsqtVecT = to_const_vec_ptr(
                &current_accum.psqt_accum[perspective as usize], 
                0
            );
            let acc_psqt_out: *mut PsqtVecT = to_mut_vec_ptr(
                &mut target_accum.psqt_accum[perspective as usize], 
                0
            );

            let offset_psqt_a0 = PSQT_BUCKETS * added[0];
            let column_psqt_a0: *const PsqtVecT = to_const_vec_ptr(&ft.psqt_weights, offset_psqt_a0);
            let offset_psqt_r0 = PSQT_BUCKETS * removed[0];
            let column_psqt_r0: *const PsqtVecT = to_const_vec_ptr(&ft.psqt_weights, offset_psqt_r0);

            if (direction == Direction::Forward && removed.len() == 1) ||
                (direction == Direction::Backward && added.len() == 1) {
                
                for i in 0 .. PSQT_BUCKETS * size_of::<PsqtWeightType>() / size_of::<PsqtVecT>() {

                    unsafe {
                        acc_psqt_out.add(i).write(
                            vec_add_32(
                                vec_sub_32(*acc_psqt_in.add(i), *column_psqt_r0.add(i)), 
                                *column_psqt_a0.add(i)
                            )
                        );
                    }

                }
            } else if direction == Direction::Forward && added.len() == 1 {
                let offset_psqrt_r1 = PSQT_BUCKETS * removed[1];
                let column_psqt_r1: *const PsqtVecT = to_const_vec_ptr(&ft.psqt_weights, offset_psqrt_r1);
                for i in 0 .. PSQT_BUCKETS * size_of::<PsqtWeightType>() / size_of::<PsqtVecT>() {

                    unsafe {
                        acc_psqt_out.add(i).write(
                            vec_sub_32(
                                vec_add_32(*acc_psqt_in.add(i), *column_psqt_a0.add(i)), 
                                vec_add_32(*column_psqt_r0.add(i), *column_psqt_r1.add(i))
                            )
                        );
                    }

                }

            } else if direction == Direction::Backward && removed.len() == 1 {
                
                let offset_psqt_a1 = PSQT_BUCKETS * added[1];
                let column_psqt_a1: *const PsqtVecT = to_const_vec_ptr(&ft.psqt_weights, offset_psqt_a1);
                for i in 0 .. PSQT_BUCKETS * size_of::<PsqtWeightType>() / size_of::<PsqtVecT>() {

                    unsafe {
                        acc_psqt_out.add(i).write(
                            vec_add_32(
                                vec_add_32(*acc_psqt_in.add(i), *column_psqt_a0.add(i)), 
                                vec_sub_32(*column_psqt_a1.add(i), *column_psqt_r0.add(i))
                            )
                        );
                    }

                }
            } else {
                let offset_psqt_a1 = PSQT_BUCKETS * added[1];
                let column_psqt_a1: *const PsqtVecT = to_const_vec_ptr(&ft.psqt_weights, offset_psqt_a1);
                let offset_psqt_r1 = PSQT_BUCKETS * removed[1];
                let column_psqt_r1: *const PsqtVecT = to_const_vec_ptr(&ft.psqt_weights, offset_psqt_r1);
                for i in 0 .. PSQT_BUCKETS * size_of::<PsqtWeightType>() / size_of::<PsqtVecT>() {

                    unsafe {
                        acc_psqt_out.add(i).write(
                            vec_add_32(
                                *acc_psqt_in.add(i), 
                                vec_sub_32(
                                    vec_add_32(*column_psqt_a0.add(i), *column_psqt_a1.add(i)), 
                                    vec_add_32(*column_psqt_r0.add(i), *column_psqt_r1.add(i))
                                )
                            )
                        );
                    }

                }
            } 

        } else {
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
    pub fn new(biases: &AVec<i16, VectorAlignment>) -> Self {
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
    pub fn clear(&mut self, biases: &AVec<i16, VectorAlignment>) {
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

    pub fn clear_with_biases(&mut self, biases: &AVec<i16, VectorAlignment>) {
        assert!(biases.len() == DIM);
        for sq in 0..SQUARES {
            for c in 0..COLORS {
                self.entries[sq][c].clear(biases);
            }
        }
    }
}

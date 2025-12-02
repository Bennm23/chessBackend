use std::{
    fmt::Debug,
    io::{self, Read},
};

use pleco::Board;

use crate::{
    accumulator::{Accumulator, AccumulatorCache, AccumulatorStack},
    constants::*,
    nnue_utils::*,
    vectors::{
        MAX_CHUNK_SIZE, Vec_T, vec_max_16, vec_min_16, vec_mulhi_16, vec_packus_16, vec_set1_16,
        vec_slli_16, vec_zero,
    },
};

type OutputType = u8;

/// `FeatureTransformer` struct for transforming chess board features.
///
/// # Type Parameters
/// - `FEATURE_DIM`: The output feature dimension size.
///
/// # Safety
/// Some methods use unsafe code for pointer arithmetic and SIMD operations.
/// # Description
pub struct FeatureTransformer<const FEATURE_DIM: usize> {
    pub biases: Vec<i16>,       // FEATURE_DIMENSIONS
    pub weights: Vec<i16>,      // FEATURE_DIMENSIONS * INPUT_DIM
    pub psqt_weights: Vec<i32>, // PSQT_BUCKETS * INPUT_DIM
}

impl<const DIM: usize> Debug for FeatureTransformer<DIM> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("  Biases Len {}\n", self.biases.len()))?;
        f.write_str(&get_first_and_last(&self.biases))?;
        f.write_fmt(format_args!("  Weights Len {}\n", self.weights.len()))?;
        f.write_str(&get_first_and_last(&self.weights))?;
        f.write_fmt(format_args!(
            "  PSQT Weights Len {}\n",
            self.psqt_weights.len()
        ))?;
        f.write_str(&get_first_and_last(&self.psqt_weights))?;
        Ok(())
    }
}

// AVX2 pack order (PackusEpi16Order) and its inverse
const PACK_ORDER: [usize; 8] = [0, 2, 1, 3, 4, 6, 5, 7];
// const PACK_ORDER: [usize; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
const INV_PACK_ORDER: [usize; 8] = {
    let mut inv = [0usize; 8];
    let mut i = 0;
    while i < 8 {
        inv[PACK_ORDER[i]] = i;
        i += 1;
    }
    inv
};
impl<const FEATURE_DIM: usize> FeatureTransformer<FEATURE_DIM> {
    pub fn read_parameters(r: &mut impl Read) -> io::Result<Self> {
        // Feature transformer
        const FT_HASH_HEADER: u32 = 2133021880;
        let ft_hash = read_u32(r)?; // Hash header
        if ft_hash != FT_HASH_HEADER {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Feature Transformer hash header mismatch",
            ));
        }
        let biases = read_leb128_i16(r, FEATURE_DIM)?;
        let weights = read_leb128_i16(r, FEATURE_DIM * INPUT_DIM)?;
        let psqt_weights = read_leb128_i32(r, PSQT_BUCKETS * INPUT_DIM)?;
        let mut ft = FeatureTransformer {
            biases,
            weights,
            psqt_weights,
        };

        ft.permute_weights();
        ft.scale_weights(L1, INPUT_DIM, true);

        Ok(ft)
    }
    pub const fn input_dims(&self) -> usize {
        INPUT_DIM
    }
    pub const fn output_dims(&self) -> usize {
        FEATURE_DIM
    }
    pub const fn new_output_buffer(&self) -> CacheAligned<[OutputType; FEATURE_DIM]> {
        CacheAligned([0; FEATURE_DIM])
    }
    // Permute 16-byte blocks according to order (matches C++ PackusEpi16Order)
    fn permute_blocks(data: &mut [i16], order: &[usize]) {
        let block_elems = 16 / std::mem::size_of::<i16>(); // 8 i16 per block
        let chunk_len = order.len() * block_elems;
        assert!(data.len() % chunk_len == 0);
        for chunk in data.chunks_mut(chunk_len) {
            let mut tmp = vec![0i16; chunk_len];
            for (j, &src_block) in order.iter().enumerate() {
                let dst = j * block_elems;
                let src = src_block * block_elems;
                tmp[dst..dst + block_elems].copy_from_slice(&chunk[src..src + block_elems]);
            }
            chunk.copy_from_slice(&tmp);
        }
    }

    pub fn permute_weights(&mut self) {
        Self::permute_blocks(&mut self.biases, &PACK_ORDER);
        Self::permute_blocks(&mut self.weights, &PACK_ORDER);
    }

    pub fn unpermute_weights(&mut self) {
        Self::permute_blocks(&mut self.biases, &INV_PACK_ORDER);
        Self::permute_blocks(&mut self.weights, &INV_PACK_ORDER);
    }

    // Scale ×2 on load (read=true), ÷2 on save (read=false); half_dims is L1, input_dims is INPUT_DIM
    pub fn scale_weights(&mut self, half_dims: usize, input_dims: usize, read: bool) {
        for j in 0..input_dims {
            let row = &mut self.weights[j * half_dims..(j + 1) * half_dims];
            for w in row {
                if read {
                    *w *= 2;
                } else {
                    *w /= 2;
                }
            }
        }
        for b in &mut self.biases {
            if read {
                *b *= 2;
            } else {
                *b /= 2;
            }
        }
    }

    pub fn transform(
        &self,
        board: &Board,
        accumulator_stack: &mut AccumulatorStack,
        accumulator_cache: &mut AccumulatorCache<FEATURE_DIM>,
        output: *mut OutputType,
        bucket: usize,
    ) -> i32 {
        // 0 = WHITE, 1 = BLACK
        let perspectives = [board.turn(), !board.turn()];

        accumulator_stack.evaluate(board, self, accumulator_cache);

        let accum = accumulator_stack.current().get_accumulator::<FEATURE_DIM>();

        let psqt = (accum.psqt_accum[perspectives[0] as usize][bucket as usize]
            - accum.psqt_accum[perspectives[1] as usize][bucket as usize])
            / 2;

        // Layer computation

        // build aligned output buffer for FT
        let output_dim: usize = self.output_dims();

        for player in 0..COLORS {
            // Offset into buffer for this color
            // FT output is [White features | Black features], each is OUTPUT_DIM/2 entries
            let buff_offset = player * (self.output_dims() / 2);

            if cfg!(target_feature = "avx2") {
                const OUTPUT_CHUNK_SIZE: usize = MAX_CHUNK_SIZE;
                assert!((self.output_dims() / 2) % OUTPUT_CHUNK_SIZE == 0);
                let num_output_chunks = self.output_dims() / 2 / OUTPUT_CHUNK_SIZE;

                let zero: Vec_T = vec_zero();
                let one: Vec_T = vec_set1_16(127 * 2);

                let in0: *const Vec_T = accum.accumulation[perspectives[player] as usize]
                    .as_ptr()
                    .cast();
                let in1: *const Vec_T = unsafe {
                    accum.accumulation[perspectives[player] as usize]
                        .as_ptr()
                        .add(L1 / 2)
                        .cast()
                };
                let out_ptr: *mut Vec_T = unsafe { output.add(buff_offset).cast() };

                const SHIFT: i32 = 7; // predifined shift as long as SSSE2 is supported

                // Loop runs over NumOutputChunks blocks inside nnue/nnue_feature_transformer.h (line 382).
                // Each block represents MaxChunkSize transformed outputs (e.g. 32 or 64 values, depending on SIMD width).
                // For each chunk it loads two SIMD vectors from the first accumulator half (in0) and two from the second half (in1),
                //   clips them to [0, 254], left-shifts the first pair (preparing for later right shift), and then multiplies the pairs with vec_mulhi_16 so the product is effectively divided by 512.
                // The resulting two vectors (pa, pb) are packed via vec_packus_16 into a single byte vector and written to out[j], producing the final transformed features for that chunk.
                // Net effect: it computes output[offset + j] = clamp(sum0,0,254) * clamp(sum1,0,254) / 512 but does it MaxChunkSize elements at a time using SIMD,
                //   which is why it iterates over NumOutputChunks rather than every index individually.
                for j in 0..num_output_chunks {
                    let sum0a: Vec_T = vec_slli_16::<SHIFT>(vec_max_16(
                        vec_min_16(unsafe { *in0.add(j * 2 + 0) }, one),
                        zero,
                    ));
                    let sum0b: Vec_T = vec_slli_16::<SHIFT>(vec_max_16(
                        vec_min_16(unsafe { *in0.add(j * 2 + 1) }, one),
                        zero,
                    ));

                    let sum1a = vec_min_16(unsafe { *in1.add(j * 2 + 0) }, one);
                    let sum1b = vec_min_16(unsafe { *in1.add(j * 2 + 1) }, one);

                    let pa = vec_mulhi_16(sum0a, sum1a);
                    let pb = vec_mulhi_16(sum0b, sum1b);

                    unsafe {
                        out_ptr.add(j).write(vec_packus_16(pa, pb));
                    }
                }
            } else {
                for j in 0..self.output_dims() / 2 {
                    let mut sum0: i16 =
                        (accum.accumulation[perspectives[player] as usize][j] / 2) as i16;
                    let mut sum1 = (accum.accumulation[perspectives[player] as usize]
                        [j + output_dim / 2]
                        / 2) as i16;
                    sum0 = sum0.clamp(0, 254);
                    sum1 = sum1.clamp(0, 254);

                    unsafe {
                        *output.add(buff_offset + j) = ((sum0 as i32 * sum1 as i32) / 512) as u8;
                    }
                }
            }
        }

        psqt
    }
}

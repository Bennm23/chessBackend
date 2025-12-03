use std::{
    fmt::Debug,
    io::{self, Read},
};

use crate::nnue_utils::*;
use crate::{
    constants::{L1, L2, MAX_SIMD_WIDTH, USE_AVX2, USE_SSSE3},
    nnue_utils::read_i32_vec,
    vectors::{
        MAX_CHUNK_SIZE, VecT, Vec128T, mm_add_epi16, mm_load_si128, mm_set1_epi16,
        mm_setzero_si128, mm_storeu_si128, vec_add_dpbusd_epi32, vec_nnz, vec_set1_32, vec_zero,
    },
};

pub const INPUT_DIMENSIONS: usize = L1;
pub const PADDED_INPUT_DIMENSIONS: usize = ceil_to_multiple(INPUT_DIMENSIONS, MAX_SIMD_WIDTH);
pub const OUTPUT_DIMENSIONS: usize = L2 + 1;
pub const PADDED_OUTPUT_DIMENSIONS: usize = ceil_to_multiple(OUTPUT_DIMENSIONS, MAX_SIMD_WIDTH);

pub type InputType = u8;
pub type OutputType = i32;

type WeightType = i8;
type BiasType = OutputType;

#[cfg(target_feature = "avx2")]
pub const CHUNK_SIZE: usize = 4;

#[cfg(target_feature = "ssse3")]
#[repr(align(64))]
pub struct OffsetIndices {
    pub offset_indices: [[u16; 8]; 256],
}

#[cfg(target_feature = "ssse3")]
const DEBRUIJN64: u64 = 0x03F7_9D71_B4CB_0A89;
#[cfg(target_feature = "ssse3")]
const LSB_INDEX64: [i32; 64] = [
    0, 47, 1, 56, 48, 27, 2, 60, 57, 49, 41, 37, 28, 16, 3, 61, 54, 58, 35, 52, 50, 42, 21, 44, 38,
    32, 29, 23, 17, 11, 4, 62, 46, 55, 26, 59, 40, 36, 15, 53, 34, 51, 20, 43, 31, 22, 10, 45, 25,
    39, 14, 33, 19, 30, 9, 24, 13, 18, 8, 12, 7, 6, 5, 63,
];

#[cfg(target_feature = "ssse3")]
const fn constexpr_lsb(bb: u64) -> i32 {
    debug_assert!(bb != 0);
    let idx = ((bb ^ (bb - 1)).wrapping_mul(DEBRUIJN64)) >> 58;
    LSB_INDEX64[idx as usize]
}

#[cfg(target_feature = "ssse3")]
pub const fn build_offset_indices() -> OffsetIndices {
    let mut table = [[0u16; 8]; 256];
    let mut i = 0;
    while i < 256 {
        let mut j = i as u64;
        let mut k = 0u64;
        while j != 0 {
            table[i][k as usize] = constexpr_lsb(j) as u16;
            j &= j - 1;
            k += 1;
        }
        while k < 8 {
            table[i][k as usize] = 0;
            k += 1;
        }
        i += 1;
    }
    OffsetIndices {
        offset_indices: table,
    }
}

#[cfg(target_feature = "ssse3")]
pub static LOOKUP: OffsetIndices = build_offset_indices();

#[inline(always)]
fn get_weight_index(i: usize) -> usize {
    // if cfg!(target_feature = "avx2") {
    //     // compiled with avx2 enabled
    // }
    // if std::is_x86_feature_detected!("ssse3") {
    //     // CPU supports SSSE3 at runtime
    // }
    if USE_SSSE3 {
        (i / CHUNK_SIZE) % (PADDED_INPUT_DIMENSIONS / CHUNK_SIZE) * OUTPUT_DIMENSIONS * CHUNK_SIZE
            + i / PADDED_INPUT_DIMENSIONS * CHUNK_SIZE
            + i % CHUNK_SIZE
    } else {
        i
    }
}
/// Affine Transformation Sparse Input
/// Sparse affine layer (input-major weights):
// - biases: one i16 per output unit; they seed the accumulator.
// - weights: i8 matrix laid out as [input][padded_output]; for each active input feature,
//   its row is scaled by the feature value (u8) and added to the outputs.
// - padded_output is output_dims rounded up for SIMD-friendly stride (AVX2/SSSE3).
// This mirrors Stockfish’s AffineTransformSparseInput: biases + sparse weighted adds.

pub struct AffineTransformSparse {
    pub biases: CacheAligned<[BiasType; OUTPUT_DIMENSIONS]>, // len = output_dims
    pub weights: CacheAligned<[WeightType; OUTPUT_DIMENSIONS * PADDED_INPUT_DIMENSIONS]>, // len = input_dims * padded_output
}

impl Debug for AffineTransformSparse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "  Sparse Affine Biases Len {}\n",
            self.biases.len()
        ))?;
        // f.write_str(&get_first_and_last(&self.biases))?;
        f.write_fmt(format_args!(
            "  Sparse Affine Weights Len {}\n",
            self.weights.len()
        ))?;
        // f.write_str(&get_first_and_last(&self.weights))?;
        Ok(())
    }
}

fn find_nnz<const NNZ_IN_DIMS: usize>(
    input: *const i32,
    output: &mut [u16; NNZ_IN_DIMS],
    count_out: &mut u32,
) {
    const INPUT_SIMD_WIDTH: usize =
        std::mem::size_of::<VecT>() / std::mem::size_of::<OutputType>();
    const CHUNK_SIZE: usize = 8;
    let num_chunks: usize = NNZ_IN_DIMS / CHUNK_SIZE;
    const INPUTS_PER_CHUNK: usize = CHUNK_SIZE / INPUT_SIMD_WIDTH;
    const OUTPUTS_PER_CHUNK: usize = CHUNK_SIZE / 8;

    let input_vector: *const VecT = input as *const VecT;

    let mut count = 0;
    let mut base = mm_setzero_si128();
    let increment = mm_set1_epi16(8);

    for i in 0..num_chunks {
        // Bitmask of non-zero values in this chunk
        let mut nnz = 0u32;

        for j in 0..INPUTS_PER_CHUNK {
            let input_chunk = unsafe { *input_vector.add(i * INPUTS_PER_CHUNK + j) };

            nnz |= (vec_nnz(input_chunk) as u32) << (j * INPUT_SIMD_WIDTH);
        }

        for j in 0..OUTPUTS_PER_CHUNK {
            let lookup = (nnz >> (j * 8)) & 0xFF;
            let offsets =
                mm_load_si128(LOOKUP.offset_indices[lookup as usize].as_ptr() as *const Vec128T);
            mm_storeu_si128(
                output[count..].as_mut_ptr() as *mut Vec128T,
                mm_add_epi16(base, offsets),
            );

            count += lookup.count_ones() as usize;
            base = mm_add_epi16(base, increment);
        }
    }

    *count_out = count as u32;
}

impl AffineTransformSparse {
    pub fn new() -> Self {
        Self {
            biases: CacheAligned([0; OUTPUT_DIMENSIONS]),
            weights: CacheAligned([0; OUTPUT_DIMENSIONS * PADDED_INPUT_DIMENSIONS]),
        }
    }

    pub fn read_parameters(r: &mut impl Read) -> io::Result<Self> {
        let mut at = Self::new();

        let bias_vec = read_i32_vec(r, OUTPUT_DIMENSIONS)?;
        at.biases.0.copy_from_slice(&bias_vec);

        for i in 0..OUTPUT_DIMENSIONS * PADDED_INPUT_DIMENSIONS {
            at.weights[get_weight_index(i)] = read_i8(r)?;
        }

        Ok(at)
    }

    pub const fn new_output_buffer(&self) -> CacheAligned<[OutputType; PADDED_OUTPUT_DIMENSIONS]> {
        CacheAligned([0i32; PADDED_OUTPUT_DIMENSIONS])
    }

    pub fn propagate(
        &self,
        // input: & CacheAligned<[InputType; PADDED_INPUT_DIMENSIONS]>,
        // out: &mut CacheAligned<[OutputType; PADDED_OUTPUT_DIMENSIONS]>
        input: *const InputType,
        output: *mut OutputType,
    ) {
        if USE_AVX2 && USE_SSSE3 {
            const OUTPUT_SIMD_WIDTH: usize = MAX_CHUNK_SIZE / std::mem::size_of::<OutputType>();

            const NUM_CHUNKS: usize = ceil_to_multiple(INPUT_DIMENSIONS, 8) / CHUNK_SIZE;
            const NUM_REGS: usize = OUTPUT_DIMENSIONS / OUTPUT_SIMD_WIDTH;
            let mut nnz = [0u16; NUM_CHUNKS];

            let mut count = 0;

            let input32: *const i32 = input as *const i32;
            // Find indices of nonzero 32-bit blocks
            find_nnz(input32, &mut nnz, &mut count);

            let bias_vector: *const VecT = self.biases.as_ptr() as *const VecT;

            let mut acc = [vec_zero(); NUM_REGS];

            for k in 0..NUM_REGS {
                acc[k] = unsafe { *bias_vector.add(k) };
            }

            for j in 0..count as usize {
                let i = nnz[j];
                let in_vec = vec_set1_32(unsafe { *input32.add(i as usize) });
                let col: *const VecT = unsafe {
                    self.weights.as_ptr().add(i as usize * OUTPUT_DIMENSIONS * CHUNK_SIZE)
                        as *const VecT
                };

                for k in 0..NUM_REGS {
                    vec_add_dpbusd_epi32(&mut acc[k], in_vec, unsafe { *col.add(k) });
                }
            }

            let outptr: *mut VecT = output as *mut VecT;
            for k in 0..NUM_REGS {
                unsafe {
                    *outptr.add(k) = acc[k];
                }
            }
        } else {

        }
    }

}

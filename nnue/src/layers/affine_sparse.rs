use std::{fmt::Debug, io::{self, Read}};

use safe_arch::*;

use crate::{constants::{L1, L2, MAX_SIMD_WIDTH, USE_SSSE3}, nnue_utils::read_i32_vec};
use crate::nnue_utils::*;

pub const INPUT_DIMENSIONS: usize = L1;
pub const PADDED_INPUT_DIMENSIONS: usize = ceil_to_multiple(INPUT_DIMENSIONS, MAX_SIMD_WIDTH);
pub const OUTPUT_DIMENSIONS: usize = L2 + 1;
pub const PADDED_OUTPUT_DIMENSIONS: usize = ceil_to_multiple(OUTPUT_DIMENSIONS, MAX_SIMD_WIDTH);

pub type InputType = u8;
pub type OutputType = i32;

#[cfg(target_feature = "avx2")]
pub const CHUNK_SIZE: usize = 4;

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
             + i / PADDED_INPUT_DIMENSIONS * CHUNK_SIZE + i % CHUNK_SIZE
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
    pub biases: Vec<i32>,     // len = output_dims
    pub weights: Vec<i8>,     // len = input_dims * padded_output (input-major)
}

impl Debug for AffineTransformSparse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("  Sparse Affine Biases Len {}\n", self.biases.len()))?;
        // f.write_str(&get_first_and_last(&self.biases))?;
        f.write_fmt(format_args!("  Sparse Affine Weights Len {}\n", self.weights.len()))?;
        // f.write_str(&get_first_and_last(&self.weights))?;
        Ok(())
    }
}

impl AffineTransformSparse {
    pub fn new() -> Self {
        Self {
            biases: vec![0; OUTPUT_DIMENSIONS],
            weights: vec![0; OUTPUT_DIMENSIONS * PADDED_INPUT_DIMENSIONS],
        }
    }

    pub fn read_parameters(r: &mut impl Read) -> io::Result<Self> {


        let mut at = Self::new();

        at.biases = read_i32_vec(r, OUTPUT_DIMENSIONS)?;

        for i in 0 .. OUTPUT_DIMENSIONS * PADDED_INPUT_DIMENSIONS {
            at.weights[get_weight_index(i)] = read_i8(r)?;
        }


        Ok(at)
    }

    // active = list of (input index, input value)
    // pub fn forward(&self, active: &[(usize, u8)], out: &mut [i32]) {
    //     assert!(out.len() >= self.output_dims);
    //     // start from biases
    //     for (o, b) in out.iter_mut().zip(self.biases.iter()) {
    //         *o = *b as i32;
    //     }

    //     // Choose fast path
    //     if cfg!(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "avx2")) {
    //         self.forward_avx2(active, out);
    //     } else if cfg!(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "ssse3")) {
    //         self.forward_ssse3(active, out);
    //     } else {
    //         self.forward_scalar(active, out);
    //     }
    // }

    // fn forward_scalar(&self, active: &[(usize, u8)], out: &mut [i32]) {
    //     for &(idx, val) in active {
    //         let row = &self.weights[idx * self.padded_output .. (idx + 1) * self.padded_output];
    //         for o in 0..self.output_dims {
    //             out[o] += row[o] as i32 * val as i32;
    //         }
    //     }
    // }

    // fn forward_ssse3(&self, active: &[(usize, u8)], out: &mut [i32]) {
    //     let chunk = 16;
    //     for &(idx, val_u8) in active {
    //         let row = &self.weights[idx * self.padded_output .. (idx + 1) * self.padded_output];
    //         let v16 = set_splat_i16_m128i(val_u8 as i16);
    //         let mut o = 0;
    //         while o + chunk <= self.output_dims {
    //             // load 16 i8 weights -> widen to i16
    //             let w0 = unsafe { load_unaligned_m128i(row[o..].as_ptr()) };
    //             let w1 = unsafe { load_unaligned_m128i(row[o + 8..].as_ptr()) };
    //             let w0_16 = convert_to_i16_m128i_from_i8_m128i(w0);
    //             let w1_16 = convert_to_i16_m128i_from_i8_m128i(w1);
    //             // madd: (w0*v0 + w1*v1) into i32 lanes
    //             let p0 = madd_i16_horizontal_add_m128i(w0_16, v16);
    //             let p1 = madd_i16_horizontal_add_m128i(w1_16, v16);

    //             // load current out, add, store
    //             let acc0 = unsafe { load_unaligned_m128i(out[o..].as_ptr()) };
    //             let acc1 = unsafe { load_unaligned_m128i(out[o + 4..].as_ptr()) };
    //             let sum0 = add_i32_m128i(acc0, p0);
    //             let sum1 = add_i32_m128i(acc1, p1);
    //             unsafe {
    //                 store_unaligned_m128i(out[o..].as_mut_ptr(), sum0);
    //                 store_unaligned_m128i(out[o + 4..].as_mut_ptr(), sum1);
    //             }
    //             o += chunk;
    //         }
    //         // tail
    //         for j in o..self.output_dims {
    //             out[j] += row[j] as i32 * val_u8 as i32;
    //         }
    //     }
    // }

    // fn forward_avx2(&self, active: &[(usize, u8)], out: &mut [i32]) {
    //     let chunk = 32;
    //     for &(idx, val_u8) in active {
    //         let row = &self.weights[idx * self.padded_output .. (idx + 1) * self.padded_output];
    //         let v16 = set_splat_i16_m256i(val_u8 as i16);
    //         let mut o = 0;
    //         while o + chunk <= self.output_dims {
    //             // load 32 i8 weights (two 16-byte lanes)
    //             let w0 = unsafe { load_unaligned_m128i(row[o..].as_ptr()) };
    //             let w1 = unsafe { load_unaligned_m128i(row[o + 16..].as_ptr()) };
    //             let w0_16 = convert_to_i16_m256i_from_i8_m128i(w0);
    //             let w1_16 = convert_to_i16_m256i_from_i8_m128i(w1);

    //             // madd: widen to i32 lanes
    //             let p0 = madd_i16_horizontal_add_m256i(w0_16, v16);
    //             let p1 = madd_i16_horizontal_add_m256i(w1_16, v16);

    //             // load/add/store
    //             let acc0 = unsafe { load_unaligned_m256i(out[o..].as_ptr()) };
    //             let acc1 = unsafe { load_unaligned_m256i(out[o + 8..].as_ptr()) };
    //             let acc2 = unsafe { load_unaligned_m256i(out[o + 16..].as_ptr()) };
    //             let acc3 = unsafe { load_unaligned_m256i(out[o + 24..].as_ptr()) };

    //             let sum0 = add_i32_m256i(acc0, p0);
    //             let sum1 = add_i32_m256i(acc1, p1);

    //             // p0 holds 8 i32s, p1 holds 8 i32s; store into four chunks of 8
    //             unsafe {
    //                 store_unaligned_m256i(out[o..].as_mut_ptr(), sum0);
    //                 store_unaligned_m256i(out[o + 8..].as_mut_ptr(), sum1);
    //             }

    //             o += chunk;
    //         }
    //         // tail
    //         for j in o..self.output_dims {
    //             out[j] += row[j] as i32 * val_u8 as i32;
    //         }
    //     }
    // }
}

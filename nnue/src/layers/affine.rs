use std::{fmt::Debug, io::{self, Read}};

use aligned_vec::AVec;

use crate::{constants::{CACHE_ALIGN, USE_AVX2, USE_SSSE3, VectorAlignment}, nnue_utils::{ceil_to_multiple, read_i8, read_i32_vec}, vectors::{Vec_T, m256_hadd, vec_add_dpbusd_epi32, vec_set1_32, vec_zero}};

pub type InputType = u8;
pub type OutputType = i32;

type BiasType = i32;
type WeightType = i8;

pub struct AffineTransform {
    pub input_dims: usize,
    pub output_dims: usize,
    pub padded_input_dims: usize,
    pub padded_output_dims: usize,
    pub biases: AVec<BiasType, VectorAlignment>, // len = output_dims
    pub weights: AVec<WeightType, VectorAlignment>, // len = output_dims * padded_input; stored in chosen layout
}

impl Debug for AffineTransform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("  Affine Layer Biases Len {}\n", self.biases.len()))?;
        // f.write_str(&get_first_and_last(&self.biases))?;
        f.write_fmt(format_args!("  Affine Layer Weights Len {}\n", self.weights.len()))?;
        // f.write_str(&get_first_and_last(&self.weights))?;
        Ok(())
    }
}

#[inline(always)]
fn get_weight_index(i: usize, padded_input_dim : usize, output_dims: usize) -> usize {
    if USE_SSSE3 {
        (i / 4) % (padded_input_dim / 4) * output_dims * 4
            + i / padded_input_dim * 4 + i % 4
    } else {
        i
    }
}
impl AffineTransform {
    pub fn new(input_dims: usize, output_dims: usize) -> Self {
        let padded_input = ceil_to_multiple(input_dims, 32);
        let padded_output = ceil_to_multiple(output_dims, 32);
        Self {
            input_dims,
            output_dims,
            padded_input_dims: padded_input,
            padded_output_dims: padded_output,
            biases: AVec::with_capacity(CACHE_ALIGN, output_dims),
            weights: AVec::with_capacity(CACHE_ALIGN, output_dims * padded_input),
        }
    }
    /// `use_scramble` should be true if you want the SIMD-friendly layout
    /// (mirrors ENABLE_SEQ_OPT in Stockfish); false for plain row-major.
    pub fn read_parameters(
        r: &mut impl Read,
        input_dims: usize,
        output_dims: usize,
    ) -> io::Result<Self> {
    
        let mut at = Self::new(input_dims, output_dims);

        let bias_vec = read_i32_vec(r, output_dims)?;
        at.biases.extend_from_slice(&bias_vec);

        let mut weights = vec![WeightType::default(); output_dims * at.padded_input_dims];
        for i in 0 .. output_dims * at.padded_input_dims {
            weights[get_weight_index(i, at.padded_input_dims, output_dims)] = read_i8(r)?;
        }
        at.weights.extend_from_slice(&weights);

        Ok(at)
    }

    pub fn new_input_buffer(&self) -> AVec<InputType, VectorAlignment> {
        let mut a = AVec::with_capacity(64, self.padded_input_dims);
        a.extend_from_slice(&vec![InputType::default(); self.padded_input_dims]);
        a
    }
    pub fn new_output_buffer(&self) -> AVec<OutputType, VectorAlignment> {
        let mut a = AVec::with_capacity(64, self.padded_output_dims);
        a.extend_from_slice(&vec![OutputType::default(); self.padded_output_dims]);
        a
    }

    fn propagate_avx2(&self, input: *const InputType, output: *mut OutputType) {
        
        if self.output_dims > 1 {

            const OUTPUT_SIMD_WIDTH: usize = std::mem::size_of::<Vec_T>() / std::mem::size_of::<OutputType>();
            debug_assert!(self.output_dims % OUTPUT_SIMD_WIDTH == 0);

            let num_chunks = ceil_to_multiple(self.input_dims, 8) / 4;
            let num_regs = self.output_dims / OUTPUT_SIMD_WIDTH;

            let input32: *const i32 = input.cast();
            let bias_vector: *const Vec_T = self.biases.as_ptr().cast();
            let mut acc = vec![vec_zero(); num_regs];

            for k in 0 .. num_regs {
                acc[k] = unsafe { *bias_vector.add(k) };
            }
            for i in 0 .. num_chunks {
                let in0 = vec_set1_32( unsafe { *input32.add(i) });
                let col0: *const Vec_T = unsafe {
                    self.weights.as_ptr()
                        .add(i * self.output_dims * 4)
                        .cast()
                };

                for k in 0 .. num_regs {
                    vec_add_dpbusd_epi32(
                        &mut acc[k], 
                        in0, 
                        unsafe { *col0.add(k) }
                    );
                }
            }

            let outptr: *mut Vec_T = output.cast();
            for k in 0 .. num_regs {
                unsafe {*outptr.add(k) = acc[k]; }
            }

        } else if self.output_dims == 1 {
        
            let input_vector: *const Vec_T = input.cast();
            const INPUT_SIMD_WIDTH: usize = std::mem::size_of::<Vec_T>() / std::mem::size_of::<InputType>();
            debug_assert!(self.padded_input_dims % INPUT_SIMD_WIDTH == 0);
            let num_chunks = self.padded_input_dims / INPUT_SIMD_WIDTH;
            let mut sum0 = vec_zero();
            let row0: *const Vec_T = self.weights.as_ptr().cast();
            for j in 0 .. num_chunks {
                let in_vec = unsafe { *input_vector.add(j) };
                vec_add_dpbusd_epi32(
                    &mut sum0,
                    in_vec,
                    unsafe { *row0.add(j) }
                );

            }
            unsafe {
                output.add(0).write(
                    m256_hadd(sum0, self.biases[0])
                );
            }
        }
    }



    pub fn propagate(&self, input: *const InputType, output: *mut OutputType) {
        if USE_SSSE3 && USE_AVX2 {
            self.propagate_avx2(input, output);
        } else {
            // self.propagate_fallback(input, output);
        }
    }
}

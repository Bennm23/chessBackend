use std::{fmt::Debug, io::{self, Read}};

use crate::{constants::USE_SSSE3, nnue_utils::{ceil_to_multiple, get_first_and_last, read_i8, read_i32_vec}};

pub type InputType = u8;
pub type OutputType = i32;



pub struct AffineTransform {
    pub input_dims: usize,
    pub output_dims: usize,
    pub padded_input_dims: usize,
    pub padded_output_dims: usize,
    pub biases: Vec<i32>, // len = output_dims
    pub weights: Vec<i8>, // len = output_dims * padded_input; stored in chosen layout
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
            biases: vec![0; output_dims],
            weights: vec![0; output_dims * padded_input],
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

        at.biases = read_i32_vec(r, output_dims)?;

        for i in 0 .. output_dims * at.padded_input_dims {
            at.weights[get_weight_index(i, at.padded_input_dims, output_dims)] = read_i8(r)?;
        }

        Ok(at)
    }

}

use std::io::{self, Read};

use crate::{constants::{OUTPUT_SCALE, WEIGHT_SCALE_BITS}, nnue_utils::{CacheAligned, ceil_to_multiple, read_u32}};

mod affine_sparse;
mod affine;
mod sq_clipped_relu;
mod clipped_relu;

// Predefined SqClippedReLU for use in BucketNet to use L2 from constants
type SqClippedReLU = sq_clipped_relu::SqClippedReLU<
{crate::constants::L2 + 1}, {ceil_to_multiple(crate::constants::L2 + 1, 32)}
>;

type ClippedReLU_0 = clipped_relu::ClippedReLU<
{crate::constants::L2 + 1}, {ceil_to_multiple(crate::constants::L2 + 1, 32)}
>;
type ClippedReLU_1 = clipped_relu::ClippedReLU<
{crate::constants::L3}, {ceil_to_multiple(crate::constants::L3, 32)}
>;

/// One of the buckets, containing 3 Fully Connected layers
/// And their transformations
/// Weights are i8, biases are i32
///
/// # Architecture
/// ```
/// 8 buckets (by material count):
///   FC0 (sparse) L1 -> L2           // sparse matmul on active features; extra 16th used as forward term
///   SqrClippedReLU on first L2        // square and clamp hidden activations
///   ClippedReLU on same L2            // linear clamp of same activations
///   Concat L2_sq + L2_lin -> L2 * 2       // build mixed feature vector
///   FC1 (dense) L2 * 2 -> L3              // standard dense layer
///   ClippedReLU L3                    // clamp hidden layer
///   FC2 (dense) L3 -> 1               // final linear output
///  + scaled forward term from FC0[15] // add king-safety-style bonus to output
/// ```
pub struct BucketNet
<const L1: usize, const L2: usize, const L3: usize>
{
    fc0: affine_sparse::AffineTransformSparse, // 3072 -> 16 (sparse)
    ac_sqr_0: SqClippedReLU,
    ac_0: ClippedReLU_0,
    fc1: affine::AffineTransform, // 30 -> 32
    ac1: ClippedReLU_1,
    fc2: affine::AffineTransform, // 32 ->  1
}

impl<const L1: usize, const L2: usize, const L3: usize> std::fmt::Debug for BucketNet<L1, L2, L3> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("  FC0: Sparse Affine Layer\n")?;
        f.write_fmt(format_args!("{:?}", self.fc0))?;

        f.write_str("  FC1: Dense Affine Layer\n")?;
        f.write_fmt(format_args!("{:?}", self.fc1))?;
        f.write_str("  FC2: Dense Affine Layer\n")?;
        f.write_fmt(format_args!("{:?}", self.fc2))?;
        Ok(())
    }
}

impl<const L1: usize, const L2: usize, const L3: usize> BucketNet<L1, L2, L3> {
    const TRANSFORMED_FEATURE_DIMENSIONS: usize = L1;
    const FC_0_OUTPUT_DIMENSIONS: usize = L2; // +1 for forward term
    const FC_1_OUTPUT_DIMENSIONS: usize = L3;

    pub fn new(
        fc0: affine_sparse::AffineTransformSparse,
        fc1: affine::AffineTransform,
        fc2: affine::AffineTransform,
    ) -> Self {
        Self {
            fc0,
            ac_sqr_0: sq_clipped_relu::SqClippedReLU::new(),
            ac_0: clipped_relu::ClippedReLU::new(),
            fc1,
            ac1: clipped_relu::ClippedReLU::new(),
            fc2 
        }
    }

    pub fn read_parameters(r: &mut impl Read) -> io::Result<Self> {
        const LAYER_HASH_HEADER: u32 = 1664316490;

        let layer_hash = read_u32(r)?; // Hash header
        if layer_hash != LAYER_HASH_HEADER {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Layer hash header mismatch"));
        }
        let fc0 = affine_sparse::AffineTransformSparse::read_parameters(r)?;
        let fc1 = affine::AffineTransform::read_parameters(
            r, Self::FC_0_OUTPUT_DIMENSIONS * 2, Self::FC_1_OUTPUT_DIMENSIONS
        )?;
        let fc2 = affine::AffineTransform::read_parameters(
            r, Self::FC_1_OUTPUT_DIMENSIONS, 1
        )?;
        Ok(BucketNet::new(fc0, fc1, fc2))
    }

    pub fn propagate(&mut self, input: *const u8) -> i32 {
        //TODO: Cache Align all of them?
        let mut fc0_out = self.fc0.new_output_buffer();
        let mut ac_sqr_out = self.ac_sqr_0.new_output_buffer();
        let mut ac0_out = self.ac_0.new_output_buffer();

        self.fc0.propagate(input, fc0_out.as_mut_ptr());


        self.ac_sqr_0.propagate(fc0_out.as_ptr(), ac_sqr_out.as_mut_ptr());
        self.ac_0.propagate(fc0_out.as_ptr(), ac0_out.as_mut_ptr());

        let mut fc1_in = self.fc1.new_input_buffer();
        for i in 0 .. Self::FC_0_OUTPUT_DIMENSIONS {
            fc1_in[i] = ac_sqr_out.0[i];
            fc1_in[i + Self::FC_0_OUTPUT_DIMENSIONS] = ac0_out.0[i];
        }

        let mut fc1_out = self.fc1.new_output_buffer();
        self.fc1.propagate(fc1_in.as_ptr(), fc1_out.as_mut_ptr());
        let mut ac1_out = self.ac1.new_output_buffer();
        self.ac1.propagate(fc1_out.as_ptr(), ac1_out.as_mut_ptr());
        let mut fc2_out = self.fc2.new_output_buffer();
        self.fc2.propagate(ac1_out.as_ptr(), fc2_out.as_mut_ptr());

        let fwd_out = 
            (fc0_out[Self::FC_0_OUTPUT_DIMENSIONS]) * (600 * OUTPUT_SCALE) / (127 * (1 << WEIGHT_SCALE_BITS));

        let output_value = (fc2_out[0] as i32) + fwd_out;
        
        println!("BucketNet output_value: {}", output_value);

        output_value

    }
}


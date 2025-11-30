use std::io::{self, Read};

use crate::{constants::*, nnue_utils::read_u32};

mod affine_sparse;
mod affine;

pub const TRANSFORMED_FEATURE_DIMENSIONS: usize = L1;
pub const FC_0_OUTPUT_DIMENSIONS: usize = L2; // +1 for forward term
pub const FC_1_OUTPUT_DIMENSIONS: usize = L3;

/// One of the buckets, containing 3 Fully Connected layers
/// And their transformations
/// Weights are i8, biases are i32
///
/// # Architecture
/// ```
/// 8 buckets (by material count):
///   FC0 (sparse) 3072 -> 16           // sparse matmul on active features; extra 16th used as forward term
///   SqrClippedReLU on first 15        // square and clamp hidden activations
///   ClippedReLU on same 15            // linear clamp of same activations
///   Concat 15_sq + 15_lin -> 30       // build mixed feature vector
///   FC1 (dense) 30 -> 32              // standard dense layer
///   ClippedReLU 32                    // clamp hidden layer
///   FC2 (dense) 32 -> 1               // final linear output
///  + scaled forward term from FC0[15] // add king-safety-style bonus to output
/// ```
pub struct BucketNet {
    fc0: affine_sparse::AffineTransformSparse, // 3072 -> 16 (sparse)
    // SqrClippedReLU<FC_0_OUTPUT_DIMENSIONS + 1>
    // ClippedReLU<FC_0_OUTPUT_DIMENSIONS + 1>
    fc1: affine::AffineTransform, // 30 -> 32
    // ClippedReLU<FC_1_OUTPUT_DIMENSIONS>
    fc2: affine::AffineTransform, // 32 ->  1
    // Final output is i32
}

impl std::fmt::Debug for BucketNet {
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

impl BucketNet {
    // Further methods for BucketNet would go here

    pub fn read_parameters(r: &mut impl Read) -> io::Result<Self> {
        // let padded_l1 = ((L1 + 31) / 32) * 32;
        // let padded_30 = 32;
        const LAYER_HASH_HEADER: u32 = 1664316490;

        let layer_hash = read_u32(r)?; // Hash header
        if layer_hash != LAYER_HASH_HEADER {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Layer hash header mismatch"));
        }
        let fc0 = affine_sparse::AffineTransformSparse::read_parameters(r)?;
        let fc1 = affine::AffineTransform::read_parameters(
            r, FC_0_OUTPUT_DIMENSIONS * 2, FC_1_OUTPUT_DIMENSIONS
        )?;
        let fc2 = affine::AffineTransform::read_parameters(
            r, FC_1_OUTPUT_DIMENSIONS, 1
        )?;
        Ok(BucketNet { fc0, fc1, fc2 })
    }
}


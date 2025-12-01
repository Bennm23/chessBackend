use std::{fmt::Debug, io::{self, Read}};

use pleco::Board;

use crate::{constants::*, nnue_utils::*};

// Converts raw board features into NNUE inputs:
// - Maintains per-side accumulators: bias + weights for active features, updated incrementally.
// - Clamps accumulator halves (e.g., 2×1536 for big net) and pairwise multiplies them,
//   packing to an 8-bit transformed vector (width FEATURE_DIMENSIONS).
// - Computes a bucketed PSQT term from a separate PSQT weight table.
// - Returns (transformed_features, psqt) to feed the bucketed network layers.
pub struct FeatureTransformer<const FEATURE_DIM: usize> {
    pub biases: Vec<i16>,                  // FEATURE_DIMENSIONS
    pub weights: Vec<i16>,                 // FEATURE_DIMENSIONS * INPUT_DIM
    pub psqt_weights: Vec<i32>,            // PSQT_BUCKETS * INPUT_DIM
}

impl<const DIM: usize> Debug for FeatureTransformer<DIM> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("  Biases Len {}\n", self.biases.len()))?;
        f.write_str(&get_first_and_last(&self.biases))?;
        f.write_fmt(format_args!("  Weights Len {}\n", self.weights.len()))?;
        f.write_str(&get_first_and_last(&self.weights))?;
        f.write_fmt(format_args!("  PSQT Weights Len {}\n", self.psqt_weights.len()))?;
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
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Feature Transformer hash header mismatch"));
        }
        let biases = read_leb128_i16(r, FEATURE_DIM)?;
        let weights = read_leb128_i16(r, FEATURE_DIM * INPUT_DIM)?;
        let psqt_weights = read_leb128_i32(r, PSQT_BUCKETS * INPUT_DIM)?;
        let mut ft = FeatureTransformer { biases, weights, psqt_weights,};

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
    pub const fn new_output_buffer(&self) -> CacheAligned<[u8; FEATURE_DIM]> {
        CacheAligned([0u8; FEATURE_DIM])
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
            let row = &mut self.weights[j * half_dims .. (j + 1) * half_dims];
            for w in row {
                if read { *w *= 2; } else { *w /= 2; }
            }
        }
        for b in &mut self.biases {
            if read { *b *= 2; } else { *b /= 2; }
        }
    }

    pub fn transform(&self, board: &Board, bucket: i32) -> i32 {

        0
    }
}

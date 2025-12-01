use std::fmt::Debug;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;

use pleco::{Board, Piece, Player};

use crate::accumulator::{Accumulator, COLORS};
use crate::constants::*;
use crate::feature_transformer::FeatureTransformer;
use crate::half_ka_v2_hm::make_index;
use crate::layers::BucketNet;
use crate::nnue_misc::EvalTrace;
use crate::nnue_utils::*;
use crate::vectors::{
    MAX_CHUNK_SIZE, Vec_T, vec_max_16, vec_min_16, vec_mulhi_16, vec_packus_16, vec_set1_16,
    vec_slli_16, vec_zero,
};


pub struct NnueEvaluator {
    pub nnue: Nnue,
    accumulator: Accumulator<3072>,
}

impl NnueEvaluator {
    pub fn new(nnue: Nnue) -> Self {
        Self {
            nnue,
            accumulator: Accumulator::new(),
        }
    }
}

fn build_accum(
    board: &Board,
    biases: &[i16],
    weights: &[i16],
    psqt_weights: &[i32],
) -> Accumulator<L1> {
    let mut accum = Accumulator::<L1>::new();

    // accum.accumulation = [*biases.clone().try_into().unwrap(); COLORS];
    for color in 0..COLORS {
        for f in 0..L1 {
            accum.accumulation[color][f] = biases[f];
        }
    }

    let ksq = [board.king_sq(Player::White), board.king_sq(Player::Black)];

    for (sq, pc) in board.get_piece_locations() {
        if pc == Piece::None {
            continue;
        }
        for c in [Player::White, Player::Black] {
            let idx = make_index(c as usize, sq.0, pc as usize, ksq[c as usize].0);
            // add weights for this feature to accumulator
            // weights are input-major: weights[idx * L1 + feature]
            let row = &weights[idx * L1..(idx + 1) * L1];
            for f in 0..L1 {
                accum.accumulation[c as usize][f] =
                    accum.accumulation[c as usize][f].saturating_add(row[f]);
            }
            // psqt accumulation
            for b in 0..PSQT_BUCKETS {
                accum.psqt_accum[c as usize][b] += psqt_weights[idx * PSQT_BUCKETS + b];
            }
        }
    }

    accum
}

pub struct Nnue {
    desc: String,
    ft: FeatureTransformer<TRANSFORMED_FEATURE_DIM_BIG>,
    buckets: Vec<BucketNet<TRANSFORMED_FEATURE_DIM_BIG, L2, L3>>, // len = 8
}

impl Debug for Nnue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NNUE\n")?;
        f.write_fmt(format_args!("Desc {}\n", self.desc))?;
        f.write_fmt(format_args!("Feature Transformer\n{:?}\n", self.ft))?;
        for (i, bucket) in self.buckets.iter().enumerate() {
            f.write_fmt(format_args!("BucketNet {}\n{:?}\n", i, bucket))?;
        }
        Ok(())
    }
}

impl Nnue {
    pub fn evaluate(&mut self, board: &Board) -> i32 {
        let bucket: i32 = (board.count_all_pieces() as i32 - 1) / 4;
        println!("Bucket selected: {}", bucket);

        // TRANSFORM BLOCK

        // replace with accumulator stack
        let accum = build_accum(
            board,
            &self.ft.biases,
            &self.ft.weights,
            &self.ft.psqt_weights,
        );

        // 0 = WHITE, 1 = BLACK
        let perspectives = [board.turn(), !board.turn()];

        let psqt = (accum.psqt_accum[perspectives[0] as usize][bucket as usize]
            - accum.psqt_accum[perspectives[1] as usize][bucket as usize])
            / 2;
        println!("PSQT term: {}", psqt);

        // Layer computation

        // build aligned output buffer for FT
        let mut buf = self.ft.new_output_buffer();
        let output_dim: usize = self.ft.output_dims();

        for player in 0..COLORS {
            // Offset into buffer for this color
            // FT output is [White features | Black features], each is OUTPUT_DIM/2 entries
            let buff_offset = player * (self.ft.output_dims() / 2);

            if cfg!(target_feature = "avx2") {
                const OUTPUT_CHUNK_SIZE: usize = MAX_CHUNK_SIZE;
                assert!((self.ft.output_dims() / 2) % OUTPUT_CHUNK_SIZE == 0);
                let num_output_chunks = self.ft.output_dims() / 2 / OUTPUT_CHUNK_SIZE;

                let zero: Vec_T = vec_zero();
                let one: Vec_T = vec_set1_16(127 * 2);

                let in0: *const Vec_T = accum.accumulation[perspectives[player] as usize].as_ptr().cast();
                let in1: *const Vec_T = unsafe {
                    accum.accumulation[perspectives[player] as usize].as_ptr().add(L1 / 2).cast()
                };
                let out_ptr: *mut Vec_T = unsafe { buf.as_mut_ptr().add(buff_offset).cast() };

                const SHIFT: i32 = 6; // Stockfish has 7 for SSSE2, else 6

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
                for j in 0..self.ft.output_dims() / 2 {
                    // BiasType sum0 = accumulation[static_cast<int>(perspectives[p])][j + 0];
                    // BiasType sum1 =
                    // accumulation[static_cast<int>(perspectives[p])][j + HalfDimensions / 2];
                    // sum0               = std::clamp<BiasType>(sum0, 0, 127 * 2);
                    // sum1               = std::clamp<BiasType>(sum1, 0, 127 * 2);
                    // output[offset + j] = static_cast<OutputType>(unsigned(sum0 * sum1) / 512);

                    let mut sum0: i16 =
                        (accum.accumulation[perspectives[player] as usize][j] / 2) as i16;
                    let mut sum1 = (accum.accumulation[perspectives[player] as usize]
                        [j + output_dim / 2]
                        / 2) as i16;
                    sum0 = sum0.clamp(0, 254);
                    sum1 = sum1.clamp(0, 254);

                    buf[buff_offset + j] = ((sum0 as i32 * sum1 as i32) / 512) as u8;
                }
            }
        }

        0
    }

    pub fn trace_eval(&mut self, board: &Board) -> EvalTrace {
        let mut trace = EvalTrace::new();
        trace.selected_bucket = (board.count_all_pieces() as usize - 1) / 4;

        for bucket in 0..PSQT_BUCKETS {
            // TRANSFORM BLOCK

            println!("====================");
            println!("Bucket selected: {}", bucket);

            // replace with accumulator stack
            let accum = build_accum(
                board,
                &self.ft.biases,
                &self.ft.weights,
                &self.ft.psqt_weights,
            );

            // 0 = WHITE, 1 = BLACK
            let perspectives = [board.turn(), !board.turn()];

            let psqt = (accum.psqt_accum[perspectives[0] as usize][bucket as usize]
                - accum.psqt_accum[perspectives[1] as usize][bucket as usize])
                / 2;
            trace.psqt[bucket] = psqt / OUTPUT_SCALE;

            // Layer computation

            // build aligned output buffer for FT
            let mut buf = self.ft.new_output_buffer();
            let output_dim: usize = self.ft.output_dims();

            for player in 0..COLORS {
                // Offset into buffer for this color
                // FT output is [White features | Black features], each is OUTPUT_DIM/2 entries
                let buff_offset = player * (self.ft.output_dims() / 2);

                if cfg!(target_feature = "avx2") {
                    const OUTPUT_CHUNK_SIZE: usize = MAX_CHUNK_SIZE;
                    assert!((self.ft.output_dims() / 2) % OUTPUT_CHUNK_SIZE == 0);
                    let num_output_chunks = self.ft.output_dims() / 2 / OUTPUT_CHUNK_SIZE;

                    let zero: Vec_T = vec_zero();
                    let one: Vec_T = vec_set1_16(127 * 2);

                    let in0: *const Vec_T = accum.accumulation[perspectives[player] as usize].as_ptr().cast();
                    let in1: *const Vec_T = unsafe {
                        accum.accumulation[perspectives[player] as usize].as_ptr().add(L1 / 2).cast()
                    };
                    let out_ptr: *mut Vec_T = unsafe { buf.as_mut_ptr().add(buff_offset).cast() };

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
                    for j in 0..self.ft.output_dims() / 2 {

                        let mut sum0: i16 =
                            (accum.accumulation[perspectives[player] as usize][j] / 2) as i16;
                        let mut sum1 = (accum.accumulation[perspectives[player] as usize]
                            [j + output_dim / 2]
                            / 2) as i16;
                        sum0 = sum0.clamp(0, 254);
                        sum1 = sum1.clamp(0, 254);

                        buf[buff_offset + j] = ((sum0 as i32 * sum1 as i32) / 512) as u8;
                    }
                }
            }

            // We now have buf filled with transformed features for this bucket
            // need to run through net
            let position = self.buckets[bucket].propagate(buf.as_ptr());
            trace.positional[bucket] = position / OUTPUT_SCALE;
        }

        trace
    }
}

// Bucket selection: bucket = (piece_count - 1) / 4 (0..7).
// Feature transform:

// Maintain per-side accumulators (bias + sum of weights for active features); update incrementally per move.
// PSQT term = (psqtAccum[stm][bucket] - psqtAccum[opp][bucket]) / 2.
// For each side, take accumulator halves (L1/2 each), clamp to [0, 254], pairwise multiply, shift (>>9), pack to u8 → transformed features (width L1).
// Output: packed features + PSQT.
// Bucketed network (for that bucket):

// FC0 (sparse): 3072→16 (15 + 1 forward term), add biases.
// Activations: square-clipped ReLU on first 15; clipped ReLU on same 15; concatenate to 30.
// FC1 (dense): 30→32, add biases; clipped ReLU.
// FC2 (dense): 32→1, add biases.
// Add forward term: take FC0[15] scaled to match Stockfish’s 600 * OutputScale / (127 * 2^WeightScaleBits) and add to FC2 output.
// Scale output: divide by OutputScale (16) to get eval units; combine psqt+positional as Stockfish does ((125*psqt + 131*pos)/128, with small-net retry logic if you implement both).

// --- Loader ---
pub fn load_big_nnue(path: impl AsRef<Path>) -> io::Result<Nnue> {
    let f = File::open(path)?;
    let mut r = BufReader::new(f);

    let version = read_u32(&mut r)?;
    let hash = read_u32(&mut r)?;
    if version != VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "version mismatch",
        ));
    }
    println!("Here is the hash: {}", hash);
    if hash != BIG_HASH {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "arch hash mismatch",
        ));
    }
    let desc_len = read_u32(&mut r)? as usize;
    let mut desc_bytes = vec![0u8; desc_len];
    r.read_exact(&mut desc_bytes)?;
    let desc = String::from_utf8_lossy(&desc_bytes).to_string();

    // Feature transformer
    let ft = FeatureTransformer::read_parameters(&mut r)?;

    let mut buckets = Vec::with_capacity(LAYER_STACKS);
    for _ in 0..LAYER_STACKS {
        let net = BucketNet::read_parameters(&mut r)?;
        buckets.push(net);
    }

    // Sanity check EOF
    let mut tail = Vec::new();
    r.read_to_end(&mut tail)?;
    if !tail.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "trailing data"));
    }

    Ok(Nnue { desc, ft, buckets })
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    #[test]
    fn test_load_big_nnue() {
        let mut nnue =
            load_big_nnue("/home/bmellin/chess/chessBackendWebFinal/nn-1c0000000000.nnue").unwrap();
        println!("{:#?}", nnue);
        assert_eq!(nnue.ft.biases.len(), L1);
        assert_eq!(nnue.ft.weights.len(), L1 * INPUT_DIM);
        assert_eq!(nnue.ft.psqt_weights.len(), PSQT_BUCKETS * INPUT_DIM);
        assert_eq!(nnue.buckets.len(), LAYER_STACKS);
    }
    #[test]
    fn test_pos1() {
        let mut nnue =
            load_big_nnue("/home/bmellin/chess/chessBackendWebFinal/nn-1c0000000000.nnue").unwrap();

        // let mut board = Board::start_pos();
        let mut board =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR b KQkq - 0 1").unwrap();
        let start = Instant::now();
        let trace = nnue.trace_eval(&mut board);
// +-------------+-------------+-------------+-------------+
// |   Bucket    |  Material   | Positional  |    Total    |
// |             |   (PSQT)    |  (Layers)   |             |
// +-------------+-------------+-------------+-------------+
// |      0      |   + 0.07    |   + 2.77    |   + 2.83    |
// |      1      |   + 0.00    |   + 0.59    |   + 0.59    |
// |      2      |   + 0.02    |   + 0.13    |   + 0.14    |
// |      3      |   + 0.03    |   + 0.19    |   + 0.22    |
// |      4      |   + 0.02    |   + 0.03    |   + 0.06    |
// |      5      |   + 0.02    |   + 0.01    |   + 0.02    |
// |      6      |   + 0.01    |   - 0.04    |   - 0.03    |
// |      7      |   + 0.01    |   - 0.14    |   - 0.13    | <-- selected
// +-------------+-------------+-------------+-------------+

        let expected_psqt_vals = [25, 1, 6, 11, 9, 6, 3, 2];
        let expected_positional_vals = [1048, 223, 48, 73, 12, 3, -16, -52];

        for bucket in 0..LAYER_STACKS {
            assert_eq!(trace.psqt[bucket], expected_psqt_vals[bucket]);
            assert_eq!(trace.positional[bucket], expected_positional_vals[bucket]);
        }
        trace.print(&board);

        println!("Eval took {} µs", start.elapsed().as_micros());
    }
}

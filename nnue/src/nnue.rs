use std::fmt::{Debug, Display};
use std::fs::File;
use std::io::{self, Read, BufReader};
use std::path::Path;

use pleco::{Board, Piece, Player};

use crate::accumulator::{Accumulator, COLORS};
use crate::constants::{*};
use crate::feature_transformer::FeatureTransformer;
use crate::half_ka_v2_hm::make_index;
use crate::layers::BucketNet;
use crate::nnue_utils::{*};

// void AccumulatorStack::evaluate(const Position&                               pos,
//                                 const FeatureTransformer<Dimensions, accPtr>& featureTransformer,
//                                 AccumulatorCaches::Cache<Dimensions>&         cache) noexcept {

//     evaluate_side<WHITE>(pos, featureTransformer, cache);
//     evaluate_side<BLACK>(pos, featureTransformer, cache);
// }

// template<Color Perspective, IndexType Dimensions, Accumulator<Dimensions> AccumulatorState::*accPtr>
// void AccumulatorStack::evaluate_side(
//   const Position&                               pos,
//   const FeatureTransformer<Dimensions, accPtr>& featureTransformer,
//   AccumulatorCaches::Cache<Dimensions>&         cache) noexcept {

//     const auto last_usable_accum = find_last_usable_accumulator<Perspective, Dimensions, accPtr>();

//     if ((m_accumulators[last_usable_accum].*accPtr).computed[Perspective])
//         forward_update_incremental<Perspective>(pos, featureTransformer, last_usable_accum);

//     else
//     {
//         update_accumulator_refresh_cache<Perspective>(featureTransformer, pos, mut_latest(), cache);
//         backward_update_incremental<Perspective>(pos, featureTransformer, last_usable_accum);
//     }
// }

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

fn build_accum(board: &Board, biases: &[i16], weights: &[i16],
               psqt_weights: &[i32]) -> Accumulator<L1> {
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
            let idx = make_index(
                c as usize,
                sq.0,
                pc as usize,
                ksq[c as usize].0,
            );
            // add weights for this feature to accumulator
            // weights are input-major: weights[idx * L1 + feature]
            let row = &weights[idx * L1 .. (idx + 1) * L1];
            for f in 0..L1 {
                accum.accumulation[c as usize][f] = accum.accumulation[c as usize][f].saturating_add(row[f]);
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
    ft: FeatureTransformer,
    buckets: Vec<BucketNet>,           // len = 8
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

        let accum = build_accum(board, &self.ft.biases, &self.ft.weights, &self.ft.psqt_weights);

        // 0 = WHITE, 1 = BLACK
        let side_to_move = board.turn() as usize;

        let psqt = (accum.psqt_accum[side_to_move][bucket as usize] - accum.psqt_accum[1 - side_to_move][bucket as usize]) / 2;
        println!("PSQT term: {}", psqt);

        for b in 0..8 {
            let bucket_psqt = (accum.psqt_accum[side_to_move][b] - accum.psqt_accum[1 - side_to_move][b]) / 2;
            let scaled = bucket_psqt / 16;
            println!("Bucket {:15} | psqt: {:15} {}", b, format_cp_aligned_dot(scaled, board), if b == bucket as usize { "<-- selected" } else { "" });
        }

        0
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
        return Err(io::Error::new(io::ErrorKind::InvalidData, "version mismatch"));
    }
    println!("Here is the hash: {}", hash);
    if hash != BIG_HASH {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "arch hash mismatch"));
    }
    let desc_len = read_u32(&mut r)? as usize;
    let mut desc_bytes = vec![0u8; desc_len];
    r.read_exact(&mut desc_bytes)?;
    let desc = String::from_utf8_lossy(&desc_bytes).to_string();

    // Feature transformer
    let ft = FeatureTransformer::read_parameters(&mut r, TRANSFORMED_FEATURE_DIM_BIG)?;

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
    use super::*;

    #[test]
    fn test_load_big_nnue() {
        let mut nnue = load_big_nnue("/home/bmellin/chess/chessBackendWebFinal/nn-1c0000000000.nnue").unwrap();
        println!("{:#?}", nnue);
        assert_eq!(nnue.ft.biases.len(), L1);
        assert_eq!(nnue.ft.weights.len(), L1 * INPUT_DIM);
        assert_eq!(nnue.ft.psqt_weights.len(), PSQT_BUCKETS * INPUT_DIM);
        assert_eq!(nnue.buckets.len(), LAYER_STACKS);

        // let mut board = Board::start_pos();
        let mut board = Board::from_fen("rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR b KQkq - 0 1").unwrap();
        nnue.evaluate(&mut board);
    }

}

use std::fmt::Debug;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;
use std::sync::LazyLock;

use pleco::{Board, Piece, Player};

use crate::accumulator::{Accumulator, AccumulatorCache, AccumulatorCaches, AccumulatorStack};
use crate::constants::*;
use crate::feature_transformer::FeatureTransformer;
use crate::half_ka_v2_hm::make_index;
use crate::layers::BucketNet;
use crate::nnue_misc::{DirtyPiece, EvalTrace};
use crate::nnue_utils::*;

static NNUE_BIG : LazyLock<Nnue> = LazyLock::new(||
    load_big_nnue("/home/bmellin/chess/chessBackendWebFinal/nn-1c0000000000.nnue").expect("Failed to load NNUE")
);

pub struct NnueEvaluator {
    accum_stack: AccumulatorStack,
    accum_cache: AccumulatorCaches,
}

impl NnueEvaluator {
    pub fn new() -> Self {
        let accum_cache = AccumulatorCaches::new(&NNUE_BIG.ft.biases);
        Self {
            accum_stack: AccumulatorStack::new(),
            accum_cache,
        }
    }
    pub fn evaluate(&mut self, board: &Board) -> EvalResult {
        NNUE_BIG.evaluate(board, &mut self.accum_stack, &mut self.accum_cache.big)
    }
    pub fn trace_eval(&mut self, board: &Board) -> EvalTrace {
        NNUE_BIG.trace_eval(board, &mut self.accum_stack, &mut self.accum_cache.big)
    }
    pub fn reset(&mut self, board: &Board) {
        self.accum_stack.reset(board, &NNUE_BIG, &mut self.accum_cache);
    }

    pub fn do_move(&mut self, board: &Board, mv: pleco::BitMove) {
        let dirty_piece = DirtyPiece::from_move(board, mv);
        self.accum_stack.push(dirty_piece);
    }
    pub fn undo_move(&mut self) {
        self.accum_stack.pop();
    }
}

fn build_accum(
    board: &Board,
    biases: &[i16],
    weights: &[i16],
    psqt_weights: &[i32],
) -> Accumulator<L1> {
    let mut accum = Accumulator::<L1>::new();

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
    pub ft: FeatureTransformer<TRANSFORMED_FEATURE_DIM_BIG>,
    buckets: Vec<BucketNet<TRANSFORMED_FEATURE_DIM_BIG, L2, L3>>,
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

pub struct EvalResult {
    pub psqt: i32,
    pub positional: i32,
}

impl EvalResult {
    pub fn raw_fmt(&self) -> String {
        format!("PSQT: {}, Positional: {}, Scaled Total: {}", self.psqt, self.positional, self.scaled_total())
    }
    pub fn cp_fmt(&self, board: &Board) -> String {
        format!(
            "CP -> PSQT: {}, Positional: {}, Scaled Total: {}",
            format_cp_aligned_dot(self.psqt, board),
            format_cp_aligned_dot(self.positional, board),
            format_cp_aligned_dot(self.scaled_total(), board)
        )
    }

    pub fn scaled_total(&self) -> i32 {
        (125 * self.psqt + 131 * self.positional) / 128
    }

}

impl Nnue {
    pub fn evaluate(
        &self,
        board: &Board,
        accum_stack: &mut AccumulatorStack,
        accum_cache: &mut AccumulatorCache<TRANSFORMED_FEATURE_DIM_BIG>
    ) -> EvalResult {
        let bucket: usize = (board.count_all_pieces() as usize - 1) / 4;

        // TRANSFORM BLOCK

        let mut buf = self.ft.new_output_buffer();
        let psqt = self.ft.transform_full(
            board,
            accum_stack,
            accum_cache,
            buf.as_mut_ptr(),
            bucket
        );

        // We now have buf filled with transformed features for this bucket
        // need to run through net
        let positional = self.buckets[bucket].propagate(buf.as_ptr());

        EvalResult { psqt: psqt / OUTPUT_SCALE, positional: positional / OUTPUT_SCALE }
    }

    pub fn trace_eval(
        &self,
        board: &Board,
        accum_stack: &mut AccumulatorStack,
        accum_cache: &mut AccumulatorCache<TRANSFORMED_FEATURE_DIM_BIG>
    ) -> EvalTrace {
        let mut trace = EvalTrace::new();
        trace.selected_bucket = (board.count_all_pieces() as usize - 1) / 4;
        trace.side_to_move = board.turn();

        for bucket in 0..PSQT_BUCKETS {
            // TRANSFORM BLOCK

            let mut buf = self.ft.new_output_buffer();
            let psqt = self.ft.transform_full(
                board,
                accum_stack,
                accum_cache,
                buf.as_mut_ptr(),
                bucket
            );

            trace.psqt[bucket] = psqt / OUTPUT_SCALE;

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
        let nnue =
            load_big_nnue("/home/bmellin/chess/chessBackendWebFinal/nn-1c0000000000.nnue").unwrap();
        println!("{:#?}", nnue);
        assert_eq!(nnue.ft.biases.len(), L1);
        assert_eq!(nnue.ft.weights.len(), L1 * INPUT_DIM);
        assert_eq!(nnue.ft.psqt_weights.len(), PSQT_BUCKETS * INPUT_DIM);
        assert_eq!(nnue.buckets.len(), LAYER_STACKS);
    } 

    #[test]
    fn test_start_pos() {
        let mut evaluator = NnueEvaluator::new();

        let board = Board::start_pos();
        evaluator.reset(&board);
        let eval = evaluator.evaluate(&board);

        println!("Turn to move: {:?}", board.turn());
        println!("Eval: {}", eval.raw_fmt());

        assert!(eval.psqt == 0);
        assert!(eval.positional == 20);
        assert!(eval.scaled_total() == 20);
    }
    #[test]
    fn test_pos1() {
        let mut evaluator = NnueEvaluator::new();
        let board =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR b KQkq - 0 1").unwrap();
        let start = Instant::now();
        evaluator.reset(&board);
        let eval = evaluator.evaluate(&board);

        println!("Turn to move: {:?}", board.turn());
        println!("Eval: {}", eval.raw_fmt());

        assert!(eval.psqt == 2);
        assert!(eval.positional == -52);
        assert!(eval.scaled_total() == -51);

        println!("Eval took {} µs", start.elapsed().as_micros());
    }
    #[test]
    fn test_white_and_black_favored() {
        let mut evaluator = NnueEvaluator::new();

        let board =
            Board::from_fen("rq2kb1r/pppb1ppp/3ppn2/8/4PP2/2P5/PP1P2PP/RNB1KBNR w KQkq - 0 1").unwrap();
        evaluator.reset(&board);
        let eval = evaluator.evaluate(&board);

        assert!(eval.psqt == -2050);
        assert!(eval.positional == 121);
        assert!(eval.scaled_total() == -1878);
    }
    #[test]
    fn test_black_and_black_favored() {
        let mut evaluator = NnueEvaluator::new();

        let board =
            Board::from_fen("rq2kb1r/pppb1ppp/3ppn2/8/4PP2/2P5/PP1P2PP/RNB1KBNR b KQkq - 0 1").unwrap();
        let start = Instant::now();
        evaluator.reset(&board);
        let eval = evaluator.evaluate(&board);

        println!("Turn to move: {:?}", board.turn());
        println!("Eval: {}", eval.raw_fmt());
        println!("CP Eval: {}", eval.cp_fmt(&board));

        assert!(eval.psqt == 2050);
        assert!(eval.positional == 580);
        assert!(eval.scaled_total() == 2595);

        println!("Eval took {} µs", start.elapsed().as_micros());
    }
    #[test]
    fn test_evaluate() {
        let mut evaluator = NnueEvaluator::new();

        let mut board =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR b KQkq - 0 1").unwrap();

        evaluator.reset(&board);//Reset at start of search

        let mut trace = evaluator.trace_eval(&board);

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

        trace.print(&board);

        let mv = *board.generate_moves().get(0).unwrap();
        let start = Instant::now();
        // println!("Applying move: {}", mv);
        evaluator.do_move(&board, mv);
        board.apply_move(mv);

        trace = evaluator.trace_eval(&board);
        trace.print(&board);

        evaluator.undo_move();
        board.undo_move();

        trace = evaluator.trace_eval(&board);
        trace.print(&board);

        //30-50 us move -> undo
        //20 us just eval
        println!("Full Trace took {} µs", start.elapsed().as_micros());
    }

    #[test]
    fn instantiate_test() {
        let start = Instant::now();
        let _evaluator = NnueEvaluator::new();
        println!("Instantiation took {} µs", start.elapsed().as_micros());
    
        println!("NNUE SIZE = {} MB", std::mem::size_of::<NnueEvaluator>() as f64 / 1_048_576f64 );
    }
}

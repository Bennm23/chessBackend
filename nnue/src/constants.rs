use aligned_vec::ConstAlign;
use pleco::{PieceType, Player};

/// This is the NNUE file that is currently supported. All hashes/versions etc are based
/// upon this file used in stockfish 17.1
pub const NNUE_FILE: &str = "nn-1c0000000000.nnue";

pub const COLORS: usize = 2; // 0=White,1=Black
pub const COLOR_OPS: [Player; COLORS] = [Player::White, Player::Black];
pub const PIECE_TYPE_NB: usize = 8; // P,N,B,R,Q,K + empty + all
pub const PAWN_THROUGH_KING: [PieceType; 6] = [
    PieceType::P,
    PieceType::N,
    PieceType::B,
    PieceType::R,
    PieceType::Q,
    PieceType::K,
];

pub const VERSION: u32 = 0x7AF32F20;
pub const BIG_HASH: u32 = 470819058;
// pub const BIG_HASH: u32 = 0xEC42E90D ^ (3072 * 2) ^ 0xCC03DAE4; // adjust if mismatched on future nets

// pub const BIG_HASH: u32 = 0xEC42E90D ^ (3072 * 2) // base hash in arch
//     ^ 0xCC03DAE4 ^ 16 ^ (0xCC03DAE4 >> 1) ^ (0xCC03DAE4 << 31) // fc0
//     ^ 0x46F11061 ^ (0x46F11061 >> 1) ^ (0x46F11061 << 31)     // ac0
//     ^ 0xCC03DAE4 ^ 32 ^ (0xCC03DAE4 >> 1) ^ (0xCC03DAE4 << 31) // fc1
//     ^ 0x46F11061 ^ (0x46F11061 >> 1) ^ (0x46F11061 << 31)     // ac1
//     ^ 0xCC03DAE4 ^ 1 ^ (0xCC03DAE4 >> 1) ^ (0xCC03DAE4 << 31); // fc2
pub const LAYER_STACKS: usize = 8;
pub const PSQT_BUCKETS: usize = 8;

pub const MAX_PLY: usize = 64;

// The feature set, halfka_v2_hm, uses 22,528 input features.
pub const INPUT_DIM: usize = 22_528;
pub const TRANSFORMED_FEATURE_DIM_BIG: usize = 3072;
pub const TRANSFORMED_FEATURE_DIM_SMALL: usize = 128;
pub const L1: usize = 3_072;
pub const L1_SMALL: usize = 128;
pub const L2: usize = 15;
pub const L2_PLUS_1: usize = L2 + 1;
pub const L3: usize = 32;


pub const MAX_SIMD_WIDTH: usize = 32; // AVX2

pub const USE_AVX2: bool = cfg!(target_feature = "avx2");
pub const USE_SSSE3: bool = cfg!(target_feature = "ssse3");

pub const SQUARES: usize = 64;

pub const OUTPUT_SCALE: i32 = 16; // Final evaluation division factor
pub const WEIGHT_SCALE_BITS: usize = 6;

pub const CACHE_ALIGN: usize = 64;
pub type VectorAlignment = ConstAlign<CACHE_ALIGN>;
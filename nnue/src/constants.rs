/// This is the NNUE file that is currently supported. All hashes/versions etc are based
/// upon this file used in stockfish 17.1
pub const NNUE_FILE: &str = "nn-1c0000000000.nnue";

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
pub const INPUT_DIM: usize = 22_528;
pub const TRANSFORMED_FEATURE_DIM_BIG: usize = 3072;
pub const TRANSFORMED_FEATURE_DIM_SMALL: usize = 128;
pub const L1: usize = 3_072;
pub const L2: usize = 15;
pub const L3: usize = 32;
pub const PSQT_BUCKETS: usize = 8;


pub const MAX_SIMD_WIDTH: usize = 32; // AVX2

pub const USE_AVX2: bool = cfg!(target_feature = "avx2");
pub const USE_SSSE3: bool = cfg!(target_feature = "ssse3");

pub const SQUARES: usize = 64;
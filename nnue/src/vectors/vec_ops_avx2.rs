use std::arch::x86_64::{__m256i, _mm_add_epi32, _mm_cvtsi128_si32, _mm_shuffle_epi32, _mm256_castsi256_si128, _mm256_extracti128_si256};

use crate::constants::{PSQT_BUCKETS, PsqtWeightType, TRANSFORMED_FEATURE_DIM_BIG, TRANSFORMED_FEATURE_DIM_SMALL, WeightType};

/// Packed 256-bit lane used for the main NNUE accumulator math (identical to Stockfish's `vec_t`).
pub type VecT = __m256i;
/// Packed 256-bit lane used for PSQT-specific SIMD operations. Currently the same as [`VecT`],
/// but kept separate in case we want to specialize PSQT arithmetic later.
pub type PsqtVecT = __m256i;

/// Number of bytes processed per AVX2 chunk. Each register holds 16 `i16` lanes (32 bytes).
pub const MAX_CHUNK_SIZE: usize = 32; // AVX2
pub const SIMD_WIDTH: usize = MAX_CHUNK_SIZE;

pub const NUM_REGISTERS_SIMD: usize = 16;


const fn best_register_count() -> usize {

    const REGISTER_SIZE: usize = std::mem::size_of::<VecT>();
    // const LANE_SIZE: usize = std::mem::size_of::<WeightType>(); // For Normal Regs
    const LANE_SIZE: usize = std::mem::size_of::<PsqtWeightType>(); // For PSQT Regs


    const NUM_LANES: usize = PSQT_BUCKETS; // PSQT
    // const NUM_LANES: usize = TRANSFORMED_FEATURE_DIM_BIG; // Big net
    // const NUM_LANES: usize = TRANSFORMED_FEATURE_DIM_BIG; //Small for small net

    const IDEAL: usize = (NUM_LANES * LANE_SIZE) / REGISTER_SIZE;

    if IDEAL <= NUM_REGISTERS_SIMD {
        return IDEAL;
    }

    let mut divisor = NUM_REGISTERS_SIMD;
    while divisor > 1 {
        if IDEAL % divisor == 0 {
            return divisor;
        }
        divisor -= 1;
    }

    1
}

pub enum RegsAndTileHeight {
    SmallNet,
    BigNet,
    Psqt,
}


/// Number of registers to use for big net accumulator tiling
pub const NUM_REGS_SMALL: usize = 8; // Derived from plugging numbers into best_register_count()
pub const TILE_HEIGHT_SMALL: usize = NUM_REGS_SMALL * std::mem::size_of::<VecT>() / 2;

/// Number of registers to use for big net accumulator tiling
pub const NUM_REGS_BIG: usize = 16; // Derived from plugging numbers into best_register_count()
pub const TILE_HEIGHT_BIG: usize = NUM_REGS_BIG * std::mem::size_of::<VecT>() / 2;

pub const NUM_PSQT_REGS: usize = 1; // Derived from plugging numbers into best_register_count()
pub const PSQT_TILE_HEIGHT: usize = NUM_PSQT_REGS * std::mem::size_of::<PsqtVecT>() / 4;

// pub const NUM_REGS: usize = best_register_count::<Vec_T, WeightType ();


pub fn to_mut_vec_ptr<T>(v: &mut [T], offset: usize) -> *mut VecT {
    unsafe {
        v.as_mut_ptr().add(offset).cast::<VecT>()
    }
}
pub fn to_const_vec_ptr<T>(v: &[T], offset: usize) -> *const VecT {
    unsafe {
        v.as_ptr().add(offset).cast::<VecT>()
    }
}


/// Returns a vector with all lanes set to zero. This mirrors `vec_zero()` in NNUE reference code.
pub fn vec_zero() -> VecT {
    unsafe { std::arch::x86_64::_mm256_setzero_si256() }
}

pub fn vec_srli_epi16<const IMM8: i32>(a: VecT) -> VecT {
    unsafe { std::arch::x86_64::_mm256_srli_epi16::<IMM8>(a) }
}

/// Broadcasts the provided 16-bit value into every lane.
pub fn vec_set1_16(x: i16) -> VecT {
    unsafe { std::arch::x86_64::_mm256_set1_epi16(x) }
}
/// Broadcasts the provided 32-bit value into every lane.
pub fn vec_set1_32(x: i32) -> VecT {
    unsafe { std::arch::x86_64::_mm256_set1_epi32(x) }
}
pub fn vec_set_32(e0: i32, e1: i32, e2: i32, e3: i32, e4: i32, e5: i32, e6: i32, e7: i32) -> VecT {
    unsafe { std::arch::x86_64::_mm256_set_epi32(e0, e1, e2, e3, e4, e5, e6, e7) }
}

const _MM_PERM_BADC: i32 = 0x4E;
const _MM_PERM_CDAB: i32 = 0xB1;

pub fn m256_hadd(sum: VecT, bias: i32) -> i32 {
    unsafe {
        let mut sum128 = _mm_add_epi32(
                _mm256_castsi256_si128(sum), 
                _mm256_extracti128_si256(sum, 1)
        );
        sum128 = _mm_add_epi32(
            sum128, 
            _mm_shuffle_epi32(sum128, _MM_PERM_BADC)
        );
        sum128 = _mm_add_epi32(
            sum128, 
            _mm_shuffle_epi32(sum128, _MM_PERM_CDAB)
        );
        _mm_cvtsi128_si32(sum128) + bias
    }
}

/// Takes the per-lane maximum of two signed 16-bit vectors.
pub fn vec_max_16(a: VecT, b: VecT) -> VecT {
    unsafe { std::arch::x86_64::_mm256_max_epi16(a, b) }
}

/// Takes the per-lane minimum of two signed 16-bit vectors.
pub fn vec_min_16(a: VecT, b: VecT) -> VecT {
    unsafe { std::arch::x86_64::_mm256_min_epi16(a, b) }
}

pub fn vec_add_32(a: VecT, b: VecT) -> VecT {
    unsafe { std::arch::x86_64::_mm256_add_epi32(a, b) }
}
pub fn vec_add_16(a: VecT, b: VecT) -> VecT {
    unsafe { std::arch::x86_64::_mm256_add_epi16(a, b) }
}
pub fn vec_sub_16(a: VecT, b: VecT) -> VecT {
    unsafe { std::arch::x86_64::_mm256_sub_epi16(a, b) }
}
pub fn vec_sub_32(a: VecT, b: VecT) -> VecT {
    unsafe { std::arch::x86_64::_mm256_sub_epi32(a, b) }
}
pub fn vec_madd_16(a: VecT, b: VecT) -> VecT {
    unsafe { std::arch::x86_64::_mm256_madd_epi16(a, b) }
}

/// Multiplies signed 16-bit lanes and keeps the high 16 bits of each 32-bit product. This matches
/// the arithmetic Stockfish uses during the feature transform clamping/multiplication step.
pub fn vec_mulhi_16(a: VecT, b: VecT) -> VecT {
    unsafe { std::arch::x86_64::_mm256_mulhi_epi16(a, b) }
}

/// Packs two signed 16-bit vectors into a single unsigned-saturated 8-bit vector. Used to convert
/// the accumulator halves into the final feature buffer.
pub fn vec_packus_16(a: VecT, b: VecT) -> VecT {
    unsafe { std::arch::x86_64::_mm256_packus_epi16(a, b) }
}
pub fn vec_packus_32(a: VecT, b: VecT) -> VecT {
    unsafe { std::arch::x86_64::_mm256_packus_epi32(a, b) }
}
pub fn vec_load_si256(ptr: *const VecT) -> VecT {
    unsafe { std::arch::x86_64::_mm256_load_si256(ptr) }
}
pub fn vec_store_si256(ptr: *mut VecT, a: VecT) {
    unsafe { std::arch::x86_64::_mm256_store_si256(ptr, a) }
}

pub fn vec_packs_epi16(a: VecT, b: VecT) -> VecT {
    unsafe { std::arch::x86_64::_mm256_packs_epi16(a, b) }
}

pub fn vec_permutevar8x32_epi32(a: VecT, idx: VecT) -> VecT {
    unsafe { std::arch::x86_64::_mm256_permutevar8x32_epi32(a, idx) }
}

/// Shifts every 16-bit lane left by the provided immediate value.
pub fn vec_slli_16<const IMM8: i32>(a: VecT) -> VecT {
    unsafe { std::arch::x86_64::_mm256_slli_epi16::<IMM8>(a) }
}

// TODO: Should we be passing by reference in this file?

pub fn vec_add_dpbusd_epi32(acc: &mut VecT, a: VecT, b: VecT) {
    let mut product0: VecT = unsafe { std::arch::x86_64::_mm256_maddubs_epi16(a, b) };
    product0 = vec_madd_16(product0, vec_set1_16(1));
    *acc = vec_add_32(*acc, product0);
}

pub fn vec_nnz(a: VecT) -> i32 {
    unsafe {
        std::arch::x86_64::_mm256_movemask_ps(std::arch::x86_64::_mm256_castsi256_ps(
            std::arch::x86_64::_mm256_cmpgt_epi32(a, vec_zero()),
        ))
    }
}

pub type Vec128T = std::arch::x86_64::__m128i;

pub fn mm_setzero_si128() -> Vec128T {
    unsafe { std::arch::x86_64::_mm_setzero_si128() }
}
pub fn mm_set1_epi16(x: i16) -> Vec128T {
    unsafe { std::arch::x86_64::_mm_set1_epi16(x) }
}
pub fn mm_load_si128(ptr: *const Vec128T) -> Vec128T {
    unsafe { std::arch::x86_64::_mm_load_si128(ptr) }
}
pub fn mm_storeu_si128(ptr: *mut Vec128T, a: Vec128T) {
    unsafe { std::arch::x86_64::_mm_storeu_si128(ptr, a) }
}
pub fn mm_store_si128(ptr: *mut Vec128T, a: Vec128T) {
    unsafe { std::arch::x86_64::_mm_store_si128(ptr, a) }
}
pub fn mm_add_epi16(a: Vec128T, b: Vec128T) -> Vec128T {
    unsafe { std::arch::x86_64::_mm_add_epi16(a, b) }
}
pub fn mm_mulhi_epi16(a: Vec128T, b: Vec128T) -> Vec128T {
    unsafe { std::arch::x86_64::_mm_mulhi_epi16(a, b) }
}

pub fn mm_packs_epi16(a: Vec128T, b: Vec128T) -> Vec128T {
    unsafe { std::arch::x86_64::_mm_packs_epi16(a, b) }
}
pub fn mm_packs_epi32(a: Vec128T, b: Vec128T) -> Vec128T {
    unsafe { std::arch::x86_64::_mm_packs_epi32(a, b) }
}

pub fn mm_packus_epi32(a: Vec128T, b: Vec128T) -> Vec128T {
    unsafe { std::arch::x86_64::_mm_packus_epi32(a, b) }
}

pub fn mm_srli_epi16<const IMM8: i32>(a: Vec128T) -> Vec128T {
    unsafe { std::arch::x86_64::_mm_srli_epi16::<IMM8>(a) }
}

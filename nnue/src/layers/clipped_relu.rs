use crate::{
    constants::{USE_AVX2, WEIGHT_SCALE_BITS},
    nnue_utils::CacheAligned,
    vectors::{
        Vec_T, Vec128T, mm_packs_epi16, mm_packus_epi32,
        mm_srli_epi16, mm_store_si128, vec_load_si256, vec_packs_epi16, vec_packus_32,
        vec_permutevar8x32_epi32, vec_set_32, vec_srli_epi16, vec_store_si256, mm_load_si128,
    },
};

pub type InputType = i32;
pub type OutputType = u8;

pub struct ClippedReLU<const INPUT_DIM: usize, const PADDED_OUT_DIM: usize> {}
    
impl<const INPUT_DIM: usize, const PADDED_OUT_DIM: usize> ClippedReLU<INPUT_DIM, PADDED_OUT_DIM> {

    pub fn new() -> Self {
        Self {}
    }

    pub fn new_output_buffer(&self) -> CacheAligned<[OutputType; PADDED_OUT_DIM]> {
        CacheAligned([OutputType::default(); PADDED_OUT_DIM])
    }

    pub fn propagate(&self, input: *const InputType, output: *mut OutputType) {
        let mut start = 0;

        if USE_AVX2 {
            if INPUT_DIM % crate::vectors::SIMD_WIDTH == 0 {
                let chunks = INPUT_DIM / crate::vectors::SIMD_WIDTH;
                let offsets = vec_set_32(7, 3, 6, 2, 5, 1, 4, 0);

                let in_vec: *const Vec_T = input as *const Vec_T;
                let out_vec: *mut Vec_T = output as *mut Vec_T;

                for i in 0..chunks {
                    let words0 = vec_srli_epi16::<{ WEIGHT_SCALE_BITS as i32 }>(vec_packus_32(
                        vec_load_si256(unsafe { in_vec.add(i * 4 + 0) }),
                        vec_load_si256(unsafe { in_vec.add(i * 4 + 1) }),
                    ));
                    let words1 = vec_srli_epi16::<{ WEIGHT_SCALE_BITS as i32 }>(vec_packus_32(
                        vec_load_si256(unsafe { in_vec.add(i * 4 + 2) }),
                        vec_load_si256(unsafe { in_vec.add(i * 4 + 3) }),
                    ));

                    vec_store_si256(
                        unsafe { out_vec.add(i) },
                        vec_permutevar8x32_epi32(vec_packs_epi16(words0, words1), offsets),
                    );
                }

                start = INPUT_DIM / crate::vectors::SIMD_WIDTH * crate::vectors::SIMD_WIDTH;
            } else {
                let chunks = INPUT_DIM / (crate::vectors::SIMD_WIDTH / 2);
                let in_vec = input as *const Vec128T;
                let out_vec = output as *mut Vec128T;
                for i in 0..chunks {
                    let words0 = mm_srli_epi16::<{ WEIGHT_SCALE_BITS as i32 }>(mm_packus_epi32(
                        mm_load_si128(unsafe { in_vec.add(i * 4 + 0) }),
                        mm_load_si128(unsafe { in_vec.add(i * 4 + 1) }),
                    ));
                    let words1 = mm_srli_epi16::<{ WEIGHT_SCALE_BITS as i32 }>(mm_packus_epi32(
                        mm_load_si128(unsafe { in_vec.add(i * 4 + 2) }),
                        mm_load_si128(unsafe { in_vec.add(i * 4 + 3) }),
                    ));

                    mm_store_si128(
                        unsafe { out_vec.add(i)},
                        mm_packs_epi16(words0, words1)
                    );
                }

                start =
                    INPUT_DIM / (crate::vectors::SIMD_WIDTH / 2) * (crate::vectors::SIMD_WIDTH / 2);
            }
        }

        for i in start..INPUT_DIM {
            let input_val = unsafe { *input.add(i) } as i128;
            let adjusted = 127.min((input_val * input_val) >> (2 * WEIGHT_SCALE_BITS + 7));
            unsafe {
                output.add(i).write(adjusted as OutputType);
            }
        }
    }
}

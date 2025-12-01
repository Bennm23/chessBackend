use crate::{
    constants::{USE_SSSE3, WEIGHT_SCALE_BITS},
    nnue_utils::CacheAligned,
    vectors::{
        Vec128T, mm_load_si128, mm_mulhi_epi16, mm_packs_epi16, mm_packs_epi32, mm_srli_epi16,
        mm_store_si128,
    },
};

pub type OutputType = u8;
pub type InputType = i32;


pub struct SqClippedReLU<const INPUT_DIM: usize, const PADDED_OUT_DIM: usize> {}

impl<const INPUT_DIM: usize, const PADDED_OUT_DIM: usize> SqClippedReLU<INPUT_DIM, PADDED_OUT_DIM> {
    const NUM_CHUNKS: usize = INPUT_DIM / 16;

    pub fn new() -> Self {
        Self {}
    }

    pub const fn new_output_buffer(&self) -> CacheAligned<[OutputType; PADDED_OUT_DIM]> {
        CacheAligned([0u8; PADDED_OUT_DIM])
    }

    pub fn propagate(&self, input: *const InputType, output: *mut OutputType) {
        let mut start = 0;
        // Really USE_SSE2 in stockfish, but not handling that distinction anyway
        if USE_SSSE3 {
            debug_assert!(WEIGHT_SCALE_BITS == 6);
            let in_vec: *const Vec128T = input as *const Vec128T;
            let out_vec: *mut Vec128T = output as *mut Vec128T;

            for i in 0..Self::NUM_CHUNKS {
                let mut words0 = mm_packs_epi32(
                    mm_load_si128(unsafe { in_vec.add(i * 4 + 0) }),
                    mm_load_si128(unsafe { in_vec.add(i * 4 + 1) }),
                );
                let mut words1 = mm_packs_epi32(
                    mm_load_si128(unsafe { in_vec.add(i * 4 + 2) }),
                    mm_load_si128(unsafe { in_vec.add(i * 4 + 3) }),
                );
                // We shift by WeightScaleBits * 2 = 12 and divide by 128
                // which is an additional shift-right of 7, meaning 19 in total.
                // MulHi strips the lower 16 bits so we need to shift out 3 more to match.
                words0 = mm_srli_epi16::<3>(mm_mulhi_epi16(words0, words0));
                words1 = mm_srli_epi16::<3>(mm_mulhi_epi16(words1, words1));

                mm_store_si128(unsafe { out_vec.add(i) }, mm_packs_epi16(words0, words1));
            }

            start = Self::NUM_CHUNKS * 16;
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

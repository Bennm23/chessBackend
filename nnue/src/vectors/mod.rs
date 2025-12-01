
#[cfg(target_feature = "avx2")]
mod vec_ops_avx2;
#[cfg(target_feature = "avx2")]
pub use crate::vectors::vec_ops_avx2::*;

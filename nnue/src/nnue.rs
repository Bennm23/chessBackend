use std::fmt::{Debug, Display};
use std::fs::File;
use std::io::{self, Read, BufReader};
use std::path::Path;

use crate::constants::{*};
use crate::feature_transformer::FeatureTransformer;
use crate::layers::BucketNet;
use crate::nnue_utils::{*};


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
    use super::*;

    #[test]
    fn test_load_big_nnue() {
        let nnue = load_big_nnue("/home/bmelling/chess/chessBackendWebFinal/nn-1c0000000000.nnue").unwrap();
        println!("{:#?}", nnue);
        assert_eq!(nnue.ft.biases.len(), L1);
        assert_eq!(nnue.ft.weights.len(), L1 * INPUT_DIM);
        assert_eq!(nnue.ft.psqt_weights.len(), PSQT_BUCKETS * INPUT_DIM);
        assert_eq!(nnue.buckets.len(), LAYER_STACKS);
    }
}

use std::{io::{Read, Seek}, iter::Sum, ops::*, sync::Arc};

use binread::{BinRead, BinResult};
use serde::{Deserialize, Serialize};

use crate::serde_extension::SerdeWrapper;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct BinaryMagic<T> {
    architecture: T,
}

impl<T> Deref for BinaryMagic<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.architecture
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for BinaryMagic<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.architecture)
    }
}

impl<T: BinRead<Args = ()> + Copy + PartialEq + Send + Sync + 'static> BinRead for BinaryMagic<T> {
    type Args = (T,);

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &binread::ReadOptions,
        (magic,): Self::Args,
    ) -> BinResult<Self> {
        let architecture = BinRead::read_options(reader, options, ())?;
        if architecture == magic {
            Ok(Self { architecture })
        } else {
            Err(binread::Error::BadMagic {
                pos: reader.stream_position()?,
                found: Box::new(architecture),
            })
        }
    }
}

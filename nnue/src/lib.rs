mod halfkp;
mod layer;
mod math_vec;
mod nnue_utils;
mod serde_extension;

use halfkp::*;
use layer::*;
use math_vec::*;
use nnue_utils::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChessPosition {
    // _piece_masks: [BitBoard; NUM_PIECE_TYPES],
    // _occupied_color: [BitBoard; NUM_COLORS],
    // _occupied: BitBoard,
    // _turn: Color,
    // _castle_rights: [CastleRights; NUM_COLORS],
    // _ep_square: Option<Square>,
    // _pinned: BitBoard,
    // _checkers: BitBoard,
    // _pawn_transposition_hash: u64,
    // _non_pawn_transposition_hash: u64,
    // _transposition_hash: u64,
    // _halfmove_clock: u8,
    // _fullmove_number: NumMoves,
    // _material_scores: [Score; 2],
}

pub trait ClippedRelu<InputType, OutputType, const N: usize> {
    fn clipped_relu(
        &self,
        scale_by_pow_of_two: OutputType,
        min: InputType,
        max: InputType,
    ) -> MathVec<OutputType, N>;

    fn clipped_relu_into(
        &self,
        scale_by_pow_of_two: OutputType,
        min: InputType,
        max: InputType,
        output: &mut [OutputType; N],
    );
}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! get_item_unchecked {
    (@internal $indexable:expr, $index:expr $(,)?) => {
        &$indexable[$index]
    };

    (@internal $indexable:expr, $index:expr, $($rest:expr),+ $(,)?) => {
        get_item_unchecked!(
            @internal
            get_item_unchecked!(@internal $indexable, $index),
            $($rest),+,
        )
    };

    ($($arg:tt)*) => {
        get_item_unchecked!(@internal $($arg)*)
    };
}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! get_item_unchecked_mut {
    (@internal $indexable:expr, $index:expr $(,)?) => {
        &mut $indexable[$index]
    };

    (@internal $indexable:expr, $index:expr, $($rest:expr),+ $(,)?) => {
        get_item_unchecked_mut!(
            @internal
            get_item_unchecked_mut!(@internal $indexable, $index),
            $($rest),+,
        )
    };

    ($($arg:tt)*) => {
        get_item_unchecked_mut!(@internal $($arg)*)
    };
}
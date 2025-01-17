use lazy_static::lazy_static;
use pleco::core::masks::{FILE_CNT, PLAYER_CNT, RANK_CNT, SQ_CNT};



pub type MyVal = i16;


//GAME EVALUATION CONSTANTS
pub const MATE: MyVal = -25_000;
pub const CHECK: MyVal = 14;
pub const STALEMATE: MyVal = 0;


//EVALUATION PATTERN CONSTANTS

pub const PASSED_PAWN: MyVal = 10;
pub const SUPPORTED_PAWN: MyVal = 5;
pub const NEIGHBOR_PAWN: MyVal = 3;
pub const PAWN_ISOLATION: MyVal = 10;

pub const ROOK_MATING: MyVal = 30;
pub const ROOK_OPEN_FILE: MyVal = 30;


pub const TWO_BISHOPS: MyVal = 15;

pub const CASTLE_ABILITY: MyVal = 7;
pub const CASTLE_BONUS: MyVal = 20;
pub const KING_BOTTOM: MyVal = 8;
pub const KING_ISOLATION_PENALTY: MyVal = -10;


//PIECE EVALUATION CONSTANTS
pub const PAWN_VALUE: MyVal = 100;
pub const KNIGHT_VALUE: MyVal = 300;
pub const BISHOP_VALUE: MyVal = 300;
pub const ROOK_VALUE: MyVal = 500;
pub const QUEEN_VALUE: MyVal = 800;
pub const KING_VALUE: MyVal = 1500;



lazy_static! {
    pub static ref PAWN_EARLY_POS: [[MyVal; SQ_CNT]; PLAYER_CNT] = [
           flatten(PAWN_EARLY_POS_EVAL),
           flatten(flip(PAWN_EARLY_POS_EVAL))   
    ];
    pub static ref KNIGHT_POS: [MyVal; SQ_CNT] = flatten(KNIGHT_POS_EVAL);

    pub static ref ROOK_EARLY_POS: [[MyVal; SQ_CNT]; PLAYER_CNT] = [
           flatten(ROOK_LATE_POS_EVAL),
           flatten(flip(ROOK_LATE_POS_EVAL))   
    ];

    pub static ref KING_EARLY_POS: [[MyVal; SQ_CNT]; PLAYER_CNT] = [
           flatten(KING_EARLY_POS_EVAL),
           flatten(flip(KING_EARLY_POS_EVAL))   
    ];
    pub static ref KING_LATE_POS: [[MyVal; SQ_CNT]; PLAYER_CNT] = [
           flatten(KING_LATE_POS_EVAL),
           flatten(flip(KING_LATE_POS_EVAL))   
    ];
}

/// The evaluation of pawns given their position on the board. Only for early game
const PAWN_EARLY_POS_EVAL: [[MyVal; FILE_CNT]; RANK_CNT] = [
    [  0,  0,  0,  0,  0,  0,  0,  0 ], //RANK 8
    [  0,  0,  0,  0,  0,  0,  0,  0 ],
    [  0,  0,  0,  0,  0,  0,  0,  0 ],
    [  0,  0,  4, 15, 15,  4,  0,  0 ],
    [  0,  0,  8, 25, 25,  8,  0,  0 ],
    [  0,  0, 10, 15, 15, 10,  0,  0 ],
    [ 15, 15, 12, 10, 10, 12, 15, 15 ],
    [  0,  0,  0,  0,  0,  0,  0,  0 ], //RANK 1
];


/// The evaluation of a knight position
const KNIGHT_POS_EVAL: [[MyVal; FILE_CNT]; RANK_CNT] = [
    [  0,  0,  0,  0,  0,  0,  0,  0 ], //RANK 8
    [  0,  4,  8, 10, 10,  8,  4,  0 ],
    [  0, 10, 20, 20, 20, 20, 10,  0 ],
    [  0, 10, 20, 20, 20, 20, 10,  0 ],
    [  0, 10, 20, 20, 20, 20, 10,  0 ],
    [  0, 10, 20, 20, 20, 20, 10,  0 ],
    [  0,  4,  8, 10, 10,  8,  4,  0 ],
    [  0,  0,  0,  0,  0,  0,  0,  0 ], //RANK 1
];


/// The evaluation of a rooks position at the end of the game
const ROOK_LATE_POS_EVAL: [[MyVal; FILE_CNT]; RANK_CNT] = [
    [ 25, 25, 25, 25, 25, 25, 25, 25 ], //RANK 8
    [ 25, 25, 25, 25, 25, 25, 25, 25 ],
    [  0,  0,  0,  0,  0,  0,  0,  0 ], 
    [  0,  0,  0,  0,  0,  0,  0,  0 ], 
    [  0,  0,  0,  0,  0,  0,  0,  0 ], 
    [  0,  0,  0,  0,  0,  0,  0,  0 ], 
    [  0,  0,  0,  0,  0,  0,  0,  0 ], 
    [  0,  0,  0,  0,  0,  0,  0,  0 ], //RANK 1
];


/// The evaluation of the king given its position on the board. Only for early game
const KING_EARLY_POS_EVAL: [[MyVal; FILE_CNT]; RANK_CNT] = [
    [  0,  0,  0,  0,  0,  0,  0,  0 ], //RANK 8
    [  0,  0,  0,  0,  0,  0,  0,  0 ],
    [  0,  0,  0,  0,  0,  0,  0,  0 ],
    [  0,  0,  0,  0,  0,  0,  0,  0 ],
    [  0,  0,  0,  0,  0,  0,  0,  0 ],
    [  0,  0,  0,  0,  0,  0,  0,  0 ],
    [  0,  0,  0,  0,  0,  0,  0,  0 ],
    [  0, 15, 25,  0,  0, 15, 25,  0 ], //RANK 1
];

const KING_LATE_POS_EVAL: [[MyVal; FILE_CNT]; RANK_CNT] = [
    [  5, 10, 15, 15, 15, 15, 10,  5 ], //RANK 8
    [  5, 10, 15, 15, 15, 15, 10,  5 ],
    [  5, 10, 15, 15, 15, 15, 10,  5 ],
    [  5, 10, 15, 15, 15, 15, 10,  5 ],
    [  5, 10, 15, 15, 15, 15, 10,  5 ],
    [  5, 10, 15, 15, 15, 15, 10,  5 ],
    [  5, 10, 15, 15, 15, 15, 10,  5 ],
    [  0,  0,  0,  0,  0,  0,  0,  0 ], //RANK 1
];



//  Flips the board, so rank_1 becomes rank_8, rank_8 becomes rank_1, rank_2 becomes rank_7, etc
fn flip(arr: [[MyVal; FILE_CNT]; RANK_CNT]) -> [[MyVal; FILE_CNT]; RANK_CNT] {
    let mut new_arr: [[MyVal; FILE_CNT]; RANK_CNT] = [[0; FILE_CNT]; RANK_CNT];
    for i in 0..RANK_CNT {
        new_arr[i] = arr[7 - i];
    }
    new_arr
}

// Flattens 2D array to a singular 1D array
fn flatten(arr: [[MyVal; FILE_CNT]; RANK_CNT]) -> [MyVal; SQ_CNT] {
    let mut new_arr: [MyVal; SQ_CNT] = [0; SQ_CNT];
    for i in 0..SQ_CNT {
        new_arr[i] = arr[i / 8][i % 8];
    }
    new_arr
}
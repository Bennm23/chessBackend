use lazy_static::lazy_static;
use pleco::core::{masks::{FILE_CNT, PLAYER_CNT, RANK_CNT, SQ_CNT}, score::Score};


// pub const DEFAULT_TT_SIZE: usize = 256;
// pub const PAWN_TABLE_SIZE: usize = 16384;
// pub const MATERIAL_TABLE_SIZE: usize = 8192;

// const TT_ALLOC_SIZE: usize = mem::size_of::<TranspositionTable>();
// const TIMER_ALLOC_SIZE: usize = mem::size_of::<TimeManager>();

// // A object that is the same size as a transposition table
// type DummyTranspositionTable = [u8; TT_ALLOC_SIZE];
// type DummyTimeManager = [u8; TIMER_ALLOC_SIZE];

// pub static USE_STDOUT: AtomicBool = AtomicBool::new(true);

// static INITIALIZED: Once = Once::new();

// /// Global Transposition Table
// static mut TT_TABLE: DummyTranspositionTable = [0; TT_ALLOC_SIZE];

// // Global Timer
// static mut TIMER: DummyTimeManager = [0; TIMER_ALLOC_SIZE];

// #[cold]
// pub fn init_globals() {
//     INITIALIZED.call_once(|| {
//         prelude::init_statics(); // Initialize static tables
//         compiler_fence(Ordering::SeqCst);
//         init_tt(); // Transposition Table
//         init_timer(); // Global timer manager
//         pawn_table::init();
//         threadpool::init_threadpool(); // Make Threadpool
//         search::init();
//     });
// }

// // Initializes the transposition table
// #[cold]
// fn init_tt() {
//     unsafe {
//         let tt = &mut TT_TABLE as *mut DummyTranspositionTable as *mut TranspositionTable;
//         ptr::write(tt, TranspositionTable::new(DEFAULT_TT_SIZE));
//     }
// }

pub type MyVal = i16;


//GAME EVALUATION CONSTANTS
pub const MATE: MyVal = -25_000;
pub const CHECK: MyVal = 14;
pub const STALEMATE: MyVal = 0;


//EVALUATION PATTERN CONSTANTS

pub const PASSED_PAWN_BONUS: Score = Score(10, 30);
pub const PROMOTING_PAWN_BONUS: Score = Score(10, 300);
pub const ADVANCED_PAWN_BONUS: Score = Score(10, 30);
pub const PAWN_STORM_BONUS: Score = Score(10, 20);
pub const DOUBLE_PAWN_PENALTY: Score = Score(15, 15);


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

pub const PAWN_UNIT: MyVal = 100;

pub type EvalVal = i32;
// pub const PAWN_MG: EvalVal = 171;
// pub const KNIGHT_MG: EvalVal = 764;
// pub const BISHOP_MG: EvalVal = 826;
// pub const ROOK_MG: EvalVal = 1282;
// pub const QUEEN_MG: EvalVal = 2526;

// pub const PAWN_EG: EvalVal = 240;
// pub const KNIGHT_EG: EvalVal = 848;
// pub const BISHOP_EG: EvalVal = 891;
// pub const ROOK_EG: EvalVal = 1373;
// pub const QUEEN_EG: EvalVal = 2646;
pub const PAWN_MG:   EvalVal = 100;
pub const KNIGHT_MG: EvalVal = 325;
pub const BISHOP_MG: EvalVal = 325;
pub const ROOK_MG:   EvalVal = 500;
pub const QUEEN_MG:  EvalVal = 1000;

pub const PAWN_EG:   EvalVal = 125;
pub const KNIGHT_EG: EvalVal = 400;
pub const BISHOP_EG: EvalVal = 450;
pub const ROOK_EG:   EvalVal = 650;
pub const QUEEN_EG:  EvalVal = 1300;

pub const MAX_PHASE: EvalVal = 128;

pub const DEFENDED_PIECE_BONUS: Score = Score(100, 100);
pub const CONTESTED_PIECE_BONUS: Score = Score(50, 50);
pub const ATTACKED_PIECE_PENALTY: Score = Score(150, 150);

pub const SOLID_DEFENDER_BONUS: Score = Score(100, 100);
pub const EQUAL_DEFENDER_BONUS: Score = Score(50, 50);
pub const DEFENDER_DIFF_SCORE: Score = Score(30, 30);

pub const MOBILITY_BONUS: [[Score; 32]; 8] = [
    [Score::ZERO; 32], // No Piece
    [Score::ZERO; 32], // Pawns (100 - 125)
    [
        Score(-30, -35), //10% 9%
        Score(-20, -25),
        Score(-5, -15),
        Score(-1, -5),
        Score(3, 2),
        Score(6, 6), // Knights (325 - 400)
        Score(10, 11),
        Score(14, 15),
        Score(18, 20), //5% 4%
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
    ],
    [
        Score(-20, -30), // (5.8%, 6.6%)
        Score(-15, -20),
        Score(5, -5),
        Score(10, 5),
        Score(14, 10),
        Score(18, 20), // Bishops (325 - 450)
        Score(22, 22),
        Score(24, 24),
        Score(26, 30),
        Score(32, 33),
        Score(34, 40),
        Score(35, 43),
        Score(37, 47),
        Score(40, 50), //12% 11%
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
    ],
    [
        Score(-23, -36),
        Score(-10, -9),
        Score(-7, 13),
        Score(-3, 25),
        Score(-1, 33),
        Score(0, 38), // Rooks (500 - 650)
        Score(4, 52),
        Score(6, 56),
        Score(13, 62),
        Score(15, 68),
        Score(18, 73),
        Score(20, 78),
        Score(22, 78),
        Score(24, 80),
        Score(28, 85),
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
    ],
    [
        Score(-16, -18),
        Score(-8, -7),
        Score(1, 4),
        Score(1, 9),
        Score(4, 17),
        Score(6, 27), // Queens (1000 - 1300)
        Score(8, 30),
        Score(8, 36),
        Score(8, 39),
        Score(10, 46),
        Score(10, 47),
        Score(10, 52),
        Score(10, 56),
        Score(12, 60),
        Score(12, 61),
        Score(14, 63),
        Score(14, 66),
        Score(15, 68),
        Score(16, 70),
        Score(17, 71),
        Score(18, 74),
        Score(19, 83),
        Score(20, 85),
        Score(21, 87),
        Score(22, 92),
        Score(25, 95),
        Score(28, 103),
        Score(31, 106),
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
        Score::ZERO,
    ],
    [Score::ZERO; 32], // King
    [Score::ZERO; 32], // All piece
];

lazy_static! {
    pub static ref PAWN_EARLY_POS: [[MyVal; SQ_CNT]; PLAYER_CNT] = [
           flatten(PAWN_EARLY_POS_EVAL),
           flatten(flip(PAWN_EARLY_POS_EVAL))   
    ];
    pub static ref PAWN_LATE_POS: [[MyVal; SQ_CNT]; PLAYER_CNT] = [
           flatten(PAWN_LATE_POS_EVAL),
           flatten(flip(PAWN_LATE_POS_EVAL))   
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
const PAWN_LATE_POS_EVAL: [[MyVal; FILE_CNT]; RANK_CNT] = [
    [650,650,650,650,650,650,650,650 ], //RANK 8
    [250,250,250,250,250,250,250,250 ],
    [100,100, 50, 50, 50, 50,100,100 ],
    [ 25, 25, 20, 20, 20, 20, 25, 25 ],
    [ 15, 15, 10, 10, 10, 10, 15, 15 ],
    [  0,  0,  0,  0,  0,  0,  0,  0 ],
    [  0,  0,  0,  0,  0,  0,  0,  0 ],
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


/// The score of a MVV_LVA[attacker_piece][captured_piece]
/// should encourage the lowest attacker value capturing the
/// highest value capture piece
///
/// We sort in ascending order
///
/// Any piece capturing a more important piece should be heavily
/// encouraged, a piece capturing one of same value is ok
/// and a piece capturing a piece of lower value should be looked
/// at earlier, but is not as important
pub const MVV_LVA: [[MyVal; 6]; 6] = [
    [ -5, -42, -43, -44, -45, -46], //Pawn = 1
    [ -4,  -5, -32, -33, -34, -35], //Knight = 2
    [ -1,  -4,  -5, -23, -24, -25], //Bishop = 3
    [ -1,  -2,  -3,  -5, -14, -15], //Rook = 4
    [ -1,  -2,  -3,  -4,  -5,  -6], //Queen = 5
    [  0,   0,   0,   0,   0,   0], //King = 6
];
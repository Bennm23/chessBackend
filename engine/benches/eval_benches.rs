use std::time::Duration;

use criterion::{black_box, criterion_group, BatchSize, Bencher, Criterion};
use engine::{
    evaluation::eval_board as eval_board,
    tables::{material::Material, pawn_table::PawnTable},
};
use pleco::Board;

fn bench_10_eval_new(b: &mut Bencher, boards: &Vec<Board>) {
    b.iter_batched(
        || {
            let tp: PawnTable = black_box(PawnTable::new());
            let tm: Material = black_box(Material::new());
            (tp, tm)
        },
        |(mut pawn_table, mut material)| {
            for board in boards.iter() {
                black_box(eval_board(&board, &mut pawn_table, &mut material));
            }
        },
        BatchSize::PerIteration,
    );
}

fn bench_engine_evaluations(c: &mut Criterion) {
    let boards: Vec<Board> = RAND_BOARD_NON_CHECKS_10
        .iter()
        .map(|b| Board::from_fen(b).unwrap())
        .collect();

    c.bench_function("New Full Evaluation", |b| bench_10_eval_new(b, &boards));
}

criterion_group!(name = eval_benches;
    config = Criterion::default()
       .sample_size(250)
       .warm_up_time(Duration::from_millis(40));
   targets = bench_engine_evaluations
);

static RAND_BOARD_NON_CHECKS_10: [&str; 10] = [
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "3qkb1r/3ppp2/3r1np1/2Q4p/5P2/1P3B2/P1P1PP1P/R2NK2R b k - 0 22",
    "2r1r3/3k4/1qpn1p2/8/RP1pP3/3R1PPp/1p5P/1N4K w - - 2 39",
    "r1bqkbnr/ppppppp1/n7/3P2p1/Q4P2/2P5/PP2P1PP/RN2KBNR b KQkq - 2 6",
    "2Q5/4k1b1/6p1/5p1p/pP1P1P2/2P5/5RPP/5RK w - - 5 45",
    "r2qkb2/1ppbpp2/p6r/3p4/6P1/1PP1P1QP/P2N1P2/RN2KB1R b KQq - 4 20",
    "r3k1nr/pp1n1pbp/1qp1p1p1/6B1/P2PP1P1/1Pp2N2/2P2P2/R2QKB1R b KQkq - 0 13",
    "5rk1/3rbp1p/4p3/1N5p/5P2/1PNP2P1/1BK4P/4R b - - 3 35",
    "r2qkbnr/pp2p1pp/2p1b3/3pNpB1/3P4/8/PP1NPPPP/R2QKB1R w KQkq - 2 8",
    "r1bqk2r/pppp3p/5b2/1P6/5p2/P5P1/1QP1P2P/RN2KB1R b KQkq - 2 16",
];
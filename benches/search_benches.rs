use std::time::Duration;

use chess_lib::processing::{eval_board, searching::find_best_move};
use criterion::{black_box, criterion_group, BatchSize, Bencher, Criterion, Fun};
use pleco::{board, Board};

const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -";

fn bench_search_kiwipete(b: &mut Bencher, depth: u8) {
    b.iter_batched(
        || {
            Board::from_fen(KIWIPETE).expect("KIWIPETE Init Failed")
        },
        |mut board| {
            let mov = black_box(find_best_move(&mut board, depth));
            board.apply_move(mov);
            black_box(find_best_move(&mut board, depth));
        },
        BatchSize::PerIteration
    )
}

fn bench_search_default(b: &mut Bencher, depth: u8) {
    b.iter_batched(
        || {
            Board::start_pos()
        },
        |mut board| {
            let mov = black_box(find_best_move(&mut board, depth));
            board.apply_move(mov);
            black_box(find_best_move(&mut board, depth));
        },
        BatchSize::PerIteration
    )
}

fn bench_engine_search(c: &mut Criterion) {
    c.bench_function(
        "Search Default Depth 5", 
        |b| { bench_search_default(b, 5);},
    );
    c.bench_function(
        "Search Default Depth 6", 
        |b| { bench_search_default(b, 6);},
    );
    // c.bench_function(
    //     "Search Default Depth 7", 
    //     |b| { bench_search_default(b, 7);},
    // );

    c.bench_function(
        "Search Kiwipete Depth 5", 
        |b| { bench_search_kiwipete(b, 5);},
    );
    c.bench_function(
        "Search Kiwipete Depth 6", 
        |b| { bench_search_kiwipete(b, 6);},
    );
    // c.bench_function(
    //     "Search Kiwipete Depth 7", 
    //     |b| { bench_search_kiwipete(b, 7);},
    // );
}

criterion_group!(name = search_benches;
    config = Criterion::default()
       .sample_size(10)
       .warm_up_time(Duration::from_millis(150));
   targets = bench_engine_search
);

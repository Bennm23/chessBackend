use std::time::Duration;

use criterion::{BatchSize, Bencher, BenchmarkId, Criterion, black_box, criterion_group};
use engine::{
    debug::{NoTrace, Tracing},
    final_search::MySearcher,
};
use pleco::Board;

const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -";
const SEARCH_DEPTHS: [u8; 3] = [6, 7, 8];

// fn bench_engine_search(c: &mut Criterion) {

//     let mut group = c.benchmark_group("engine_search");

//     for depth in SEARCH_DEPTHS {
//         // ---- Default ----
//         let default_time = match depth {
//             6 => Duration::from_secs(3),
//             7 => Duration::from_secs(3),
//             8 => Duration::from_secs(5),
//             _ => Duration::from_secs(5),
//         };

//         group.measurement_time(default_time);
//         group.sample_size(10);

//         group.bench_with_input(
//             BenchmarkId::new("Search Default Depth", depth),
//             &depth,
//             |b, &d| {
//                 b.iter_batched(
//                     || (Board::start_pos(), MySearcher::new(NoTrace::new(), None)),
//                     |(mut board, mut searcher)| {
//                         black_box(searcher.find_best_move(&mut board, d));
//                     },
//                     BatchSize::PerIteration,
//                 );
//             },
//         );

//         // ---- Kiwipete ----
//         let kiwipete_time = match depth {
//             6 => Duration::from_secs_f64(8.0),   // Criterion recommended ~7.2s
//             7 => Duration::from_secs_f64(30.0),  // Criterion recommended ~29.7s
//             8 => Duration::from_secs_f64(70.0),  // Criterion recommended ~68.9s
//             _ => Duration::from_secs(10),
//         };

//         group.measurement_time(kiwipete_time);
//         group.sample_size(10);

//         group.bench_with_input(
//             BenchmarkId::new("Search Kiwipete Depth", depth),
//             &depth,
//             |b, &d| {
//                 b.iter_batched(
//                     || (Board::from_fen(KIWIPETE).unwrap(), MySearcher::new(NoTrace::new(), None)),
//                     |(mut board, mut searcher)| {
//                         black_box(searcher.find_best_move(&mut board, d));
//                     },
//                     BatchSize::PerIteration,
//                 );
//             },
//         );
//     }

//     group.finish();
// }

// criterion_group!(
//     name = search_benches;
//     config = Criterion::default()
//         .warm_up_time(Duration::from_secs(1));
//     targets = bench_engine_search
// );
fn bench_search_kiwipete(b: &mut Bencher, depth: u8) {
    b.iter_batched(
        || {
            (
                Board::from_fen(KIWIPETE).expect("KIWIPETE Init Failed"),
                MySearcher::new(NoTrace::new(), None),
            )
        },
        |(mut board, mut searcher)| {
            let mov = black_box(searcher.find_best_move(&mut board, depth));
            board.apply_move(mov);
            black_box(searcher.find_best_move(&mut board, depth));
        },
        BatchSize::PerIteration,
    )
}

fn bench_search_default(b: &mut Bencher, depth: u8) {
    b.iter_batched(
        || (Board::start_pos(), MySearcher::new(NoTrace::new(), None)),
        |(mut board, mut searcher)| {
            let mov = black_box(searcher.find_best_move(&mut board, depth));
            board.apply_move(mov);
            black_box(searcher.find_best_move(&mut board, depth));
        },
        BatchSize::PerIteration,
    )
}

fn bench_engine_search(c: &mut Criterion) {
    for depth in SEARCH_DEPTHS {
        let bench_name = format!("Search Default Depth {}", depth);
        c.bench_function(&bench_name, |b| {
            bench_search_default(b, depth);
        });
    }

    for depth in SEARCH_DEPTHS {
        let bench_name = format!("Search Kiwipete Depth {}", depth);
        c.bench_function(&bench_name, |b| {
            bench_search_kiwipete(b, depth);
        });
    }
}

criterion_group!(name = search_benches;
    config = Criterion::default()
       .sample_size(10)
       .warm_up_time(Duration::from_millis(150));
   targets = bench_engine_search
);

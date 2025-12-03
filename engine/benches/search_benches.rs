use std::{sync::LazyLock, time::Duration};

use criterion::{BatchSize, Bencher, Criterion, black_box, criterion_group};
use engine::{
    debug::{NoTrace, Tracing},
    search_wip::MySearcher,
};
use pleco::Board;

const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -";
const SEARCH_DEPTHS: [u8; 3] = [6, 7, 8];

use std::sync::Mutex;
use nnue::nnue::NnueEvaluator;

static NNUE_EVAL: LazyLock<Mutex<NnueEvaluator>> = LazyLock::new(|| {
    Mutex::new(NnueEvaluator::new())
});


fn bench_search_kiwipete(b: &mut Bencher, depth: u8) {
    b.iter_batched(
        || Board::from_fen(KIWIPETE).unwrap(),
        |mut board| {
            let mv = run_search(&mut board, depth);
            board.apply_move(mv);
            run_search(&mut board, depth);
        },
        BatchSize::PerIteration,
    );
}
fn run_search(board: &mut Board, depth: u8) -> pleco::BitMove {
    let mut guard = NNUE_EVAL.lock().unwrap();
    let mut searcher = MySearcher::new(&mut *guard, NoTrace::new(), None);
    // let mut searcher = MySearcher::new(NoTrace::new(), None);
    black_box(searcher.find_best_move(board, depth))
}
fn bench_search_default(b: &mut Bencher, depth: u8) {
    b.iter_batched(
        || Board::start_pos(),
        |mut board| {
            let mv = run_search(&mut board, depth);
            board.apply_move(mv);
            run_search(&mut board, depth);
        },
        BatchSize::PerIteration,
    );
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

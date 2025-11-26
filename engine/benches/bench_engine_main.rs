#[macro_use]
extern crate criterion;

mod eval_benches;
mod search_benches;

criterion_main!(
    // eval_benches::eval_benches,
    search_benches::search_benches,
);
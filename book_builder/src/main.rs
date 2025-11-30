mod lumbras;
mod utils;

use book::{save_book_to_ron};
use lumbras::load_lumbras_book;

fn main() {
    const FNAME: &str = "My_Book";
    // const FNAME: &str = "Benn_Test";
    let file_path = format!("/home/bmellin/chess_db/openings/{}.pgn", FNAME);

    let book = load_lumbras_book(
        &file_path,
        2600,
        5,
        Some(10)
    );
    let out_name = format!("lumbras_{}.ron", FNAME.to_lowercase());
    println!("Saving to {}", out_name);

    if let Some(book) = book {
        println!("Loaded book with {} positions.", book.len());
        save_book_to_ron(&book, &out_name).expect("Failed to Save ron");
    } else {
        println!("Failed to load book.");
    }

}

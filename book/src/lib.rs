use std::collections::HashMap;
use rand::Rng;
use ron::ser::{to_string_pretty, PrettyConfig};
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::Write;

/// A Book Move Entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BookMove {
    mv: String,
    weight: u32,
}

pub type Book = HashMap<String, Vec<BookMove>>;

pub fn add_book_move(book: &mut Book, fen: &str, mv: &str) {
    // Get or create vector for this FEN
    let entry = book.entry(fen.to_string()).or_default();

    // Look for existing move
    if let Some(book_move) = entry.iter_mut().find(|bm| bm.mv == mv) {
        // Increment weight
        book_move.weight += 1;
    } else {
        // Insert new
        entry.push(BookMove {
            mv: mv.to_string(),
            weight: 1,
        });
    }
}
fn choose_weighted_move(moves: &[BookMove]) -> &BookMove {
    assert!(!moves.is_empty(), "Book entry cannot be empty");

    // Total weight
    let total: u32 = moves.iter().map(|m| m.weight).sum();

    // Choose a random number in [0, total)
    let mut rng = rand::rng();
    let mut roll = rng.random_range(0 .. total);

    // Walk the list until we find where `roll` lands
    for m in moves {
        if roll < m.weight {
            return m;
        }
        roll -= m.weight;
    }

    // Should never happen
    panic!("Failed to choose weighted move");
}

pub fn get_book_move(book: &Book, fen: &str) -> Option<String> {
    let fen = normalize_fen(fen, None);
    if let Some(moves) = book.get(&fen) {
        // Select a move randomly, but influenced by weight
        let mv = choose_weighted_move(moves);
        return Some(mv.mv.clone());
    }
    None
}
pub fn load_from_ron(path: &str) -> Book {
    // Read the file
    let ron_string = std::fs::read_to_string(path).expect("Failed to read RON file");

    // Deserialize
    let book: Book = ron::from_str(&ron_string).expect("Failed to parse RON");

    book
}

pub fn save_book_to_ron(book: &Book, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pretty = PrettyConfig::default();

    // Serialize the book to a pretty RON string
    let ron_string = to_string_pretty(book, pretty)?;

    // Write to file
    let mut file = File::create(path)?;
    file.write_all(ron_string.as_bytes())?;

    Ok(())
}


pub fn print_book(book: &Book) {
    for (fen, moves) in book {
        println!("FEN: {}", fen);
        for bm in moves {
            println!("  Move: {}, Weight: {}", bm.mv, bm.weight);
        }
    }
}
/// Normalize a FEN so only meaningful book fields remain:
/// 1. board
/// 2. side to move
/// 3. castling
/// 4. en passant
#[inline(always)]
pub fn normalize_fen(fen: &str, ep_square: Option<String>) -> String {
    let parts: Vec<&str> = fen.split_whitespace().collect();

    // parts[0] = board layout
    // parts[1] = side to move
    // parts[2] = castling rights
    // parts[3] = en-passant square
    // format!("{} {} {} {}", parts[0], parts[1], parts[2], parts[3])
    //TODO: Don't use e.p. info for book, pleco not updating it right?
    format!("{} {} {} {}", parts[0], parts[1], parts[2], ep_square.unwrap_or(parts[3].to_string()))
}

// #[cfg(test)]
// mod tests {
//     use pleco::Board;

//     use super::*;

//     #[test]
//     fn test_normalize_fen_startpos() {
//         let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -";
//         let norm = normalize_fen(fen);
//         assert_eq!(
//             norm,
//             "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -"
//         );
//     }

//     #[test]
//     fn test_normalize_fen_after_move() {
//         let mut board = Board::start_pos();
//         board.apply_uci_move("e2e4");

//         let fen = board.fen();
//         let norm = normalize_fen(&fen);

//         assert_eq!(
//             norm,
//             "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq -"
//         );
//     }

//     #[test]
//     fn test_insert_into_empty_book() {
//         let mut book: Book = HashMap::new();
//         let fen = "startpos";
//         add_book_move(&mut book, fen, "e2e4");

//         let entry = book.get(fen).unwrap();
//         assert_eq!(entry.len(), 1);
//         assert_eq!(entry[0].mv, "e2e4");
//         assert_eq!(entry[0].weight, 1);
//     }

//     #[test]
//     fn test_increment_existing_move() {
//         let mut book: Book = HashMap::new();
//         let fen = "startpos";

//         add_book_move(&mut book, fen, "e2e4");
//         add_book_move(&mut book, fen, "e2e4");

//         let entry = book.get(fen).unwrap();
//         assert_eq!(entry.len(), 1);
//         assert_eq!(entry[0].mv, "e2e4");
//         assert_eq!(entry[0].weight, 2);
//     }

//     #[test]
//     fn test_insert_multiple_moves_same_fen() {
//         let mut book: Book = HashMap::new();
//         let fen = "startpos";

//         add_book_move(&mut book, fen, "e2e4");
//         add_book_move(&mut book, fen, "d2d4");
//         add_book_move(&mut book, fen, "g1f3");

//         let entry = book.get(fen).unwrap();
//         assert_eq!(entry.len(), 3);

//         assert_eq!(entry[0].weight, 1);
//         assert_eq!(entry[1].weight, 1);
//         assert_eq!(entry[2].weight, 1);
//     }

//     #[test]
//     fn test_multiple_fens() {
//         let mut book: Book = HashMap::new();

//         add_book_move(&mut book, "fen1", "e2e4");
//         add_book_move(&mut book, "fen2", "d2d4");

//         assert_eq!(book.get("fen1").unwrap()[0].mv, "e2e4");
//         assert_eq!(book.get("fen2").unwrap()[0].mv, "d2d4");

//         assert!(book.get("fen1").unwrap().len() == 1);
//         assert!(book.get("fen2").unwrap().len() == 1);
//     }

//     #[test]
//     fn test_increment_one_move_but_not_others() {
//         let mut book: Book = HashMap::new();
//         let fen = "startpos";

//         add_book_move(&mut book, fen, "e2e4");
//         add_book_move(&mut book, fen, "d2d4");
//         add_book_move(&mut book, fen, "e2e4");

//         let entry = book.get(fen).unwrap();

//         assert_eq!(entry.len(), 2);

//         let e4 = entry.iter().find(|m| m.mv == "e2e4").unwrap();
//         let d4 = entry.iter().find(|m| m.mv == "d2d4").unwrap();

//         assert_eq!(e4.weight, 2);
//         assert_eq!(d4.weight, 1);
//     }
// }
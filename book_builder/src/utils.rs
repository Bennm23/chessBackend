use std::{fs::File, io::{BufRead, BufReader}, str::FromStr};

pub fn file_to_lines(file_path: &str) -> Vec<String> {
    println!("Reading lines from file: {}", file_path);
    let file = File::open(file_path);
    if let Err(e) = file {
        panic!("Cannot open file {}: {:?}", file_path, e);
    }
    let file = file.expect("Failed to convert file to lines");
    let reader = BufReader::new(file);

    reader.lines()
        .map(|line| line.expect("Failed to read line").trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}


pub fn san_to_uci(san: &str, fen: &str) -> Option<String> {
    // println!("Converting SAN '{}' on FEN '{}'", san, fen);
    let san_board = match chess::Board::from_str(fen) {
        Ok(board) => board,
        Err(_) => {
            // println!("Failed to parse FEN: {} due to {}", fen, e);
            return None;
        }
    };
    // Need to remove = from promotion moves
    let san_move = chess::ChessMove::from_san(&san_board, &san.replace("=", ""));
    match san_move {
        Ok(mv) => {
            // println!("Converted SAN '{}' to UCI '{}'", san, mv.to_string());
            Some(mv.to_string())
        }
        Err(_) => {
            None
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_san_to_uci() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let san = "e4";
        let uci = san_to_uci(san, fen);

        println!("UCI Move: {:?}", uci);

        let uci = uci.expect("Failed to convert SAN to UCI");
        assert_eq!(uci, "e2e4");
    }
}
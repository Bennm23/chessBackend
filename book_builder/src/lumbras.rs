use book::Book;
use chess::Rank;
use pleco::SQ;
use crate::utils;
use std::{io, ops::ControlFlow};
use pgn_reader::{RawTag, Reader, SanPlus, Skip, Visitor};


pub type LumbrasPgnSet = Vec<LumbrasPgnEntry>;

pub struct LumbrasPgnEntry {
    min_rating: u32,
    move_sequence: Vec<String>,
}

pub struct LumbrasVisitor {
    white_elo: Option<u32>,
    black_elo: Option<u32>,
    max_depth: Option<usize>,
}

impl LumbrasVisitor {
    pub fn new(max_depth: Option<usize>) -> Self {
        Self {
            white_elo: None,
            black_elo: None,
            max_depth,
        }
    }
}

impl Visitor for LumbrasVisitor {
    // Returned from begin_tags()
    type Tags = ();

    // The per-game mutable movetext state
    type Movetext = Vec<String>;

    // The output for a completed game
    type Output = LumbrasPgnEntry;

    // --- TAGS ------------------------------------------------------

    fn begin_tags(&mut self) -> ControlFlow<Self::Output, Self::Tags> {
        // Reset for each game
        self.white_elo = None;
        self.black_elo = None;
        ControlFlow::Continue(())
    }

    fn tag(&mut self, _tags: &mut Self::Tags, name: &[u8], value: RawTag<'_>) -> ControlFlow<Self::Output> {
        let name = std::str::from_utf8(name).unwrap_or("");
        let value = value.decode_utf8().unwrap_or_default();
        match name {
            "WhiteElo" => {
                if let Ok(v) = value.parse::<u32>() {
                    self.white_elo = Some(v);
                }
            }
            "BlackElo" => {
                if let Ok(v) = value.parse::<u32>() {
                    self.black_elo = Some(v);
                }
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }

    // --- MOVETEXT --------------------------------------------------

    fn begin_movetext(
        &mut self,
        _tags: Self::Tags,
    ) -> ControlFlow<Self::Output, Self::Movetext> {
        // Start a new vector for SAN moves
        ControlFlow::Continue(Vec::new())
    }

    fn san(
        &mut self,
        moves: &mut Self::Movetext,
        san_plus: SanPlus,
    ) -> ControlFlow<Self::Output> {
        if let (Some(max_depth), moves_len) = (self.max_depth, moves.len()) {
            if moves_len >= max_depth {
                return ControlFlow::Continue(());
            }
        }
        moves.push(san_plus.to_string());
        ControlFlow::Continue(())
    }

    fn begin_variation(
        &mut self,
        _movetext: &mut Self::Movetext,
    ) -> ControlFlow<Self::Output, Skip> {
        // Skip ALL side variations, stay in mainline only
        ControlFlow::Continue(Skip(true))
    }

    // --- OUTPUT ----------------------------------------------------

    fn end_game(&mut self, moves: Self::Movetext) -> Self::Output {
        let w = self.white_elo.unwrap_or(0);
        let b = self.black_elo.unwrap_or(0);

        LumbrasPgnEntry {
            min_rating: w.min(b),
            move_sequence: moves,
        }
    }
}

impl std::fmt::Display for LumbrasPgnEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LumbrasPgnEntry(min_rating: {}, moves: {:?})", self.min_rating, self.move_sequence)
    }
}


fn pgn_lines_to_entry(lines: &[String], max_moves: Option<usize>) -> LumbrasPgnEntry {
    let mut reader = Reader::new(io::Cursor::new(lines.join("\n")));
    let mut visitor = LumbrasVisitor::new(max_moves);

    let moves = reader.read_game(&mut visitor).unwrap();
    assert!(moves.is_some());
    let entry = moves.unwrap();
    entry
}
fn split_to_pgn_sets(lines: Vec<String>, max_moves: Option<usize>) -> Vec<LumbrasPgnEntry> {
    let mut result: Vec<LumbrasPgnEntry> = Vec::new();
    let mut current: Vec<String> = Vec::new();

    for line in lines {
        
        if line.starts_with("[Event") {
            if !current.is_empty() {
                result.push(pgn_lines_to_entry(&current, max_moves));
                current = Vec::new();
            }
        }
        current.push(line);
    }

    // Push last block if any
    if !current.is_empty() {
        result.push(pgn_lines_to_entry(&current, max_moves));
    }

    result
}


fn load_lumbras_pgns(
    file_path: &str,
    min_elo: u32,
    min_moves: usize,
    max_moves: Option<usize>,
) -> LumbrasPgnSet {
    let lines = utils::file_to_lines(file_path);
    let pgn_groups  = split_to_pgn_sets(lines, max_moves);
    let mut pgn_set: LumbrasPgnSet = Vec::new();

    println!("Total PGN groups: {}", pgn_groups.len());

    for pgn_entry in pgn_groups {
        if pgn_entry.min_rating < min_elo {
            continue;
        }
        if pgn_entry.move_sequence.len() < min_moves {
            continue;
        }
        if let Some(max) = max_moves {
            if pgn_entry.move_sequence.len() > max {
                println!("Skipping entry with {} moves > max {}", pgn_entry.move_sequence.len(), max);
                continue;
            }
        }

        pgn_set.push(pgn_entry);
    }

    pgn_set
}

pub fn load_lumbras_book(
    file_path: &str,
    min_elo: u32,
    min_moves: usize,
    max_moves: Option<usize>,
) -> Option<Book> {
    // Placeholder implementation: In a real scenario, this would load from a file or database.
    let pgns = load_lumbras_pgns(&file_path, min_elo, min_moves, max_moves);
    println!("Loaded {} PGN entries from Lumbras book.", pgns.len());

    let mut book : Book = Book::new();

    let mut ctr = 0;
    'entry_loop: for entry in pgns.iter() {
        // We have the move list, for each move

        ctr += 1;
        if ctr % 100 == 0 {
            println!("Processing entry {}", ctr);
        }
        let mut board = pleco::Board::start_pos();
        // println!("--------------------------------");
        // println!("Processing Entry {}", entry);

        // let mut mv_cntr = 0;
        // Ensure all are good
        for mv in &entry.move_sequence {

            let fen = board.fen();
            let uci_move = match utils::san_to_uci(&mv, &fen) {
                Some(uci) => uci,
                None => {
                    // println!("Failed to convert SAN '{}' on FEN '{}'", mv, fen);
                    continue 'entry_loop;
                }
            };
            let res = board.apply_uci_move(&uci_move);
            if !res {
                panic!("Failed to apply move {} on fen {}", mv, fen);
            }
        }

        board = pleco::Board::start_pos();

        for mv in &entry.move_sequence {

            let fen = board.fen();
            let uci_move = match utils::san_to_uci(&mv, &fen) {
                Some(uci) => uci,
                None => {
                    continue 'entry_loop;
                }
            };

            let ep_square: Option<String> = if let Some(mv) = board.last_move() {
                let (is_double, double) = mv.is_double_push();
                if is_double {
                    if board.turn() == pleco::Player::White {
                        // Then black did the double push
                        Some(SQ(double + 8).to_string())
                    } else {
                        // Then white did the double push
                        Some(SQ(double - 8).to_string())
                    }
                } else {
                    None
                }
            } else {
                None
            };
            let fen = book::normalize_fen(&fen, ep_square);
            book::add_book_move(&mut book, &fen, &uci_move);

            let res = board.apply_uci_move(&uci_move);
            if !res {
                panic!("Failed to apply move {} on fen {}", mv, board.fen());
            }
        }
    }

    Some(book)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_san_clock() {
        let pgn = r#"[Event "CCT Chess.com Classic 2025 | Play-in - Match Play"]
[Site "?"]
[Date "2025.05.19"]
[Round "2"]
[White "Dubov, Daniil"]
[Black "Erigaisi Arjun"]
[Result "0-1"]
[WhiteElo "2688"]
[BlackElo "2708"]
[ECO "E05l"]
[ImportDate "2025-06-03"]
[Source "LichessBroadcast"]
[WhiteTitle "GM"]
[BlackTitle "GM"]
[TimeControl "600"]
[WhiteFideId "24126055"]
[BlackFideId "35009192"]

1. d4 {[%clk 0:09:59]} 1. ... d5 {[%clk 0:09:58]} 2. c4 {[%clk 0:09:59]} 
2. ... e6 {[%clk 0:09:57]} 3. Nf3 {[%clk 0:09:58]} 3. ... Nf6 {[%clk
0:09:56]} 4. g3 {[%clk 0:09:58]} 4. ... Be7 {[%clk 0:09:53]} 5. Bg2 {[%clk
0:09:56]} 5. ... O-O {[%clk 0:09:53]} 6. O-O {[%clk 0:09:56]} 6. ... dxc4 
{[%clk 0:09:52]} 7. Qc2 {[%clk 0:09:55]} 7. ... a6 {[%clk 0:09:50]} 8. a4 
{[%clk 0:09:54]} 8. ... Bd7 {[%clk 0:09:47]} 9. Rd1 {[%clk
0:09:52]} 9. ... Bc6 {[%clk 0:09:44]} 10. Bg5 {[%clk 0:09:51]} 10. ... 
Nbd7 {[%clk 0:09:30]} 11. Qxc4 {[%clk 0:09:47]} 11. ... h6 {[%clk
0:09:25]} 12. Bxf6 {[%clk 0:09:46]} 12. ... Nxf6 {[%clk 0:09:24]} 13. Nc3 
{[%clk 0:09:44]} 13. ... a5 {[%clk 0:09:22]} 14. Ne5 {[%clk 0:09:43]} 
14. ... Bxg2 {[%clk 0:09:21]} 15. Kxg2 {[%clk 0:09:43]} 15. ... c6 {[%clk 
0:09:21]} 16. e3 {[%clk 0:09:42]} 16. ... Qb6 {[%clk 0:09:18]} 17. Qe2 {
[%clk 0:09:41]} 17. ... Qa6 {[%clk 0:09:13]} 18. Qf3 {[%clk
0:09:39]} 18. ... Rad8 {[%clk 0:09:02]} 19. h4 {[%clk 0:09:35]} 19. ... 
Nd5 {[%clk 0:08:52]} 20. Ne4 {[%clk 0:09:23]} 20. ... Qb6 {[%clk
0:08:39]} 21. Rd2 {[%clk 0:09:06]} 21. ... Qb3 {[%clk 0:08:26]} 22. Nd3 {
[%clk 0:08:51]} 22. ... Qb6 {[%clk 0:08:16]} 23. Rc1 {[%clk 0:08:39]} 
23. ... Qb3 {[%clk 0:08:07]} 24. g4 {[%clk 0:08:31]} 24. ... Nf6 {[%clk
0:05:46]} 25. Nec5 {[%clk 0:07:34]} 25. ... Bxc5 {[%clk 0:05:41]} 26. Nxc5
{[%clk 0:07:34]} 26. ... Qb4 {[%clk 0:05:40]} 27. Rd3 {[%clk
0:07:21]} 27. ... Qxb2 {[%clk 0:05:32]} 28. Qd1 {[%clk 0:07:17]} 28. ... 
Qb6 {[%clk 0:05:14]} 29. g5 {[%clk 0:06:51]} 29. ... Nd5 {[%clk 0:05:06]} 
30. gxh6 {[%clk 0:06:42]} 30. ... gxh6 {[%clk 0:04:49]} 31. Rb1 {[%clk
0:06:08]} 31. ... Nb4 {[%clk 0:04:48]} 32. Qh5 {[%clk 0:06:02]} 32. ... 
Kh7 {[%clk 0:04:23]} 33. e4 {[%clk 0:05:25]} 33. ... Nxd3 {[%clk
0:04:13]} 0-1
"#;
        let mut reader = Reader::new(io::Cursor::new(&pgn));
        let mut visitor = LumbrasVisitor::new(None);

        let moves = reader.read_game(&mut visitor).unwrap();
        assert!(moves.is_some());
        let entry = moves.unwrap();
        println!("Entry: {}", entry);
        assert_eq!(entry.min_rating, 2688);
        assert_eq!(entry.move_sequence.len(), 66);
        assert_eq!(entry.move_sequence[0], "d4");
        assert_eq!(entry.move_sequence[65], "Nxd3");

    }

    #[test]
    fn test_san_base() {
        let pgn = r#"
[Event "Titled Tue 27. May Late"]
[Site "chess.com INT"]
[Date "2025.05.27"]
[Round "6"]
[White "Ermolaev, Evgeny"]
[Black "Chelly, Yahya"]
[Result "1-0"]
[WhiteElo "2282"]
[BlackElo "2039"]
[ECO "E04b"]
[EventDate "2025.05.27"]
[ImportDate "2025-06-03"]
[Source "TWIC"]
[WhiteTitle "FM"]
[BlackTitle "CM"]
[WhiteFideId "24103195"]
[BlackFideId "5516200"]

1. d4 d5 2. c4 e6 3. Nf3 Nf6 4. g3 dxc4 5. Bg2 Bb4+ 6. Bd2 a5 7. Qc2 Bxd2+
8. Qxd2 O-O 9. Ne5 c6 10. Nxc4 b5 11. Ne5 Qb6 12. O-O Rd8 13. e3 Nbd7 14. 
Nxc6 Bb7 15. Nxd8 Bxg2 16. Kxg2 Rxd8 17. Nc3 b4 18. Ne2 Ne4 19. Qc2 Ndf6 
20. Rac1 Qb7 21. Qc6 Qe7 22. Qc7 Rd7 23. Qxa5 h5 24. Nf4 g5 25. Nxh5 Nxh5 
26. Qa8+ Kg7 27. Qxe4 Nf6 28. Qe5 Rd5 29. Qc7 Qf8 30. Qc8 Qd6 31. Rc6 Qe7 
32. Rfc1 Ne4 33. f3 Nd2 34. R6c2 Nxf3 35. Kxf3 Qf6+ 36. Kg2 g4 37. Rf2 Qh6
38. Rcf1 f5 39. e4 Rxd4 40. exf5 exf5 41. Qxf5 Qh3+ 42. Kg1 Rd6 43. Qf8+ 
Kg6 44. Qxd6+ Kg5 45. Qe7+ 1-0"#;
        let mut reader = Reader::new(io::Cursor::new(&pgn));
        let mut visitor = LumbrasVisitor::new(None);

        let moves = reader.read_game(&mut visitor).unwrap();
        assert!(moves.is_some());
        let entry = moves.unwrap();
        assert_eq!(entry.min_rating, 2039);
        assert_eq!(entry.move_sequence.len(), 89);
        assert_eq!(entry.move_sequence[0], "d4");
        assert_eq!(entry.move_sequence[88], "Qe7+");
    }
}
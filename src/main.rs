pub mod generated;
pub mod processing;

use generated::chess::{self, FindBest, FindBestResponse, GetValidMoves, Position, ValidMovesResponse};
use generated::common::MessageID;
use pleco::bots::{AlphaBetaSearcher, IterativeSearcher};
use pleco::tools::Searcher;
use pleco::{Piece, PieceType, SQ};
use processing::searching::find_best_move;
use protobuf::EnumOrUnknown;
use protobuf::{Enum, Message, MessageField};
use std::str::FromStr;
use std::time::Instant;
use std::{
    io::{Read, Write},
    net::TcpStream,
    thread,
};

fn main() {
    let serv = thread::spawn(|| server());

    serv.join().unwrap();
}

fn server() {
    println!("Starting server");
    let serv = std::net::TcpListener::bind("127.0.0.1:7878").unwrap();
    for stream in serv.incoming() {
        let mut sock = match stream {
            Ok(s) => {
                println!("Got connection from ");
                s
            }
            Err(e) => {
                println!("Got Error {e}");
                return;
            }
        };
        thread::spawn(move || read_from_socket(&mut sock));
    }
}

fn read_from_socket(socket: &mut TcpStream) {
    let mut size: [u8; 4] = [0; 4];
    let mut res: [u8; 1024] = [0; 1024];
    loop {
        let received_length = socket.peek(&mut res).expect("SOCKET PEEK FAILED");
        if received_length == 0 {
            continue;
        }
        //Read Byte Length
        socket.read(&mut size).expect("COULDNT READ BYTE LENGTH");
        let read_size = i32::from_be_bytes(size);

        //Read Message ID
        socket.read(&mut size).expect("COULDNT READ MSG ID");
        let id = i32::from_be_bytes(size);
        let msg_id = match MessageID::from_i32(id) {
            Some(id) => id,
            None => {
                println!("Failed to convert message ID");
                continue;
            }
        };

        let to_read = &mut res[0..read_size as usize];

        match socket.read_exact(to_read) {
            Ok(_) => {}
            Err(_) => {
                println!("Failed to Read Exact Buffer");
                continue;
            }
        }
        handle_message(&msg_id, &to_read, socket);
    }
}
fn send(stream: &mut TcpStream, bytes: &[u8]) {
    let _res = stream.write(bytes).expect("Stream Write Failed");
}
fn send_failed_ack(stream: &mut TcpStream) {
    let res = i32::to_be_bytes(-1);
    send(stream, &res);
}
fn send_success_ack(stream: &mut TcpStream) {
    let res = i32::to_be_bytes(1);
    send(stream, &res);
}

pub static SEARCH_DEPTH: i8 = 7;
pub static LATE_SEARCH_DEPTH: i8 = 9;
const NUM_THREADS: usize = 4;

impl Position {
    pub fn from_grid(col: i32, row: i32) -> Position {
        let mut pos = Position::new();
        pos.row = row;
        pos.col = col;
        pos
    }

    pub fn out_of_bounds(col: i32, row: i32) -> bool {
        col > 7 || col < 0 || row > 7 || row < 0
    }

    pub fn move_to(&self, col_inc: i32, row_inc: i32) -> Position {
        Position::from_grid(self.col + col_inc, self.row + row_inc)
    }

    pub fn from_cindex(cindex: u8) -> Position {
        Position::from_grid(
            (cindex % 8) as i32,
            (7 - cindex / 8) as i32,
        )
    }
    pub fn to_cindex(&self) -> u8 {
        ((self.row - 7).abs() * 8 + self.col) as u8
    }
}

fn rc_to_cindex(row: i32, col: i32) -> u8 {
    ((row - 7).abs() * 8 + col) as u8
}

pub fn convert_to_proto_piece(piece: PieceType) -> generated::chess::PieceType {
    match piece {
        PieceType::P => generated::chess::PieceType::PAWN,
        PieceType::B => generated::chess::PieceType::BISHOP,
        PieceType::N => generated::chess::PieceType::KNIGHT,
        PieceType::R => generated::chess::PieceType::ROOK,
        PieceType::Q => generated::chess::PieceType::QUEEN,
        PieceType::K => generated::chess::PieceType::KING,
        _ => generated::chess::PieceType::NONE
    }
}

fn handle_message(id: &MessageID, bytes: &[u8], socket: &mut TcpStream) {
    match id {
        MessageID::FIND_BEST => {
            let mut cl = socket.try_clone().unwrap();
            let request_msg =
                FindBest::parse_from_bytes(bytes).expect("Could not parse FindBest message");

            println!("Got FEN String = {}", request_msg.fen_string);
            thread::spawn(move || {
                let start = Instant::now();
                let mut board = pleco::Board::from_fen(&request_msg.fen_string).expect("Board Fen Create Failed");


                let mv = find_best_move(&mut board, 9);
                // let mv = IterativeSearcher::best_move(board, 7);


                println!("Move = {mv}");

                let elapsed = start.elapsed();

                println!("Search Took {} ms", elapsed.as_millis());

                println!("mv Src = {}", mv.get_src_u8());
                println!("mv dest = {}", mv.get_dest_u8());
                let from = mv.get_src_u8();
                let to = mv.get_dest_u8();

                let mut response = FindBestResponse::new();
                // //Row position is 0 indexed at white row
                response.from_pos = MessageField::some(Position::from_cindex(from));
                response.end_pos =  MessageField::some(Position::from_cindex(to));

                if mv.is_promo() {
                    response.promoted_piece = EnumOrUnknown::new(convert_to_proto_piece(mv.promo_piece()));
                } else {
                    response.promoted_piece = EnumOrUnknown::new(generated::chess::PieceType::NONE);
                }

                send_proto_msg(&mut cl, &response, &MessageID::FIND_BEST_RESPONSE);
            });
        }
        MessageID::GET_VALID_MOVES => {
            let mut cl = socket.try_clone().unwrap();
            let request_msg = GetValidMoves::parse_from_bytes(bytes)
                .expect("Could not parse Get Valid Moves Msg from bytes");

            thread::spawn(move || {
                let board = pleco::Board::from_fen(&request_msg.fen_string).expect("Board Fen Create Failed");

                let mvs = board.generate_moves();
                let mut proto_moves = Vec::new();

                let piece = board.piece_at_sq(SQ(rc_to_cindex(request_msg.piece_to_move.row, request_msg.piece_to_move.col)));
                let sq = rc_to_cindex(request_msg.piece_to_move.row, request_msg.piece_to_move.col);

                let ksq = board.king_sq(board.turn());

                for mv in mvs {
                    let from = mv.get_src_u8();
                    if from == sq {

                        let mut to = mv.get_dest_u8();
                        //Manually adjust castle move
                        if from == ksq.0 {
                            if from == 60 && to == 63 {
                                to = 62;
                            } else if from == 60 && to == 56 {
                                to = 58;
                            } else if from == 4 && to == 0 {
                                to = 2;
                            } else if from == 4 && to == 7 {
                                to = 6;
                            }
                            println!("King Move = {from} -> {to}");
                        }
                        let pos = Position::from_cindex(to);
                        proto_moves.push(pos);
                    }
                }

                let mut response = ValidMovesResponse::new();
                response.moves = proto_moves;
                response.request_piece = request_msg.piece_to_move;

                send_proto_msg(&mut cl, &response, &MessageID::VALID_MOVES_RESPONSE);
            });
        }
        _ => {
            println!("Got Unhandled MSG ID");
        }
    }
}

pub fn send_proto_msg(socket: &mut TcpStream, msg: &impl Message, msg_id: &MessageID) {
    let write_result = msg.write_to_bytes();
    match write_result {
        Ok(bytes) => {
            let size_bytes = i32::to_be_bytes(bytes.len() as i32);
            send(socket, &size_bytes);
            send(socket, &msg_id.value().to_be_bytes());
            send(socket, &bytes);
        }
        Err(_) => {
            println!("Failed to send proto message!");
        }
    }
}


pub mod tester {
    fn add(a: i32, b: i32) {

    }
}
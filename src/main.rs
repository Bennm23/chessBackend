mod generated;
mod processing;

use generated::chess::*;
use generated::common::*;

use processing::new_handles::fix_castle_rights;
use processing::{new_handles, processor::*};

use protobuf::{Enum, Message, MessageField};
use std::str::FromStr;
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
                send_failed_ack(socket);
                continue;
            }
        };

        let to_read = &mut res[0..read_size as usize];

        match socket.read_exact(to_read) {
            Ok(_) => {}
            Err(_) => {
                println!("Failed to Read Exact Buffer");
                send_failed_ack(socket);
                continue;
            }
        }
        send_success_ack(socket);
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

fn handle_message(id: &MessageID, bytes: &[u8], socket: &mut TcpStream) {
    match id {
        MessageID::GET_BEST_MOVE => {
            let mut cl = socket.try_clone().unwrap();
            let request_msg =
                GetBestMove::parse_from_bytes(bytes).expect("Could not parse GetBestMove message");

            thread::spawn(move || {
                let board = request_msg.board.unwrap();

                // let depth = if board.turnCount > 60 { LATE_SEARCH_DEPTH } else { SEARCH_DEPTH };
                let res = board.find_best_move_chunks(SEARCH_DEPTH, &request_msg.player.unwrap());

                let mut response = BestMoveResponse::new();
                response.best_move = MessageField::some(res.unwrap());

                send_proto_msg(&mut cl, &response);
            });
        }
        MessageID::FIND_BEST => {
            let mut cl = socket.try_clone().unwrap();
            let request_msg =
                FindBest::parse_from_bytes(bytes).expect("Could not parse FindBest message");

            println!("Got FEN String = {}", request_msg.fen_string);
            thread::spawn(move || {
                let orig_board =
                    chess::Board::from_str(&request_msg.fen_string).expect("FEN STRING ERROR");

                let board = fix_castle_rights(&orig_board, request_msg.fen_string);

                println!("Start Board =\n{:?}", board);
                let res = new_handles::find_best_move_chunks(board, SEARCH_DEPTH);

                println!("Best Move = {}", res);

                let from = res.get_source().to_index();
                let to = res.get_dest().to_index();

                let mut response = FindBestResponse::new();
                //Row position is 0 indexed at white row
                response.from_pos = MessageField::some(Position::from_grid(
                    (from % 8) as i32,
                    (7 - from / 8) as i32,
                ));
                response.end_pos =
                    MessageField::some(Position::from_grid((to % 8) as i32, (7 - to / 8) as i32));

                send_proto_msg(&mut cl, &response);
            });
        }
        MessageID::GET_VALID_MOVES => {
            let mut cl = socket.try_clone().unwrap();
            let request_msg = GetValidMoves::parse_from_bytes(bytes)
                .expect("Could not parse Get Valid Moves Msg from bytes");

            thread::spawn(move || {
                let ret = get_valid_moves(&request_msg);
                let result = match ret {
                    Some(res) => res,
                    None => {
                        println!("Get Valid Moves Returned NONE");
                        Vec::new()
                    }
                };
                let mut response = ValidMovesResponse::new();
                response.moves = result;

                send_proto_msg(&mut cl, &response);
            });
        }
        _ => {
            println!("Got Unhandled MSG ID");
        }
    }
}

pub fn send_proto_msg(socket: &mut TcpStream, msg: &impl Message) {
    let write_result = msg.write_to_bytes();
    match write_result {
        Ok(bytes) => {
            let size_bytes = i32::to_be_bytes(bytes.len() as i32);
            send(socket, &size_bytes);
            send(socket, &bytes);
        }
        Err(_) => {
            send_failed_ack(socket);
        }
    }
}

use engine::searching::{eval_search, start_search, start_search_quiet};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ClientMessage {
    GetBestMove { fen: String },
    GetBoardEval { fen: String },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ServerMessage {
    BestMove { best_move: String },
    BoardEval { score: f64 },
    Error { message: String },
}
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::StreamExt;
use std::{net::SocketAddr, time::Instant};
use tower_http::cors::{Any, CorsLayer};

mod test_fens;

fn test_suite() {
    const GAMES: usize = 86;

    let mut new_is_white: bool = false;
    let start = Instant::now();

    let mut new_wins = 0f32;
    let mut old_wins = 0f32;
    let mut total_draws = 0;

    let mut important: Vec<String> = vec![];

    for game in 0..GAMES {
        // let mut board = pleco::Board::default();
        let mut board = pleco::Board::from_fen(
            test_fens::EARLY_BALANCED_FENS[game % test_fens::EARLY_BALANCED_FENS.len()]
        )
        .expect("Fen parse failed");

        println!("Game {}, New Playing as {}: Start FEN = {}", game + 1, if new_is_white { "White" } else { "Black" }, board.fen());
        important.push(format!("Game {}, New Playing as {}: Start FEN = {}", game + 1, if new_is_white { "White" } else { "Black" }, board.fen()));

        'gameloop: while !board.generate_moves().is_empty() {
            let white_to_move = board.turn() == pleco::Player::White;
            let mv = if (white_to_move && new_is_white) || (!white_to_move && !new_is_white) {
                // New engine to move
                engine::final_search::start_search_quiet(&mut board)
            } else {
                // Old engine to move
                engine::searching::start_search_quiet(&mut board)
            };
            if mv.is_null() {
                println!("Final Move null detected, ending game.");
                break 'gameloop;
            }
            board.apply_move(mv);
        }


        let white_to_move = board.turn() == pleco::Player::White;
        if board.checkmate() {
            let str;
            // If new player is to move
            if white_to_move == new_is_white {
                str = format!("Game {}: Old engine wins", game + 1);
                old_wins += 1.0;
            } else {
                str = format!("Game {}: New engine wins", game + 1);
                new_wins += 1.0;
            }

            println!("{}", str);
            important.push(str);
        } else {
            println!("Game {}: Draw by stalemate", game + 1);
            important.push(format!("Game {}: Draw by stalemate", game + 1));
            total_draws += 1;
        }
        println!("Game {}: End FEN = {}", game + 1, board.fen());
        important.push(format!("Game {}: End FEN = {}", game + 1, board.fen()));

        new_is_white = !new_is_white;
    }

    println!("========== Run Over Results ==========");
    for line in important {
        println!("{}", line);
    }
    println!("========= Final Scores =========");
    println!("Total Games: {}", GAMES);
    // println!("Average Game Length: {}", total_moves as f32 / GAMES as f32);
    println!("Old Wins: {}", old_wins);
    println!("New Wins: {}", new_wins);
    println!("Draws: {}", total_draws);
    let duration = start.elapsed();
    println!("Total Time: {} seconds", duration.as_secs());
}

async fn backend() {
    // Configure CORS to allow requests from Vite frontend
    let cors = CorsLayer::new()
        // .allow_origin(
        //     "http://localhost:5173"
        //         .parse::<axum::http::HeaderValue>()
        //         .unwrap(),
        // )
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new().route("/", get(ws_handler)).layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Server running at ws://{}", addr);
    axum::serve(listener, app).await.unwrap();
}

#[tokio::main]
async fn main() {
    test_suite();
    // backend().await;
}

// Blundered mate 1k1rr3/pp3p1Q/5q2/P7/4n1B1/1P1p3P/3P1PP1/1R3K1R w - - 2 25
// https://www.chess.com/analysis/game/computer/461571475/review?move=47&move=47&tab=review&classification=greatfind&autorun=true
async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.next().await {
        if let Message::Text(text) = msg {
            // Parse message
            match serde_json::from_str::<ClientMessage>(&text) {
                Ok(ClientMessage::GetBestMove { fen }) => {
                    println!("Received FEN: {}", fen);

                    let mut board = pleco::Board::from_fen(&fen).expect("Board Fen Create Failed");
                    let mv = engine::final_search::start_search(&mut board);

                    // Dummy best move logic
                    let best_move = ServerMessage::BestMove {
                        best_move: mv.to_string(),
                    };

                    let resp_text = serde_json::to_string(&best_move).unwrap();
                    if socket.send(Message::Text(resp_text.into())).await.is_err() {
                        break;
                    }
                }
                Ok(ClientMessage::GetBoardEval { fen }) => {
                    println!("Received FEN for eval: {}", fen);
                    let mut board = pleco::Board::from_fen(&fen).expect("Board Fen Create Failed");
                    let score = engine::final_search::eval_search(&mut board);
                    let eval = ServerMessage::BoardEval { score };

                    if socket.send(Message::Text(serde_json::to_string(&eval).unwrap().into())).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let err = ServerMessage::Error {
                        message: format!("Invalid message: {}", e),
                    };
                    let err_text = serde_json::to_string(&err).unwrap();
                    if socket.send(Message::Text(err_text.into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}
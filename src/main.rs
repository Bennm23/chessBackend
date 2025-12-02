use book::Book;
use pleco::BitMove;
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
use tokio::sync::RwLock;
use std::{env, net::SocketAddr, sync::LazyLock};
use tower_http::cors::{Any, CorsLayer};


static BOOK: std::sync::LazyLock<Book> = std::sync::LazyLock::new(
    || book::load_from_ron(
        &env::var("OPENING_BOOK").unwrap_or_else(|_| "/home/deploy/book.ron".to_string())
    )
);

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

    let app = Router::new().route("/ws", get(ws_handler)).layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Server running at ws://{}", addr);
    axum::serve(listener, app).await.unwrap();
}

mod test_ops;
#[tokio::main]
async fn main() {
    // test_ops::test_suite();
    backend().await;
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

                    let mut board = match pleco::Board::from_fen(&fen) {
                        Ok(b) => b,
                        Err(e) => {
                            let err = ServerMessage::Error {
                                message: format!("Invalid FEN: {:?}", e),
                            };
                            let err_text = serde_json::to_string(&err).unwrap();
                            if socket.send(Message::Text(err_text.into())).await.is_err() {
                                break;
                            }
                            continue;
                        }
                    };
                    
                    let mut book_opt: Option<BitMove> = None;

                    if board.moves_played() <= 10 {
                        println!("Checking book for move...");
                        // Check book first
                        if let Some(book_move) = book::get_book_move(&BOOK, &fen) {
                            println!("Book move found: {}", book_move);
                            let applied = board.apply_uci_move(&book_move);
                            if applied {
                                book_opt = board.last_move();
                                if book_opt.is_none() {
                                    println!("Book move was not valid: {}", book_move);
                                }
                            } else {
                                println!("Failed to apply book move: {}", book_move);
                            }
                        } else {
                            println!("No book move found.");
                        }
                    }
                    let mv = match book_opt {
                        Some(bm) => bm,
                        None => engine::search_test::start_search(&mut board),
                    };

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
                    let score = engine::search_test::eval_search(&mut board);
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
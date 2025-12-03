use book::Book;
use pleco::BitMove;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ClientMessage {
    GetBestMove { fen: String, move_history: Vec<String> },
    GetBoardEval { fen: String },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ServerMessage {
    BestMove { best_move: String},
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
use std::{env, net::SocketAddr};
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

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4) // 4 worker threads
        .thread_stack_size(3 * 1024 * 1024) // Set stack size to 3 MiB
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        println!("Tokio runtime with custom thread stack size is running!");
        backend().await;
    });
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

fn check_fens(fen1: &str, fen2: &str) -> bool {
    let fen1_split = fen1.split_whitespace().collect::<Vec<&str>>();
    let fen2_split = fen2.split_whitespace().collect::<Vec<&str>>();
    for i in 0 .. fen1_split.len() {
        // Skip EP square, pleco only sets ep if it can be captured
        if i == 3 {
            continue;
        }
        if fen1_split[i] != fen2_split[i] {
            return false;
        }
    }
    true
}

fn try_book_move(board: &mut pleco::Board, fen: &str) -> Option<BitMove> {
    let mut book_opt: Option<BitMove> = None;
    if board.moves_played() <= 10 {
        // Check book first
        if let Some(book_move) = book::get_book_move(&BOOK, &fen) {
            let applied = board.apply_uci_move(&book_move);
            if applied {
                book_opt = board.last_move();
                if book_opt.is_none() {
                    println!("Book move was not valid: {}", book_move);
                }
            } else {
                println!("Failed to apply book move: {}", book_move);
            }
        }
    }
    book_opt
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.next().await {
        if let Message::Text(text) = msg {
            // Parse message
            match serde_json::from_str::<ClientMessage>(&text) {
                Ok(ClientMessage::GetBestMove { fen, move_history }) => {

                    // Build board from move history to ensure repetitions are handled correctly
                    let mut board = pleco::Board::start_pos();
                    let mut moves_failed = false;
                    for mv in &move_history {
                        let applied = board.apply_uci_move(mv);
                        if !applied {
                            moves_failed = true;
                            println!("Failed to apply move from history: {}", mv);
                            break;
                        }
                    }

                    if !check_fens(&board.fen(), &fen) || moves_failed {
                        eprintln!("Received FEN: {}", fen);
                        eprintln!("Board after applying move history: {}", board.fen());
                        let err = ServerMessage::Error {
                            message: format!("FEN and Move History do not match"),
                        };
                        let err_text = serde_json::to_string(&err).unwrap();
                        if socket.send(Message::Text(err_text.into())).await.is_err() {
                            break;
                        }
                        continue;
                    }
                    
                    let book_opt: Option<BitMove> = try_book_move(&mut board, &fen);

                    let mv = match book_opt {
                        Some(bm) => bm,
                        None => engine::search::start_search(&mut board),
                    };

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
                    let score = engine::search::eval_search(&mut board);
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
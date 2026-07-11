mod server;
mod user_repository;

use crate::server::Server;
use crate::user_repository::UserRepository;
use sorcerers::{
    networking::{
        MAX_MESSAGE_SIZE,
        message::{ClientMessage, Message},
    },
    query::QueryCache,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::{io::AsyncReadExt, net::TcpListener, sync::Mutex};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    QueryCache::init();

    // Enable board-evaluation debug output with `--eval` or `SORCERERS_DEBUG_EVAL=1`.
    let test_state = std::env::args().any(|a| a == "--test-state")
        || std::env::var("SORCERERS_TEST_STATE").is_ok_and(|v| v == "1");
    if test_state {
        println!("Server test-state mode enabled – new games will include the seeded test board.");
    }

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set, for example postgres://user:password@localhost/sorcerers");
    let users = UserRepository::connect(&database_url).await?;

    let socket = TcpListener::bind("0.0.0.0:5000".parse::<SocketAddr>()?).await?;
    let server = Arc::new(Mutex::new(Server::new(test_state, users)));

    loop {
        let (stream, addr) = socket.accept().await?;
        let server_clone = Arc::clone(&server);
        tokio::spawn(async move {
            let (mut reader, writer) = stream.into_split();
            let writer = Arc::new(Mutex::new(writer));
            loop {
                let mut len: [u8; std::mem::size_of::<usize>()] = [0; std::mem::size_of::<usize>()];
                if reader.read_exact(&mut len).await.is_err() {
                    let mut server = server_clone.lock().await;
                    server
                        .process_message(
                            &Message::ClientMessage(ClientMessage::Disconnect),
                            Arc::clone(&writer),
                            &addr,
                        )
                        .await
                        .expect("message to be processed");
                    break;
                }

                let len = usize::from_be_bytes(len);
                if len > MAX_MESSAGE_SIZE {
                    eprintln!("closing connection from {addr}: message too large ({len} bytes)");
                    break;
                }

                let mut buf = vec![0; len];
                let read_bytes = match reader.read_exact(&mut buf).await {
                    Ok(read_bytes) => read_bytes,
                    Err(_) => {
                        let mut server = server_clone.lock().await;
                        server
                            .process_message(
                                &Message::ClientMessage(ClientMessage::Disconnect),
                                Arc::clone(&writer),
                                &addr,
                            )
                            .await
                            .expect("message to be processed");
                        break;
                    }
                };
                if read_bytes == 0 {
                    let mut server = server_clone.lock().await;
                    server
                        .process_message(
                            &Message::ClientMessage(ClientMessage::Disconnect),
                            Arc::clone(&writer),
                            &addr,
                        )
                        .await
                        .expect("message to be processed");
                    break;
                }

                let msg: Message = match rmp_serde::from_slice(&buf) {
                    Ok(msg) => msg,
                    Err(err) => {
                        eprintln!("closing connection from {addr}: invalid message: {err}");
                        break;
                    }
                };
                let mut server = server_clone.lock().await;
                if let Err(err) = server
                    .process_message(&msg, Arc::clone(&writer), &addr)
                    .await
                {
                    eprintln!("closing connection from {addr}: {err}");
                    break;
                }
            }
        });
    }
}

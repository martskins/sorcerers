mod server;

use crate::server::Server;
use sorcerers::{
    networking::message::{ClientMessage, Message},
    query::QueryCache,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::{io::AsyncReadExt, net::TcpListener, sync::Mutex};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    QueryCache::init();

    let socket = TcpListener::bind("0.0.0.0:5000".parse::<SocketAddr>()?).await?;
    let server = Arc::new(Mutex::new(Server::new()));
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

                let mut buf = vec![0; usize::from_be_bytes(len)];
                let read_bytes = reader.read_exact(&mut buf).await.expect("read from socket");
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

                let msg: Message = rmp_serde::from_slice(&buf).expect("valid message");
                let mut server = server_clone.lock().await;
                server
                    .process_message(&msg, Arc::clone(&writer), &addr)
                    .await
                    .expect("message to be processed");
            }
        });
    }
}

mod server;

use crate::server::Server;
use sorcerers::networking::message::Message;
use std::{net::SocketAddr, sync::Arc};
use tokio::{io::AsyncReadExt, net::TcpListener, sync::Mutex};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let socket = TcpListener::bind("0.0.0.0:8080".parse::<SocketAddr>().unwrap()).await?;
    let server = Arc::new(Mutex::new(Server::new()));
    loop {
        let (stream, _) = socket.accept().await?;
        let server_clone = Arc::clone(&server);
        tokio::spawn(async move {
            let (mut reader, writer) = stream.into_split();
            let writer = Arc::new(Mutex::new(writer));
            loop {
                let mut buf = vec![0; 32000];
                let msg: Message = loop {
                    if let Ok(n) = reader.read(&mut buf).await {
                        if n == 0 {
                            continue;
                        }

                        break rmp_serde::from_slice(&buf).unwrap();
                    }
                };

                let mut server = server_clone.lock().await;
                server.process_message(&msg, Arc::clone(&writer)).await.unwrap();
            }
        });
    }
}

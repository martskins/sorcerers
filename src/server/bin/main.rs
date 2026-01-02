mod server;

use crate::server::Server;
use sorcerers::{networking::message::Message, query::QueryCache};
use std::{net::SocketAddr, sync::Arc};
use tokio::{io::AsyncReadExt, net::TcpListener, sync::Mutex};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    QueryCache::init();

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
                let read_bytes = reader.read(&mut buf).await.unwrap();
                if read_bytes == 0 {
                    // TODO: Hnndle disconnection properly
                    break;
                }

                let msg: Message = rmp_serde::from_slice(&buf).unwrap();
                let mut server = server_clone.lock().await;
                server.process_message(&msg, Arc::clone(&writer)).await.unwrap();
            }
        });
    }
}

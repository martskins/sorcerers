mod server;

use crate::server::Server;
use std::net::SocketAddr;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let sock = UdpSocket::bind("0.0.0.0:8080".parse::<SocketAddr>().unwrap()).await?;
    let mut server = Server::new(sock);
    let mut buf = [0; 1024];
    loop {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
        tokio::select! {
            res = server.socket.recv_from(&mut buf) => {
                let (len, addr) = res.unwrap();
                server.process_message(&buf[..len], addr).await.unwrap();
                server.process_effects().await.unwrap();
            }
            _ = interval.tick() => {
                server.process_effects().await.unwrap();
            }
        }

        // let (_len, addr) = server.socket.recv_from(&mut buf).await?;
        // server.process_message(&buf, addr).await?;
        // server.process_effects()?;
    }
}

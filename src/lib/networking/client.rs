use crate::networking::Message;
use std::net::UdpSocket;

#[derive(Debug)]
pub struct Client {
    socket: std::net::UdpSocket,
}

impl Clone for Client {
    fn clone(&self) -> Self {
        Client {
            socket: self.socket.try_clone().unwrap(),
        }
    }
}

impl Client {
    pub fn new(addr: &str) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind("127.0.0.1:0")?;
        socket.connect(addr)?;
        Ok(Client { socket })
    }

    pub fn send(&self, message: Message) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec(&message)?;
        self.socket.send(&bytes)?;
        Ok(())
    }

    pub fn recv(&self) -> anyhow::Result<Message> {
        let mut res = [0; 32000];
        let _ = self.socket.recv(&mut res).unwrap();
        let response: Message = rmp_serde::from_slice(&res).unwrap();
        Ok(response)
    }
}

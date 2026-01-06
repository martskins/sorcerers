use crate::networking::message::{Message, ToMessage};
use std::net::TcpStream;
use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub enum Socket {
    SocketAddr(std::net::SocketAddr),
    Noop,
}

#[derive(Debug)]
pub struct Client {
    local_mode: bool,
    reader: Arc<Mutex<TcpStream>>,
    writer: Arc<Mutex<TcpStream>>,
}

impl Clone for Client {
    fn clone(&self) -> Self {
        Client {
            local_mode: self.local_mode.clone(),
            reader: Arc::clone(&self.reader),
            writer: Arc::clone(&self.writer),
        }
    }
}

impl Client {
    pub fn connect(addr: &str) -> anyhow::Result<Self> {
        let stream = std::net::TcpStream::connect(addr)?;
        Ok(Client {
            local_mode: addr == "127.0.0.1:5000",
            reader: Arc::new(Mutex::new(stream.try_clone()?)),
            writer: Arc::new(Mutex::new(stream)),
        })
    }

    pub fn is_in_local_mode(&self) -> bool {
        self.local_mode
    }

    pub fn send<T: ToMessage>(&self, message: T) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec(&message.to_message())?;
        let mut stream = self
            .writer
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock writer: {}", e))?;
        stream.write_all(&bytes)?;
        Ok(())
    }

    pub fn recv(&self) -> anyhow::Result<Option<Message>> {
        let mut res = [0; 32000];
        let mut stream = self
            .reader
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock reader: {}", e))?;
        let _ = stream.read(&mut res)?;
        let response: Message = rmp_serde::from_slice(&res)?;
        Ok(Some(response))
    }
}

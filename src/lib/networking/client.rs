use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;

use crate::networking::{
    MAX_MESSAGE_SIZE,
    message::{Message, ServerMessage, ToMessage},
};
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
    reader: Arc<Mutex<TcpStream>>,
    writer: Arc<Mutex<TcpStream>>,
}

impl Clone for Client {
    fn clone(&self) -> Self {
        Client {
            reader: Arc::clone(&self.reader),
            writer: Arc::clone(&self.writer),
        }
    }
}

impl Client {
    pub fn connect(addr: &str) -> anyhow::Result<Self> {
        let stream = std::net::TcpStream::connect(addr)?;
        Ok(Client {
            reader: Arc::new(Mutex::new(stream.try_clone()?)),
            writer: Arc::new(Mutex::new(stream)),
        })
    }

    pub fn send<T: ToMessage>(&self, message: T) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec(&message.to_message())?;
        let len = bytes.len();
        if len > MAX_MESSAGE_SIZE {
            return Err(anyhow::anyhow!("message too large: {} bytes", len));
        }
        let mut stream = self
            .writer
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock writer: {}", e))?;
        stream.write_all(&len.to_be_bytes())?;
        stream.write_all(&bytes)?;
        Ok(())
    }

    pub fn recv(&self) -> anyhow::Result<Option<Message>> {
        let mut stream = self
            .reader
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock reader: {}", e))?;
        let mut len: [u8; std::mem::size_of::<usize>()] = [0; std::mem::size_of::<usize>()];
        stream.read_exact(&mut len)?;

        let len = usize::from_be_bytes(len);
        if len > MAX_MESSAGE_SIZE {
            return Err(anyhow::anyhow!("message too large: {} bytes", len));
        }

        let mut res = vec![0; len];
        stream.read_exact(&mut res)?;
        let response: Message = rmp_serde::from_slice(&res)?;
        Ok(Some(response))
    }

    pub async fn send_to_stream(
        message: &ServerMessage,
        stream: Arc<tokio::sync::Mutex<OwnedWriteHalf>>,
    ) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec(&message.to_message())?;
        let len = bytes.len();
        if len > MAX_MESSAGE_SIZE {
            return Err(anyhow::anyhow!("message too large: {} bytes", len));
        }
        let mut stream = stream.lock().await;
        stream.write_all(&len.to_be_bytes()).await?;
        stream.write_all(&bytes).await?;

        Ok(())
    }
}

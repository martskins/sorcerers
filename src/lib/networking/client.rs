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
        let stream = std::net::TcpStream::connect(addr).unwrap();
        Ok(Client {
            reader: Arc::new(Mutex::new(stream.try_clone().unwrap())),
            writer: Arc::new(Mutex::new(stream)),
        })
    }

    pub fn send<T: ToMessage>(&self, message: T) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec(&message.to_message()).unwrap();
        let mut stream = self.writer.lock().unwrap();
        stream.write_all(&bytes).unwrap();
        Ok(())
    }

    pub fn recv(&self) -> anyhow::Result<Option<Message>> {
        let mut res = [0; 32000];
        let mut stream = self.reader.lock().unwrap();
        let _ = stream.read(&mut res)?;
        let response: Message = rmp_serde::from_slice(&res).unwrap();
        Ok(Some(response))
    }
}

use crate::networking::message::Message;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio_serde::formats::Bincode;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

pub mod client;
pub mod message;

pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

pub type FramedReader = tokio_serde::Framed<
    FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
    Message,
    Message,
    Bincode<Message, Message>,
>;
pub type FramedWriter = tokio_serde::Framed<
    FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
    Message,
    Message,
    Bincode<Message, Message>,
>;

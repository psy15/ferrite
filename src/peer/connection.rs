use super::handshake::Handshake;
use super::message::Message;
use crate::{FeriteError, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct PeerConnection {
    stream: TcpStream,
    pub peer_id: [u8; 20],
}

impl PeerConnection {
    pub async fn connect(addr: &str, info_hash: [u8; 20], our_peer_id: [u8; 20]) -> Result<Self> {
        let mut stream = TcpStream::connect(addr)
            .await
            .map_err(|e| FeriteError::Peer(e.to_string()))?;

        // send handshake
        let handshake = Handshake::new(info_hash, our_peer_id);
        stream
            .write_all(&handshake.to_bytes())
            .await
            .map_err(|e| FeriteError::Peer(e.to_string()))?;

        // receive handshake
        let mut buf = [0u8; 68];
        stream
            .read_exact(&mut buf)
            .await
            .map_err(|e| FeriteError::Peer(e.to_string()))?;

        let peer_handshake = Handshake::from_bytes(&buf)?;

        // verify same torrent
        if peer_handshake.info_hash != info_hash {
            return Err(FeriteError::Peer("info_hash mismatch".to_string()));
        }

        Ok(Self {
            stream,
            peer_id: peer_handshake.peer_id,
        })
    }

    pub async fn send_message(&mut self, msg: Message) -> Result<()> {
        let payload = msg.to_bytes();
        let length = payload.len() as u32;
        self.stream
            .write_all(&length.to_be_bytes())
            .await
            .map_err(|e| FeriteError::Peer(e.to_string()))?;
        self.stream
            .write_all(&payload)
            .await
            .map_err(|e| FeriteError::Peer(e.to_string()))?;
        Ok(())
    }

    pub async fn read_message(&mut self) -> Result<Message> {
        let mut len_buf = [0u8; 4];
        self.stream
            .read_exact(&mut len_buf)
            .await
            .map_err(|e| FeriteError::Peer(e.to_string()))?;
        let length = u32::from_be_bytes(len_buf);

        let mut payload = vec![0u8; length as usize];
        self.stream
            .read_exact(&mut payload)
            .await
            .map_err(|e| FeriteError::Peer(e.to_string()))?;

        Message::from_bytes(&payload)
    }
}

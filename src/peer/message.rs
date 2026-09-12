use crate::{FeriteError, Result};

#[derive(Debug)]
pub enum Message {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u32),
    Bitfield(Vec<u8>),
    Request {
        index: u32,
        begin: u32,
        length: u32,
    },
    Piece {
        index: u32,
        begin: u32,
        data: Vec<u8>,
    },
    Cancel {
        index: u32,
        begin: u32,
        length: u32,
    },
}

impl Message {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() {
            return Err(FeriteError::Peer("empty message".to_string()));
        }
        match bytes[0] {
            0 => Ok(Message::Choke),
            1 => Ok(Message::Unchoke),
            2 => Ok(Message::Interested),
            3 => Ok(Message::NotInterested),
            4 => {
                let index = u32::from_be_bytes(
                    bytes[1..5]
                        .try_into()
                        .map_err(|_| FeriteError::Peer("invalid have index".to_string()))?,
                );
                Ok(Message::Have(index))
            }
            5 => Ok(Message::Bitfield(bytes[1..].to_vec())),
            6 => {
                let index = u32::from_be_bytes(
                    bytes[1..5]
                        .try_into()
                        .map_err(|_| FeriteError::Peer("invalid request index".to_string()))?,
                );
                let begin = u32::from_be_bytes(
                    bytes[5..9]
                        .try_into()
                        .map_err(|_| FeriteError::Peer("invalid request begin".to_string()))?,
                );
                let length = u32::from_be_bytes(
                    bytes[9..13]
                        .try_into()
                        .map_err(|_| FeriteError::Peer("invalid request length".to_string()))?,
                );
                Ok(Message::Request {
                    index,
                    begin,
                    length,
                })
            }
            7 => {
                let index = u32::from_be_bytes(
                    bytes[1..5]
                        .try_into()
                        .map_err(|_| FeriteError::Peer("invalid piece index".to_string()))?,
                );
                let begin = u32::from_be_bytes(
                    bytes[5..9]
                        .try_into()
                        .map_err(|_| FeriteError::Peer("invalid piece begin".to_string()))?,
                );
                let data = bytes[9..].to_vec();
                Ok(Message::Piece { index, begin, data })
            }
            8 => {
                let index = u32::from_be_bytes(
                    bytes[1..5]
                        .try_into()
                        .map_err(|_| FeriteError::Peer("invalid cancel index".to_string()))?,
                );
                let begin = u32::from_be_bytes(
                    bytes[5..9]
                        .try_into()
                        .map_err(|_| FeriteError::Peer("invalid cancel begin".to_string()))?,
                );
                let length = u32::from_be_bytes(
                    bytes[9..13]
                        .try_into()
                        .map_err(|_| FeriteError::Peer("invalid cancel length".to_string()))?,
                );
                Ok(Message::Cancel {
                    index,
                    begin,
                    length,
                })
            }
            id => Err(FeriteError::Peer(format!("unknown message id: {}", id))),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Message::Choke => vec![0],
            Message::Unchoke => vec![1],
            Message::Interested => vec![2],
            Message::NotInterested => vec![3],
            Message::Have(index) => {
                let mut b = vec![4];
                b.extend_from_slice(&index.to_be_bytes());
                b
            }
            Message::Request {
                index,
                begin,
                length,
            } => {
                let mut b = vec![6];
                b.extend_from_slice(&index.to_be_bytes());
                b.extend_from_slice(&begin.to_be_bytes());
                b.extend_from_slice(&length.to_be_bytes());
                b
            }
            Message::Piece { index, begin, data } => {
                let mut b = vec![7];
                b.extend_from_slice(&index.to_be_bytes());
                b.extend_from_slice(&begin.to_be_bytes());
                b.extend_from_slice(data);
                b
            }
            _ => vec![],
        }
    }
}

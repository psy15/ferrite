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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choke() {
        let bytes = vec![0];
        assert!(matches!(
            Message::from_bytes(&bytes).unwrap(),
            Message::Choke
        ));
    }

    #[test]
    fn test_unchoke() {
        let bytes = vec![1];
        assert!(matches!(
            Message::from_bytes(&bytes).unwrap(),
            Message::Unchoke
        ));
    }

    #[test]
    fn test_interested() {
        let bytes = vec![2];
        assert!(matches!(
            Message::from_bytes(&bytes).unwrap(),
            Message::Interested
        ));
    }

    #[test]
    fn test_have() {
        let mut bytes = vec![4];
        bytes.extend_from_slice(&42u32.to_be_bytes());
        assert!(matches!(
            Message::from_bytes(&bytes).unwrap(),
            Message::Have(42)
        ));
    }

    #[test]
    fn test_bitfield() {
        let bytes = vec![5, 0xFF, 0xAB, 0x12];
        match Message::from_bytes(&bytes).unwrap() {
            Message::Bitfield(b) => assert_eq!(b, vec![0xFF, 0xAB, 0x12]),
            _ => panic!("wrong type"),
        }
    }

    #[test]
    fn test_request() {
        let mut bytes = vec![6];
        bytes.extend_from_slice(&1u32.to_be_bytes()); // index
        bytes.extend_from_slice(&0u32.to_be_bytes()); // begin
        bytes.extend_from_slice(&16384u32.to_be_bytes()); // length
        match Message::from_bytes(&bytes).unwrap() {
            Message::Request {
                index,
                begin,
                length,
            } => {
                assert_eq!(index, 1);
                assert_eq!(begin, 0);
                assert_eq!(length, 16384);
            }
            _ => panic!("wrong type"),
        }
    }

    #[test]
    fn test_piece() {
        let mut bytes = vec![7];
        bytes.extend_from_slice(&0u32.to_be_bytes()); // index
        bytes.extend_from_slice(&0u32.to_be_bytes()); // begin
        bytes.extend_from_slice(&[1, 2, 3, 4]); // data
        match Message::from_bytes(&bytes).unwrap() {
            Message::Piece { index, begin, data } => {
                assert_eq!(index, 0);
                assert_eq!(begin, 0);
                assert_eq!(data, vec![1, 2, 3, 4]);
            }
            _ => panic!("wrong type"),
        }
    }

    #[test]
    fn test_empty_message_errors() {
        let result = Message::from_bytes(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_message_id_errors() {
        let result = Message::from_bytes(&[99]);
        assert!(result.is_err());
    }

    #[test]
    fn test_to_bytes_interested() {
        let bytes = Message::Interested.to_bytes();
        assert_eq!(bytes, vec![2]);
    }

    #[test]
    fn test_to_bytes_request() {
        let bytes = Message::Request {
            index: 0,
            begin: 0,
            length: 16384,
        }
        .to_bytes();
        assert_eq!(bytes[0], 6);
        assert_eq!(bytes.len(), 13);
    }
}

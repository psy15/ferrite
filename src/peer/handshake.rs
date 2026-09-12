use crate::{FeriteError, Result};

const PROTOCOL: &[u8] = b"BitTorrent protocol";

#[derive(Debug)]
pub struct Handshake {
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
}

impl Handshake {
    pub fn new(info_hash: [u8; 20], peer_id: [u8; 20]) -> Self {
        Self { info_hash, peer_id }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(68);
        bytes.push(19);
        bytes.extend_from_slice(PROTOCOL);
        bytes.extend_from_slice(&[0u8; 8]);
        bytes.extend_from_slice(&self.info_hash);
        bytes.extend_from_slice(&self.peer_id);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 68 {
            return Err(FeriteError::Peer("handshake too short".to_string()));
        }
        if bytes[0] != 19 {
            return Err(FeriteError::Peer("invalid protocol length".to_string()));
        }
        if &bytes[1..20] != PROTOCOL {
            return Err(FeriteError::Peer("invalid protocol string".to_string()));
        }
        let info_hash = bytes[28..48]
            .try_into()
            .map_err(|_| FeriteError::Peer("invalid info_hash".to_string()))?;
        let peer_id = bytes[48..68]
            .try_into()
            .map_err(|_| FeriteError::Peer("invalid peer_id".to_string()))?;
        Ok(Self { info_hash, peer_id })
    }
}

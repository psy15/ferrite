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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handshake_to_bytes_length() {
        let info_hash = [1u8; 20];
        let peer_id = [2u8; 20];
        let h = Handshake::new(info_hash, peer_id);
        assert_eq!(h.to_bytes().len(), 68);
    }

    #[test]
    fn test_handshake_roundtrip() {
        let info_hash = [1u8; 20];
        let peer_id = [2u8; 20];
        let h = Handshake::new(info_hash, peer_id);
        let bytes = h.to_bytes();
        let parsed = Handshake::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.info_hash, info_hash);
        assert_eq!(parsed.peer_id, peer_id);
    }

    #[test]
    fn test_handshake_wrong_protocol() {
        let mut bytes = [0u8; 68];
        bytes[0] = 19;
        // wrong protocol string, all zeros
        let result = Handshake::from_bytes(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_handshake_too_short() {
        let bytes = [0u8; 10];
        let result = Handshake::from_bytes(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_handshake_starts_with_protocol_length() {
        let info_hash = [0u8; 20];
        let peer_id = [0u8; 20];
        let h = Handshake::new(info_hash, peer_id);
        let bytes = h.to_bytes();
        assert_eq!(bytes[0], 19);
        assert_eq!(&bytes[1..20], b"BitTorrent protocol");
    }
}

use crate::{
    bencode::decoder::{BencodeValue, decode},
    torrent::types::TorrentFile,
};

#[derive(Debug, Clone)]
pub struct Peer {
    pub ip: String,
    pub port: u16,
}

fn url_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("%{:02x}", b)).collect()
}

fn parse_peers(response: &[u8]) -> Vec<Peer> {
    let decoded = decode(response).unwrap();
    let mut peers = vec![];

    if let BencodeValue::Dict(pairs) = decoded {
        for (key, value) in &pairs {
            if key == b"peers" {
                match value {
                    // compact format - 6 bytes per peer
                    BencodeValue::Bytes(peer_bytes) => {
                        for chunk in peer_bytes.chunks(6) {
                            let ip = format!("{}.{}.{}.{}", chunk[0], chunk[1], chunk[2], chunk[3]);
                            let port = u16::from_be_bytes([chunk[4], chunk[5]]);
                            peers.push(Peer { ip, port });
                        }
                    }
                    // dictionary format - list of dicts
                    BencodeValue::List(peer_list) => {
                        for peer in peer_list {
                            if let BencodeValue::Dict(peer_pairs) = peer {
                                let mut ip = None;
                                let mut port = None;
                                for (k, v) in peer_pairs {
                                    match k.as_slice() {
                                        b"ip" => {
                                            if let BencodeValue::Bytes(b) = v {
                                                ip = Some(String::from_utf8_lossy(b).to_string());
                                            }
                                        }
                                        b"port" => {
                                            if let BencodeValue::Integer(n) = v {
                                                port = Some(*n as u16);
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                if let (Some(ip), Some(port)) = (ip, port) {
                                    peers.push(Peer { ip, port });
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    peers
}

/// Announces to the tracker and returns a list of peers.
pub fn announce(torrent: &TorrentFile) -> Vec<Peer> {
    let info_hash = torrent.info_hash;
    // TODO: generate random peer_id at startup
    let peer_id: [u8; 20] = *b"-FE0001-XXXXXXXXXXXX";
    let port: u16 = std::env::var("FERRITE_PORT")
        .unwrap_or("6881".to_string())
        .parse()
        .unwrap_or(6881);
    let uploaded = 0;
    let downloaded = 0;
    let left = torrent.length;
    let event = "started";

    let url = format!(
        "{}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&event={}",
        torrent.announce,
        url_encode(&info_hash),
        url_encode(&peer_id),
        port,
        uploaded,
        downloaded,
        left,
        event
    );

    let response = reqwest::blocking::get(&url).unwrap().bytes().unwrap();
    parse_peers(&response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_peers_dict_format() {
        let response = b"d5:peersld2:ip9:127.0.0.14:porti6881eeee";
        let peers = parse_peers(response);
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].ip, "127.0.0.1");
        assert_eq!(peers[0].port, 6881);
    }

    #[test]
    fn test_parse_peers_compact_format() {
        // 127.0.0.1:6881 in compact format
        // 127=0x7f, 0=0x00, 0=0x00, 1=0x01, 6881=0x1AE1
        let response = b"d5:peers6:\x7f\x00\x00\x01\x1a\xe1e";
        let peers = parse_peers(response);
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].ip, "127.0.0.1");
        assert_eq!(peers[0].port, 6881);
    }
}

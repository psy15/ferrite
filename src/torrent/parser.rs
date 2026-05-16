use crate::bencode::decoder::BencodeValue;
use crate::torrent::types::TorrentFile;
use sha1::{Digest, Sha1};

fn extract_string(value: &BencodeValue) -> String {
    match value {
        BencodeValue::Bytes(b) => String::from_utf8(b.clone()).unwrap(),
        _ => panic!("expected string"),
    }
}

fn extract_u64(value: &BencodeValue) -> u64 {
    match value {
        BencodeValue::Integer(n) => *n as u64,
        _ => panic!("expected integer"),
    }
}

/// Parses a torrent file from raw bytes and decoded bencode.
/// The raw bytes are needed to compute info_hash via SHA1
/// on the exact bytes of the info dict.
pub fn parse(raw: &[u8], bencode: BencodeValue) -> TorrentFile {
    let pairs = match bencode {
        BencodeValue::Dict(pairs) => pairs,
        _ => panic!("expected dict"),
    };

    let mut announce = None;
    let mut name = None;
    let mut length = None;
    let mut piece_length = None;
    let mut announce_list = None;
    let mut pieces: Option<&BencodeValue> = None;
    let mut info_hash: Option<[u8; 20]> = None;

    for (key, value) in &pairs {
        match key.as_slice() {
            b"announce" => announce = Some(value),
            b"announce-list" => {
                let mut result: Vec<Vec<String>> = vec![];
                if let BencodeValue::List(outer) = value {
                    // outer is Vec<BencodeValue>
                    // each item inside is BencodeValue::List (inner list)
                    // each item inside that is BencodeValue::Bytes (the URL)
                    for item in outer {
                        let mut inner_vec: Vec<String> = vec![];
                        if let BencodeValue::List(inner) = item {
                            for url in inner {
                                if let BencodeValue::Bytes(b) = url
                                    && let Ok(s) = std::str::from_utf8(b)
                                {
                                    inner_vec.push(s.to_string());
                                }
                            }
                        }
                        result.push(inner_vec);
                    }
                    announce_list = Some(result);
                }
            }
            b"info" => {
                // find the "4:info" which has length 6
                // windows(6) slides a 6-byte window across the input and checks each one
                let info_start = raw.windows(6).position(|w| w == b"4:info").unwrap();
                let info_dict_start = info_start + 6;

                let mut depth = 0;
                let mut info_dict_end = info_dict_start;
                let mut i = 0;

                while i < raw[info_dict_start..].len() {
                    let byte = raw[info_dict_start + i];
                    match byte {
                        b'd' | b'l' => {
                            depth += 1;
                            i += 1;
                        }
                        b'e' => {
                            depth -= 1;
                            if depth == 0 {
                                info_dict_end = info_dict_start + i + 1;
                                break;
                            }
                            i += 1;
                        }
                        b'i' => {
                            // skip integer — find closing e
                            let end = raw[info_dict_start + i..]
                                .iter()
                                .position(|&b| b == b'e')
                                .unwrap();
                            i += end + 1;
                        }
                        b'0'..=b'9' => {
                            // skip string — read length, skip content
                            let colon = raw[info_dict_start + i..]
                                .iter()
                                .position(|&b| b == b':')
                                .unwrap();
                            let string_length: usize = std::str::from_utf8(
                                &raw[info_dict_start + i..info_dict_start + i + colon],
                            )
                            .unwrap()
                            .parse()
                            .unwrap();
                            i += colon + 1 + string_length;
                        }
                        _ => i += 1,
                    }
                }

                let info_bytes = &raw[info_dict_start..info_dict_end];
                // this updates the outer variable
                info_hash = Some(Sha1::digest(info_bytes).into());
                // loop through info dict
                if let BencodeValue::Dict(info_pairs) = value {
                    for (info_key, info_value) in info_pairs {
                        match info_key.as_slice() {
                            b"name" => name = Some(info_value),
                            b"length" => length = Some(info_value),
                            b"piece length" => piece_length = Some(info_value),
                            b"pieces" => pieces = Some(info_value),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    TorrentFile {
        announce: extract_string(announce.unwrap()),
        announce_list,
        name: extract_string(name.unwrap()),
        length: extract_u64(length.unwrap()),
        piece_length: extract_u64(piece_length.unwrap()),
        pieces: match pieces.unwrap() {
            BencodeValue::Bytes(b) => b
                .chunks(20)
                .map(|chunk| chunk.try_into().unwrap())
                .collect(),
            _ => panic!("pieces is not bytes"),
        },
        info_hash: info_hash.unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bencode::decoder::decode;

    #[test]
    fn test_parse_ubuntu_torrent() {
        let raw = std::fs::read("tests/fixtures/ubuntu.torrent").unwrap();
        let bencode = decode(&raw);
        let torrent = parse(&raw, bencode);

        assert_eq!(torrent.announce, "https://torrent.ubuntu.com/announce");
        assert_eq!(torrent.name, "ubuntu-22.04.5-desktop-amd64.iso");
        assert_eq!(torrent.length, 4762707968);
        assert_eq!(torrent.piece_length, 262144);
        assert_eq!(torrent.pieces.len(), 18169); // 4762707968 / 262144 rounded up
        assert_eq!(torrent.info_hash.len(), 20);

        assert!(torrent.announce_list.is_some());
        let list = torrent.announce_list.unwrap();
        assert!(!list.is_empty());
        assert!(list[0].contains(&"https://torrent.ubuntu.com/announce".to_string()));
    }
}

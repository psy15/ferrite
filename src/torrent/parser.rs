use crate::torrent::types::{FileInfo, TorrentFile};
use crate::{FeriteError, bencode::decoder::BencodeValue};
use sha1::{Digest, Sha1};

fn extract_string(value: &BencodeValue) -> crate::Result<String> {
    match value {
        BencodeValue::Bytes(b) => {
            Ok(String::from_utf8(b.clone())
                .map_err(|e| FeriteError::TorrentParse(e.to_string()))?)
        }
        _ => Err(FeriteError::TorrentParse(
            "expected string, got different type".to_string(),
        )),
    }
}

fn extract_u64(value: &BencodeValue) -> crate::Result<u64> {
    match value {
        BencodeValue::Integer(n) => Ok(*n as u64),
        _ => Err(FeriteError::TorrentParse(
            "expected integer, got different type".to_string(),
        )),
    }
}

/// Parses a torrent file from raw bytes and decoded bencode.
/// The raw bytes are needed to compute info_hash via SHA1
/// on the exact bytes of the info dict.
pub fn parse(raw: &[u8], bencode: BencodeValue) -> crate::Result<TorrentFile> {
    let pairs = match bencode {
        BencodeValue::Dict(pairs) => pairs,
        _ => return Err(FeriteError::TorrentParse("expected dict".to_string())),
    };

    let mut announce = None;
    let mut name = None;
    let mut length = None;
    let mut piece_length = None;
    let mut announce_list = None;
    let mut pieces: Option<&BencodeValue> = None;
    let mut info_hash: Option<[u8; 20]> = None;
    let mut files: Option<&BencodeValue> = None;

    for (key, value) in &pairs {
        match key.as_slice() {
            b"announce" => announce = Some(value),
            b"announce-list" => {
                let mut result: Vec<Vec<String>> = vec![];
                if let BencodeValue::List(outer) = value {
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
                // compute info_hash from raw bytes
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
                            let end = raw[info_dict_start + i..]
                                .iter()
                                .position(|&b| b == b'e')
                                .unwrap();
                            i += end + 1;
                        }
                        b'0'..=b'9' => {
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
                info_hash = Some(Sha1::digest(info_bytes).into());

                // extract fields from decoded info dict
                if let BencodeValue::Dict(info_pairs) = value {
                    for (info_key, info_value) in info_pairs {
                        match info_key.as_slice() {
                            b"name" => name = Some(info_value),
                            b"length" => length = Some(info_value),
                            b"piece length" => piece_length = Some(info_value),
                            b"pieces" => pieces = Some(info_value),
                            b"files" => files = Some(info_value),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // handle single-file vs multi-file
    let (parsed_files, total_length) = if let Some(BencodeValue::List(file_list)) = files {
        let mut parsed = vec![];
        let mut total = 0u64;
        for f in file_list {
            if let BencodeValue::Dict(pairs) = f {
                let mut file_length = 0u64;
                let mut file_path = String::new();
                for (k, v) in pairs {
                    match k.as_slice() {
                        b"length" => {
                            if let BencodeValue::Integer(n) = v {
                                file_length = *n as u64;
                            }
                        }
                        b"path" => {
                            if let BencodeValue::List(parts) = v {
                                let parts: Vec<String> = parts
                                    .iter()
                                    .filter_map(|p| {
                                        if let BencodeValue::Bytes(b) = p {
                                            String::from_utf8(b.clone()).ok()
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                                file_path = parts.join("/");
                            }
                        }
                        _ => {}
                    }
                }
                total += file_length;
                parsed.push(FileInfo {
                    path: file_path,
                    length: file_length,
                });
            }
        }
        (Some(parsed), total)
    } else {
        let l =
            extract_u64(length.ok_or(FeriteError::TorrentParse("missing length".to_string()))?)?;
        (None, l)
    };

    Ok(TorrentFile {
        announce: extract_string(
            announce.ok_or(FeriteError::TorrentParse("missing announce".to_string()))?,
        )?,
        announce_list,
        name: extract_string(name.ok_or(FeriteError::TorrentParse("missing name".to_string()))?)?,
        length: total_length,
        piece_length: extract_u64(piece_length.ok_or(FeriteError::TorrentParse(
            "missing piece length".to_string(),
        ))?)?,
        pieces: match pieces.ok_or(FeriteError::TorrentParse("missing pieces".to_string()))? {
            BencodeValue::Bytes(b) => b
                .chunks(20)
                .map(|chunk| chunk.try_into().expect("piece must be 20 bytes"))
                .collect(),
            _ => return Err(FeriteError::TorrentParse("pieces is not bytes".to_string())),
        },
        info_hash: info_hash.ok_or(FeriteError::TorrentParse("missing info_hash".to_string()))?,
        files: parsed_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bencode::decoder::decode;

    #[test]
    fn test_parse_ubuntu_torrent() {
        let raw = std::fs::read("tests/fixtures/ubuntu.torrent").unwrap();
        let bencode = decode(&raw).unwrap();
        let torrent = parse(&raw, bencode).unwrap();

        assert_eq!(torrent.announce, "https://torrent.ubuntu.com/announce");
        assert_eq!(torrent.name, "ubuntu-22.04.5-desktop-amd64.iso");
        assert_eq!(torrent.length, 4762707968);
        assert_eq!(torrent.piece_length, 262144);
        assert_eq!(torrent.pieces.len(), 18169);
        assert_eq!(torrent.info_hash.len(), 20);

        assert!(torrent.announce_list.is_some());
        let list = torrent.announce_list.unwrap();
        assert!(!list.is_empty());
        assert!(list[0].contains(&"https://torrent.ubuntu.com/announce".to_string()));
    }

    #[test]
    fn test_parse_multifile_torrent() {
        let raw = std::fs::read("tests/fixtures/bigbuckbunny.torrent").unwrap();
        let bencode = decode(&raw).unwrap();
        let torrent = parse(&raw, bencode).unwrap();

        assert_eq!(torrent.name, "Big Buck Bunny");
        assert!(torrent.files.is_some());
        let files = torrent.files.unwrap();
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].path, "Big Buck Bunny.en.srt");
        assert_eq!(files[0].length, 140);
        assert_eq!(files[1].path, "Big Buck Bunny.mp4");
        assert_eq!(files[1].length, 276134947);
        assert_eq!(files[2].path, "poster.jpg");
        assert_eq!(torrent.length, 276445467);
    }
}

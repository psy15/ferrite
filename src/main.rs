use ferrite_core::bencode::decoder::decode;
use ferrite_core::peer::connection::PeerConnection;
use ferrite_core::peer::message::Message;
use ferrite_core::torrent::parser::parse;
use ferrite_core::tracker::http::announce;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::main]
async fn main() {
    let raw = std::fs::read("tests/fixtures/ubuntu26.torrent").unwrap();
    let bencode = decode(&raw).unwrap();
    let torrent = parse(&raw, bencode).unwrap();

    let all_peers = tokio::task::spawn_blocking({
        let torrent = torrent.clone();
        move || {
            let mut peers = announce(&torrent).unwrap_or_default();

            if let Some(ref list) = torrent.announce_list {
                for tier in list {
                    for url in tier {
                        if url.starts_with("http") {
                            let mut t = torrent.clone();
                            t.announce = url.clone();
                            if let Ok(more) = announce(&t) {
                                peers.extend(more);
                            }
                        }
                    }
                }
            }

            let mut t2 = torrent.clone();
            t2.announce = "http://tracker.opentrackr.org:1337/announce".to_string();
            if let Ok(more) = announce(&t2) {
                peers.extend(more);
            }

            peers
        }
    })
    .await
    .unwrap();

    let ipv4_peers: Vec<_> = all_peers.iter().filter(|p| !p.ip.contains(':')).collect();
    println!("Got {} IPv4 peers", ipv4_peers.len());

    let peer_id: [u8; 20] = *b"-FE0001-XXXXXXXXXXXX";

    for peer in ipv4_peers.iter() {
        let addr = format!("{}:{}", peer.ip, peer.port);
        println!("Trying {}", addr);

        let conn_result = timeout(
            Duration::from_secs(5),
            PeerConnection::connect(&addr, torrent.info_hash, peer_id),
        )
        .await;

        let mut conn = match conn_result {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => {
                println!("Failed: {}", e);
                continue;
            }
            Err(_) => {
                println!("Timeout connecting to {}", addr);
                continue;
            }
        };

        println!("Handshake success!");
        if let Err(e) = conn.send_message(Message::Interested).await {
            println!("Send error: {}", e);
            continue;
        }

        let mut unchoked = false;
        for _ in 0..10 {
            match timeout(Duration::from_secs(5), conn.read_message()).await {
                Ok(Ok(Message::Unchoke)) => {
                    unchoked = true;
                    println!("Unchoked!");
                    break;
                }
                Ok(Ok(Message::Choke)) => {
                    if let Err(e) = conn.send_message(Message::Interested).await {
                        println!("Send error: {}", e);
                        break;
                    }
                }
                Ok(Ok(Message::Bitfield(bits))) => {
                    println!("Bitfield ({} bytes)", bits.len());
                }
                Ok(Ok(msg)) => println!("Received: {:?}", msg),
                Ok(Err(e)) => {
                    println!("Read error: {}", e);
                    break;
                }
                Err(_) => {
                    println!("Read timeout");
                    break;
                }
            }
        }

        if unchoked {
            let piece_length = 262144usize;
            let block_size = 16384usize;
            let num_blocks = piece_length / block_size;
            let mut piece_data = vec![0u8; piece_length];
            let mut blocks_received = 0;

            // pipeline all requests first
            for block in 0..num_blocks {
                let begin = (block * block_size) as u32;
                if let Err(e) = conn
                    .send_message(Message::Request {
                        index: 0,
                        begin,
                        length: block_size as u32,
                    })
                    .await
                {
                    println!("Send error: {}", e);
                    break;
                }
            }
            println!("Sent all {} requests", num_blocks);

            // now read all responses
            for _ in 0..num_blocks * 2 {
                match timeout(Duration::from_secs(10), conn.read_message()).await {
                    Ok(Ok(Message::Piece { index, begin, data })) => {
                        println!("Got block at offset {}", begin);
                        let offset = begin as usize;
                        piece_data[offset..offset + data.len()].copy_from_slice(&data);
                        blocks_received += 1;
                        if blocks_received == num_blocks {
                            break;
                        }
                    }
                    Ok(Ok(Message::Choke)) => {
                        println!("Choked mid-download, re-sending Interested");
                        if let Err(e) = conn.send_message(Message::Interested).await {
                            println!("Send error: {}", e);
                            break;
                        }
                    }
                    Ok(Ok(Message::Unchoke)) => {
                        println!("Unchoked again, re-sending requests");
                        for block in blocks_received..num_blocks {
                            let begin = (block * block_size) as u32;
                            if let Err(e) = conn
                                .send_message(Message::Request {
                                    index: 0,
                                    begin,
                                    length: block_size as u32,
                                })
                                .await
                            {
                                println!("Send error: {}", e);
                                break;
                            }
                        }
                    }
                    Ok(Ok(msg)) => println!("Unexpected: {:?}", msg),
                    Ok(Err(e)) => {
                        println!("Error: {}", e);
                        break;
                    }
                    Err(_) => {
                        println!("Read timeout");
                        break;
                    }
                }
            }

            if blocks_received == num_blocks {
                use sha1::{Digest, Sha1};
                let hash: [u8; 20] = Sha1::digest(&piece_data).into();
                if hash == torrent.pieces[0] {
                    println!("SHA1 verified! Writing to disk...");
                    std::fs::write("piece_0.bin", &piece_data).unwrap();
                    println!("Written to piece_0.bin");
                    return;
                } else {
                    println!("SHA1 mismatch! Piece corrupted.");
                }
            } else {
                println!(
                    "Incomplete: got {}/{} blocks, trying next peer",
                    blocks_received, num_blocks
                );
            }
        }
    }
}

use ferrite_core::bencode::decoder::decode;
use ferrite_core::peer::connection::PeerConnection;
use ferrite_core::torrent::parser::parse;
use ferrite_core::tracker::http::announce;

#[tokio::main]
async fn main() {
    let raw = std::fs::read("tests/fixtures/ubuntu.torrent").unwrap();
    let bencode = decode(&raw).unwrap();
    let torrent = parse(&raw, bencode).unwrap();

    println!("Parsed torrent: {}", torrent.name);

    // run blocking announce before async context
    let peers = tokio::task::spawn_blocking(move || announce(&torrent).unwrap())
        .await
        .unwrap();

    println!("Got {} peers", peers.len());

    let peer_id: [u8; 20] = *b"-FE0001-XXXXXXXXXXXX";

    // re-parse torrent since we moved it above
    let raw2 = std::fs::read("tests/fixtures/ubuntu.torrent").unwrap();
    let bencode2 = decode(&raw2).unwrap();
    let torrent2 = parse(&raw2, bencode2).unwrap();

    for peer in peers.iter().filter(|p| !p.ip.contains(':')).take(10) {
        let addr = format!("{}:{}", peer.ip, peer.port);
        println!("Trying {}", addr);
        match PeerConnection::connect(&addr, torrent2.info_hash, peer_id).await {
            Ok(conn) => {
                println!(
                    "Handshake success! peer_id: {}",
                    String::from_utf8_lossy(&conn.peer_id)
                );
                break;
            }
            Err(e) => println!("Failed: {}", e),
        }
    }
}

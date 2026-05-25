use ferrite_core::bencode::decoder::decode;
use ferrite_core::torrent::parser::parse;
use ferrite_core::tracker::http::announce;

fn main() {
    let raw = std::fs::read("tests/fixtures/ubuntu.torrent").unwrap();
    let bencode = decode(&raw).unwrap();
    let torrent = parse(&raw, bencode).unwrap();

    println!("Parsed torrent: {}", torrent.name);
    println!("Announcing to: {}", torrent.announce);

    let peers = announce(&torrent).unwrap();
    println!("Got {} peers", peers.len());
    for peer in peers.iter().take(5) {
        println!("  {}:{}", peer.ip, peer.port);
    }
}

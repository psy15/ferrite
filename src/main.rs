use ferrite_core::bencode::decoder::decode;
use ferrite_core::download::manager::{
    DownloadConfig, PieceManager, create_output_file, peer_task, resume,
};
use ferrite_core::torrent::parser::parse;
use ferrite_core::tracker::http::announce;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    let raw = std::fs::read("tests/fixtures/bigbuckbunny.torrent").unwrap();
    let bencode = decode(&raw).unwrap();
    let torrent = parse(&raw, bencode).unwrap();

    println!("Torrent: {}", torrent.name);
    println!("Size: {} bytes", torrent.length);
    println!("Pieces: {}", torrent.pieces.len());

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

    if ipv4_peers.is_empty() {
        println!("No IPv4 peers found, exiting");
        return;
    }

    fn generate_peer_id() -> [u8; 20] {
        use std::time::{SystemTime, UNIX_EPOCH};
        let mut id = *b"-FE0001-XXXXXXXXXXXX";
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let bytes = seed.to_be_bytes();
        for i in 0..4 {
            id[8 + i] = bytes[i];
            id[12 + i] = bytes[i].wrapping_add(i as u8 * 37);
            id[16 + i] = bytes[i].wrapping_mul(i as u8 + 13);
        }
        id
    }

    let peer_id = generate_peer_id();
    let total_pieces = torrent.pieces.len();
    let manager = Arc::new(Mutex::new(PieceManager::new(total_pieces)));

    let files = torrent.files.clone().unwrap_or_else(|| {
        vec![ferrite_core::torrent::types::FileInfo {
            path: torrent.name.clone(),
            length: torrent.length,
        }]
    });

    create_output_file(&torrent.name, &files).await.unwrap();

    resume(
        &torrent.name,
        &files,
        &torrent.pieces,
        torrent.piece_length as usize,
        torrent.length,
        &manager,
    )
    .await;

    let mut handles = vec![];

    let config = Arc::new(DownloadConfig {
        info_hash: torrent.info_hash,
        peer_id,
        piece_hashes: torrent.pieces.clone(),
        piece_length: torrent.piece_length as usize,
        output_dir: torrent.name.clone(),
        file_size: torrent.length,
        files,
    });

    for peer in ipv4_peers.iter() {
        let addr = format!("{}:{}", peer.ip, peer.port);
        let handle = tokio::spawn(peer_task(addr, config.clone(), manager.clone()));
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }

    let (done, total) = manager.lock().unwrap().progress();
    if manager.lock().unwrap().is_complete() {
        println!("Download complete!");
    } else {
        println!("Incomplete: {}/{} pieces", done, total);
    }
}

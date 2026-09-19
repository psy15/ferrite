use crate::torrent::types::FileInfo;
use crate::{FeriteError, Result};
use std::io::SeekFrom;
use std::sync::{Arc, Mutex};
use tokio::fs::OpenOptions;
use tokio::io::AsyncSeekExt;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, PartialEq)]
enum PieceState {
    NotStarted,
    InProgress,
    Done,
}

pub struct PieceManager {
    states: Vec<PieceState>,
    done_count: usize,
    pub total: usize,
}

impl PieceManager {
    pub fn new(total_pieces: usize) -> Self {
        Self {
            states: vec![PieceState::NotStarted; total_pieces],
            done_count: 0,
            total: total_pieces,
        }
    }

    pub fn next_piece(&mut self) -> Option<u32> {
        for (index, state) in self.states.iter_mut().enumerate() {
            if *state == PieceState::NotStarted {
                *state = PieceState::InProgress;
                return Some(index as u32);
            }
        }
        None
    }

    pub fn mark_failed(&mut self, index: u32) {
        self.states[index as usize] = PieceState::NotStarted;
    }

    pub fn mark_done(&mut self, index: u32) {
        self.states[index as usize] = PieceState::Done;
        self.done_count += 1;
    }

    pub fn is_complete(&self) -> bool {
        self.done_count == self.total
    }

    pub fn progress(&self) -> (usize, usize) {
        (self.done_count, self.total)
    }

    pub fn missing_pieces(&self) -> Vec<u32> {
        self.states
            .iter()
            .enumerate()
            .filter(|(_, s)| **s == PieceState::NotStarted)
            .map(|(i, _)| i as u32)
            .collect()
    }
}

pub async fn create_output_file(output_dir: &str, files: &[FileInfo]) -> Result<()> {
    tokio::fs::create_dir_all(output_dir)
        .await
        .map_err(FeriteError::Io)?;

    for file in files {
        let path = format!("{}/{}", output_dir, file.path);
        if tokio::fs::metadata(&path).await.is_ok() {
            continue;
        }
        if let Some(parent) = std::path::Path::new(&path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(FeriteError::Io)?;
        }
        let f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .await
            .map_err(FeriteError::Io)?;
        f.set_len(file.length).await.map_err(FeriteError::Io)?;
    }

    Ok(())
}

pub async fn resume(
    output_dir: &str,
    files: &[FileInfo],
    piece_hashes: &[[u8; 20]],
    piece_length: usize,
    file_size: u64,
    manager: &Arc<Mutex<PieceManager>>,
) {
    use sha1::{Digest, Sha1};
    use tokio::io::AsyncReadExt;

    // for resume, read pieces sequentially across all files
    // build a flat reader by reading from each file in order
    let total = piece_hashes.len();
    let mut done = 0;

    for (i, expected_hash) in piece_hashes.iter().enumerate() {
        let actual_length = if i == total - 1 {
            (file_size - (i as u64 * piece_length as u64)) as usize
        } else {
            piece_length
        };

        let mut buf = vec![0u8; actual_length];
        let mut buf_offset = 0usize;
        let piece_start = i as u64 * piece_length as u64;
        let piece_end = piece_start + actual_length as u64;
        let mut file_start = 0u64;

        for file in files {
            let file_end = file_start + file.length;
            if piece_start < file_end && piece_end > file_start {
                let read_start = piece_start.max(file_start);
                let read_end = piece_end.min(file_end);
                let read_len = (read_end - read_start) as usize;
                let file_offset = read_start - file_start;

                let path = format!("{}/{}", output_dir, file.path);
                if let Ok(mut f) = tokio::fs::File::open(&path).await {
                    use tokio::io::AsyncSeekExt;
                    if f.seek(SeekFrom::Start(file_offset)).await.is_ok() {
                        let _ = f
                            .read_exact(&mut buf[buf_offset..buf_offset + read_len])
                            .await;
                        buf_offset += read_len;
                    }
                }
            }
            file_start = file_end;
        }

        let hash: [u8; 20] = Sha1::digest(&buf).into();
        if hash == *expected_hash {
            manager.lock().unwrap().mark_done(i as u32);
            done += 1;
        }
    }

    println!("Fast resume: {}/{} pieces already done", done, total);
    let missing = manager.lock().unwrap().missing_pieces();
    if !missing.is_empty() {
        println!("Missing: {:?}", missing);
    }
}

pub async fn write_piece(
    output_dir: &str,
    piece_index: u32,
    piece_length: u64,
    data: &[u8],
    files: &[FileInfo],
) -> Result<()> {
    let piece_start = piece_index as u64 * piece_length;
    let piece_end = piece_start + data.len() as u64;
    let mut file_start = 0u64;

    for file in files {
        let file_end = file_start + file.length;

        if piece_start < file_end && piece_end > file_start {
            let write_start = piece_start.max(file_start);
            let write_end = piece_end.min(file_end);
            let data_slice =
                &data[(write_start - piece_start) as usize..(write_end - piece_start) as usize];
            let file_offset = write_start - file_start;
            let path = format!("{}/{}", output_dir, file.path);

            if let Some(parent) = std::path::Path::new(&path).parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(FeriteError::Io)?;
            }

            let mut f = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(&path)
                .await
                .map_err(FeriteError::Io)?;

            f.seek(SeekFrom::Start(file_offset))
                .await
                .map_err(FeriteError::Io)?;

            f.write_all(data_slice).await.map_err(FeriteError::Io)?;
        }

        file_start = file_end;
    }

    Ok(())
}

pub struct DownloadConfig {
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
    pub piece_hashes: Vec<[u8; 20]>,
    pub piece_length: usize,
    pub output_dir: String,
    pub file_size: u64,
    pub files: Vec<FileInfo>,
}

pub async fn peer_task(
    addr: String,
    config: Arc<DownloadConfig>,
    manager: Arc<Mutex<PieceManager>>,
) {
    use crate::peer::connection::PeerConnection;
    use crate::peer::message::Message;
    use sha1::{Digest, Sha1};
    use tokio::time::{Duration, timeout};

    let mut conn = match timeout(
        Duration::from_secs(5),
        PeerConnection::connect(&addr, config.info_hash, config.peer_id),
    )
    .await
    {
        Ok(Ok(c)) => c,
        Ok(Err(e)) => {
            println!("Connect failed {}: {}", addr, e);
            return;
        }
        Err(_) => {
            println!("Connect timeout {}", addr);
            return;
        }
    };

    println!("Connected to {}", addr);

    if conn.send_message(Message::Interested).await.is_err() {
        return;
    }

    let mut unchoked = false;
    for _ in 0..10 {
        match timeout(Duration::from_secs(5), conn.read_message()).await {
            Ok(Ok(Message::Unchoke)) => {
                unchoked = true;
                break;
            }
            Ok(Ok(Message::Bitfield(_))) => {}
            Ok(Ok(Message::Choke)) => {
                let _ = conn.send_message(Message::Interested).await;
            }
            _ => break,
        }
    }

    if !unchoked {
        return;
    }

    loop {
        let piece_index = {
            let mut m = manager.lock().unwrap();
            match m.next_piece() {
                Some(i) => i,
                None => break,
            }
        };

        let actual_length = if piece_index == (config.piece_hashes.len() - 1) as u32 {
            (config.file_size - (piece_index as u64 * config.piece_length as u64)) as usize
        } else {
            config.piece_length
        };

        let block_size = 16384usize;
        let num_blocks = actual_length.div_ceil(block_size);
        let mut piece_data = vec![0u8; actual_length];
        let mut blocks_received = 0;

        for block in 0..num_blocks {
            let begin = (block * block_size) as u32;
            let this_block_size = if begin as usize + block_size > actual_length {
                actual_length - begin as usize
            } else {
                block_size
            };

            if conn
                .send_message(Message::Request {
                    index: piece_index,
                    begin,
                    length: this_block_size as u32,
                })
                .await
                .is_err()
            {
                manager.lock().unwrap().mark_failed(piece_index);
                return;
            }
        }

        for _ in 0..num_blocks * 2 {
            match timeout(Duration::from_secs(10), conn.read_message()).await {
                Ok(Ok(Message::Piece { begin, data, .. })) => {
                    let offset = begin as usize;
                    if offset + data.len() <= piece_data.len() {
                        piece_data[offset..offset + data.len()].copy_from_slice(&data);
                        blocks_received += 1;
                        if blocks_received == num_blocks {
                            break;
                        }
                    }
                }
                Ok(Ok(Message::Choke)) => {
                    manager.lock().unwrap().mark_failed(piece_index);
                    return;
                }
                Ok(Ok(_)) => {}
                _ => {
                    manager.lock().unwrap().mark_failed(piece_index);
                    return;
                }
            }
        }

        if blocks_received < num_blocks {
            manager.lock().unwrap().mark_failed(piece_index);
            return;
        }

        let hash: [u8; 20] = Sha1::digest(&piece_data).into();
        if hash != config.piece_hashes[piece_index as usize] {
            manager.lock().unwrap().mark_failed(piece_index);
            continue;
        }

        if write_piece(
            &config.output_dir,
            piece_index,
            config.piece_length as u64,
            &piece_data,
            &config.files,
        )
        .await
        .is_err()
        {
            manager.lock().unwrap().mark_failed(piece_index);
            return;
        }

        manager.lock().unwrap().mark_done(piece_index);
        let (done, total) = manager.lock().unwrap().progress();
        println!("Progress: {}/{} pieces", done, total);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_piece_marks_in_progress() {
        let mut m = PieceManager::new(3);
        assert_eq!(m.next_piece(), Some(0));
        assert_eq!(m.next_piece(), Some(1));
        assert_eq!(m.next_piece(), Some(2));
        assert_eq!(m.next_piece(), None);
    }

    #[test]
    fn test_mark_failed_makes_available_again() {
        let mut m = PieceManager::new(3);
        m.next_piece(); // take piece 0
        m.mark_failed(0);
        assert_eq!(m.next_piece(), Some(0)); // available again
    }

    #[test]
    fn test_mark_done() {
        let mut m = PieceManager::new(3);
        m.next_piece();
        m.mark_done(0);
        assert_eq!(m.done_count, 1);
        assert!(!m.is_complete());
    }

    #[test]
    fn test_is_complete() {
        let mut m = PieceManager::new(2);
        m.next_piece();
        m.mark_done(0);
        m.next_piece();
        m.mark_done(1);
        assert!(m.is_complete());
    }

    #[test]
    fn test_missing_pieces() {
        let mut m = PieceManager::new(3);
        m.next_piece(); // piece 0 in progress
        m.mark_done(0); // piece 0 done
        let missing = m.missing_pieces();
        assert_eq!(missing, vec![1, 2]);
    }
}

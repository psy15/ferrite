#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: String,
    pub length: u64,
}

#[derive(Debug, Clone)]
pub struct TorrentFile {
    pub announce: String,
    pub announce_list: Option<Vec<Vec<String>>>,
    pub name: String,
    pub length: u64, // total size (sum of all files)
    pub piece_length: u64,
    pub pieces: Vec<[u8; 20]>,
    pub info_hash: [u8; 20],
    pub files: Option<Vec<FileInfo>>, // None for single-file torrents
}

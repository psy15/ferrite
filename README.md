# ferrite

A BitTorrent client written in Rust. Built on Tokio for async networking, handles concurrent peer connections, piece verification, and disk writes without a garbage collector getting in the way.

The core engine runs as a background daemon and exposes a gRPC server. A terminal UI sits on top, fully keyboard-driven, updating in real time. The core is also usable as a standalone library.

## Features

- `.torrent` file and magnet link support
- HTTP and UDP tracker communication
- Peer wire protocol - concurrent connections, choke/unchoke, piece requests
- SHA1 verification on every piece before writing to disk
- gRPC interface - terminal UI and any future frontend connect to the same daemon

## Status

- [x] Bencode decoder
- [x] Torrent file parser + info hash computation
- [ ] HTTP tracker communication
- [ ] Peer wire protocol
- [ ] Piece management + SHA1 verification
- [ ] gRPC server
- [ ] Terminal UI
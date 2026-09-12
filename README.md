# ferrite

A BitTorrent client written in Rust. Uses Tokio for async networking.

The core engine (`ferrite-core`) is a standalone library. A terminal UI built with ratatui sits on top and imports it directly.

## What it does

- Reads `.torrent` files and decodes their binary bencode format
- Computes the info_hash, which identifies a torrent to every peer in the world
- Announces to HTTP trackers and gets back a list of peers
- Connects to peers over raw TCP and does the BitTorrent handshake
- Downloads pieces using the peer wire protocol with pipelined block requests
- Verifies every piece against its SHA1 hash before writing to disk

## Architecture

```
ferrite-tui  ->  imports  ->  ferrite-core
(ratatui)                     (tokio, sha1)
```

The TUI imports the core as a library. No daemon, no IPC.

## Status

- [x] Bencode decoder
- [x] Torrent file parser + info_hash computation
- [x] HTTP tracker, peer list parsing (compact + dict format)
- [x] TCP peer connection + BitTorrent handshake
- [x] Peer wire message parsing (all 8 message types)
- [x] Piece download, pipelined requests, out-of-order block assembly
- [x] SHA1 verification + disk write
- [ ] Download manager, all pieces, concurrent peers
- [ ] Magnet link support
- [ ] UDP tracker
- [ ] Terminal UI (ratatui)
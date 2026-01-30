# Distributed KV Store (kv_store)

A experimental peer-to-peer distributed key-value store written in Rust.

This project demonstrates a simple gossip-based KV store using the iroh ecosystem (gossip, tickets, blobs). It exposes a minimal interactive CLI that allows you to set/get/delete keys locally and gossip those updates to other peers subscribed to the same topic.

# Status

1. Two node store (completed)
2. Multi node (Pending)
3. CRDT (Pending)
3 B. merkle trees (Pending)
4. Optimization (reduce copy)

## Features

- Interactive command-line interface (REPL-like).
- Set/get/delete keys on a local in-memory store.
- Broadcast updates to peers via a gossip topic using `iroh-gossip`.
- Simple last-write-wins merge based on an embedded timestamp.
- Generate and use endpoint tickets to join other peers.

## Key concepts & internals

- `StoreValue` — serializable struct containing the value, origin node, optional node id, and a unix timestamp.
- `KVStore` — an in-memory HashMap guarded by a Mutex. Exposes `set`, `get`, `delete`, `merge`, and `print_store`.
- Gossip messages are serialized with `wincode` and broadcast on a 32-byte topic prefixed with `kv-store`.
- When a gossip message is received, the node calls `merge` and keeps the `StoreValue` with the newer timestamp.

## Dependencies (high level)

- Rust (stable toolchain)
- tokio (async runtime)
- iroh, iroh-gossip, iroh-tickets (P2P/gossip primitives)
- wincode (binary serialization)
- serde (serialization helpers)

See `Cargo.toml` for the exact dependency versions used in this repository.

## Build

Ensure you have a working Rust toolchain (rustup + cargo).

To build the project:

```bash
cargo build --release
```

Or for a debug build while developing:

```bash
cargo build
```

The binary (debug) will be available at `target/debug/kv_store`.

## Run & Usage

Start the node (example using debug binary):

```bash
cargo run --release
# or
./target/debug/kv_store
```

Once started you'll see a simple prompt. Supported interactive commands:

- `set <key> <value>` — store a value locally and broadcast it to the gossip topic.
- `get <key>` — print the value for a key if present locally.
- `delete <key>` — remove the key locally and broadcast a delete message.
- `ticket` — generate and print an `EndpointTicket` for this node; give it to another node so they can `join` you.
- `join <ticket>` — subscribe to the gossip topic for a peer described by the ticket (connect to that peer’s gossip).
- `print` — dump the local in-memory store to stdout.
- `q`, `quit`, `exit` — exit the program.

Example session (two machines or two processes):

1. Start node A: `cargo run --release` → run `ticket` and copy the printed ticket.
2. Start node B: `cargo run --release` → run `join <ticket-from-A>`.
3. On node B, `set mykey hello`.
4. Node A should receive the gossip and merge the value.

Note: tickets include the endpoint address used by the `iroh` transport; if your nodes are behind NATs or firewalls, connectivity depends on your network topology.

## Serialization & Message Format

- Gossip messages are instances of `GossipStoreMessage { key: String, value: StoreValue }` serialized using `wincode`.

## Limitations & Notes

- The store is in-memory only. No persistence across restarts.
- Merge policy is simple: later timestamp wins. This is not a robust CRDT — it's for experimentation.
- There is minimal error handling in the interactive CLI. Inputs are parsed naively.
- No authentication beyond what `iroh` provides. Be mindful when running on public networks.

## Extending / Next steps

- Add persistent storage (sled, rocksdb, or simple file-based snapshotting).
- Replace timestamp-based LWW with a proper CRDT for conflict-free merging.
- Expose a JSON/HTTP or gRPC API for remote clients (instead of the interactive CLI).
- Add tests for serialization and merge behavior.

## Where to look in the code

- `src/main.rs` — main program logic, CLI, gossip handling, and KVStore implementation.
- `Cargo.toml` — dependency versions.

## License

No license specified. Add a LICENSE file if you want to make the project open source under a particular license.

---

If you'd like, I can also:

- add a tiny example script that launches two local instances and demonstrates `ticket`/`join`/`set` automatically,
- add a CONTRIBUTING section or CI workflow for linting/tests,
- or convert the REPL into a simple TCP/HTTP API for remote usage.

If you'd like any of those, tell me which and I will implement it next.

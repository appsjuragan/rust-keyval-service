# Rust Key-Value Memory Service

A lightweight in-memory key–value store written in **Rust**, featuring:

- Key expiration (TTL)
- Automatic cleanup thread
- Memory usage tracking
- Periodic monitoring (every 1 second)
- Simple TCP text protocol on **port 11223**

A **Go benchmark client** is included to load-test the service with a ~30% cache hit ratio and multi-threaded request generation.

---

## 🚀 Features

### 🦀 Rust Key-Value Service
- Commands supported:
SET key value ttl_seconds
GET key
DEL key
STATS

- Automatic key expiry using background cleaner thread  
- Monitoring every 1 second:
- Current KV count
- Total expired keys
- Estimated memory usage in bytes
- Thread-safe & memory-leak-free (Arc + Mutex)
- Simple line-based TCP protocol

### 🐹 Go Benchmark Client
- Multi-threaded load generator
- ~30% cache-hit ratio simulation
- Randomized keys, values, and TTLs
- Benchmark metrics:
- Operations per second
- Average latency
- Hit/miss ratio

---

## 📦 Project Structure
```
.
├── src/
│ └── main.rs # Rust KV server
├── poc/
│ ├── client.go # Simple Go client example
│ ├── bench.go # Go benchmark generator
├── Cargo.toml
├── go.mod
├── .gitignore
└── README.md
---
```

## 🛠 Running the Rust Service

```bash
cd rust-keyval-service
cargo run --release
```

Example output:
```
[monitor] keys=123 expired=44 mem=18240 bytes
Listening on port 11223...
```

🧪 Using the Service Manually
With Netcat:
```
nc localhost 11223
```

Example:
```
SET hello world 5
OK
GET hello
world
STATS
keys=1 expired=0 mem=32
```
⚡ Running the Go Benchmark
```
cd poc
go run bench.go
```

Example result:
```
Threads: 20
Requests: 200000
Hit ratio: 31%
Ops/sec: 105,221
Avg latency: 0.18 ms
```

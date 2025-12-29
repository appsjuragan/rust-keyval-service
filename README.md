# 🔑 Rust Key-Value Service with Backoffice Dashboard

A high-performance in-memory key-value store written in **Rust**, featuring real-time metrics visualization with an ELK stack and modern web dashboard.

![Rust](https://img.shields.io/badge/Rust-1.83-orange)
![Docker](https://img.shields.io/badge/Docker-Compose-blue)
![License](https://img.shields.io/badge/License-MIT-green)

---

## ✨ Features

### 🦀 Key-Value Service
- **Commands**: `SET`, `GET`, `DEL`, `STATS`
- **Key Expiration (TTL)**: Automatic cleanup of expired keys
- **Thread Pool**: 8 concurrent workers for high throughput
- **Metrics Tracking**: Hits, misses, memory usage, operations count
- **Elasticsearch Integration**: Automatic metrics emission every 5 seconds

### 📊 Backoffice Dashboard
- **Real-time Charts**: Hit/miss ratio, operations breakdown, trending graphs
- **Auto-refresh**: Updates every 5 seconds
- **Modern UI**: Dark theme with glassmorphism design
- **Responsive**: Works on desktop and mobile

### 🐳 Docker Ready
- **Alpine-based Images**: Minimal footprint (~27MB for KV service)
- **Full Stack**: Elasticsearch, Kibana, KV Service, Dashboard
- **Single Command Deploy**: `docker-compose up -d`

---

## 🚀 Quick Start

### Using Docker (Recommended)

```bash
# Clone the repository
git clone https://github.com/your-repo/rust-keyval-service.git
cd rust-keyval-service

# Start all services
docker-compose -f docker-compose.elk.yml up -d

# Wait ~30 seconds for Elasticsearch to be ready
```

### Access Points

| Service | URL |
|---------|-----|
| **Dashboard** | http://localhost:8080 |
| **KV Service** | `nc localhost 11223` |
| **Elasticsearch** | http://localhost:9200 |
| **Kibana** | http://localhost:5601 |

---

## 📦 Project Structure

```
rust-keyval-service/
├── src/
│   └── main.rs              # Rust KV server with metrics
├── backoffice/
│   ├── index.html           # Dashboard UI
│   ├── index.css            # Styling
│   ├── app.js               # Chart.js visualizations
│   └── Dockerfile           # Frontend container
├── config/
│   └── elasticsearch.yml    # ES CORS configuration
├── poc/
│   ├── bench.go             # Basic benchmark
│   ├── bench-random.go      # Hit/miss ratio benchmark
│   └── client.go            # Simple Go client
├── Cargo.toml
├── Dockerfile               # KV service container
├── docker-compose.elk.yml   # Full stack deployment
└── README.md
```

---

## 🔧 Protocol

The service uses a simple TCP text protocol on **port 11223**.

### Commands

```bash
# Connect
nc localhost 11223

# SET key ttl_seconds value
SET username 60 john_doe
> OK

# GET key
GET username
> john_doe

# DEL key
DEL username
> 1

# STATS
STATS
> items=5 memory=320bytes expired=2 hits=100 misses=30 hit_ratio=76.9%
```

---

## 📈 Benchmarking

### Generate Traffic

```bash
cd poc

# Basic benchmark: 10 threads, 30 seconds
go run bench.go 10 30

# Hit/miss benchmark: 4 threads, 30 seconds, 100 cache keys
go run bench-random.go 4 30 100
```

### Expected Results

```
🔥 Starting benchmark
Threads      : 4
Seconds      : 30
Cache Keys   : 100
Hit Ratio    : ~30%

====== Benchmark Results ======
Total Ops : 450000
Duration  : 30.00s
Throughput: 15000.00 ops/sec
================================
```

---

## 🏗️ Development

### Build Locally

```bash
# Build and run
cargo run --release

# Output
KV server listening on port 11223
[MONITOR] items=0  memory=0 bytes  expired=0  hits=0  misses=0  hit_ratio=0.0%
```

### Run with Docker

```bash
# Build images
docker-compose -f docker-compose.elk.yml build

# Start services
docker-compose -f docker-compose.elk.yml up -d

# View logs
docker logs -f kv-service

# Stop services
docker-compose -f docker-compose.elk.yml down
```

---

## 🐳 Docker Image Sizes

| Image | Size |
|-------|------|
| `rust-keyval-service-kv-service` | **26.7 MB** |
| `rust-keyval-service-backoffice` | **81.2 MB** |

---

## 📊 Dashboard Preview

The backoffice dashboard provides real-time visualization of:

- **Hit/Miss Ratio** - Doughnut chart showing cache efficiency
- **Operations Breakdown** - GET/SET/DELETE distribution
- **Trending Graphs** - Historical data over the last hour
- **Usage Statistics** - Memory usage, cached items, expired keys

---

## 🔧 Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ES_URL` | `http://localhost:9200` | Elasticsearch endpoint |

### Elasticsearch Settings

CORS is configured in `config/elasticsearch.yml`:

```yaml
http.cors.enabled: true
http.cors.allow-origin: "*"
http.cors.allow-methods: OPTIONS, HEAD, GET, POST, PUT, DELETE
```

---

## 📝 License

MIT License - feel free to use this project for any purpose.

---

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Commit your changes
4. Push to the branch
5. Open a Pull Request

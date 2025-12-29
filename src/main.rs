use std::{
    collections::HashMap,
    env,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}},
    thread,
    time::{Duration, SystemTime},
};

use crossbeam_channel;
use chrono::Utc;
use serde::Serialize;

#[derive(Clone)]
struct Item {
    value: Vec<u8>,
    expires_at: Option<SystemTime>,
}

type Db = Arc<Mutex<HashMap<String, Item>>>;

const THREAD_POOL_SIZE: usize = 8;
const CLEAN_INTERVAL: u64 = 1;
const METRICS_INTERVAL: u64 = 5;
const ES_INDEX: &str = "kv-metrics";

fn get_es_url() -> String {
    env::var("ES_URL").unwrap_or_else(|_| "http://localhost:9200".to_string())
}

// GLOBAL METRICS
static EXPIRED_COUNTER: AtomicUsize = AtomicUsize::new(0);
static HIT_COUNTER: AtomicUsize = AtomicUsize::new(0);
static MISS_COUNTER: AtomicUsize = AtomicUsize::new(0);
static GET_COUNTER: AtomicUsize = AtomicUsize::new(0);
static SET_COUNTER: AtomicUsize = AtomicUsize::new(0);
static DEL_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Serialize)]
struct Metrics {
    #[serde(rename = "@timestamp")]
    timestamp: String,
    hits: usize,
    misses: usize,
    hit_ratio: f64,
    gets: usize,
    sets: usize,
    deletes: usize,
    expired: usize,
    items: usize,
    memory_bytes: usize,
}

fn main() {
    let db: Db = Arc::new(Mutex::new(HashMap::new()));

    start_cleaner(db.clone());
    start_monitor(db.clone());
    start_metrics_emitter(db.clone());

    let listener = TcpListener::bind("0.0.0.0:11223").expect("cannot bind 11223");
    println!("KV server listening on port 11223");

    let (tx, rx) = crossbeam_channel::unbounded();

    // Worker pool
    for _ in 0..THREAD_POOL_SIZE {
        let rx_clone = rx.clone();
        let db_clone = db.clone();

        thread::spawn(move || loop {
            if let Ok(stream) = rx_clone.recv() {
                handle_client(stream, db_clone.clone());
            }
        });
    }

    // Accept loop
    for stream in listener.incoming() {
        if let Ok(s) = stream {
            tx.send(s).ok();
        }
    }
}

fn handle_client(mut stream: TcpStream, db: Db) {
    let mut buf = [0u8; 4096];

    loop {
        let read = match stream.read(&mut buf) {
            Ok(0) => return,
            Ok(n) => n,
            Err(_) => return,
        };

        let input = String::from_utf8_lossy(&buf[..read]).trim().to_string();
        let mut parts = input.splitn(4, ' ');
        let cmd = parts.next().unwrap_or("").to_uppercase();

        match cmd.as_str() {
            "SET" => {
                let key = parts.next().unwrap_or("");
                let ttl = parts.next().unwrap_or("0").parse::<u64>().unwrap_or(0);
                let value = parts.next().unwrap_or("").as_bytes().to_vec();

                let expires_at = if ttl > 0 {
                    Some(SystemTime::now() + Duration::from_secs(ttl))
                } else {
                    None
                };

                db.lock().unwrap().insert(key.to_string(), Item { value, expires_at });
                SET_COUNTER.fetch_add(1, Ordering::Relaxed);
                stream.write_all(b"OK\n").ok();
            }

            "GET" => {
                let key = parts.next().unwrap_or("").to_string();
                let mut db_guard = db.lock().unwrap();
                GET_COUNTER.fetch_add(1, Ordering::Relaxed);

                if let Some(item) = db_guard.get(&key) {
                    if let Some(exp) = item.expires_at {
                        if exp <= SystemTime::now() {
                            db_guard.remove(&key);
                            EXPIRED_COUNTER.fetch_add(1, Ordering::Relaxed);
                            MISS_COUNTER.fetch_add(1, Ordering::Relaxed);
                            stream.write_all(b"(expired)\n").ok();
                            continue;
                        }
                    }
                    HIT_COUNTER.fetch_add(1, Ordering::Relaxed);
                    let mut data = item.value.clone();
                    data.push(b'\n');
                    stream.write_all(&data).ok();
                } else {
                    MISS_COUNTER.fetch_add(1, Ordering::Relaxed);
                    stream.write_all(b"(nil)\n").ok();
                }
            }

            "DEL" => {
                let key = parts.next().unwrap_or("").to_string();
                let removed = db.lock().unwrap().remove(&key);
                DEL_COUNTER.fetch_add(1, Ordering::Relaxed);
                if removed.is_some() { stream.write_all(b"1\n").ok(); }
                else { stream.write_all(b"0\n").ok(); }
            }

            "STATS" => {
                let db_guard = db.lock().unwrap();

                let items = db_guard.len();
                let memory: usize = db_guard.values()
                    .map(|v| v.value.len() + 64)
                    .sum();

                let hits = HIT_COUNTER.load(Ordering::Relaxed);
                let misses = MISS_COUNTER.load(Ordering::Relaxed);
                let total = hits + misses;
                let hit_ratio = if total > 0 { (hits as f64 / total as f64) * 100.0 } else { 0.0 };

                let msg = format!(
                    "items={} memory={}bytes expired={} hits={} misses={} hit_ratio={:.1}%\n",
                    items,
                    memory,
                    EXPIRED_COUNTER.load(Ordering::Relaxed),
                    hits,
                    misses,
                    hit_ratio,
                );
                stream.write_all(msg.as_bytes()).ok();
            }

            _ => {
                stream.write_all(b"ERR unknown command\n").ok();
            }
        }
    }
}

fn start_cleaner(db: Db) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(CLEAN_INTERVAL));

        let mut db_guard = db.lock().unwrap();
        let now = SystemTime::now();
        let before = db_guard.len();

        db_guard.retain(|_, item| {
            if let Some(exp) = item.expires_at {
                exp > now
            } else {
                true
            }
        });

        let after = db_guard.len();
        let removed = before - after;

        if removed > 0 {
            EXPIRED_COUNTER.fetch_add(removed, Ordering::Relaxed);
        }
    });
}

fn start_monitor(db: Db) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));

        let db_guard = db.lock().unwrap();
        let items = db_guard.len();
        let memory: usize = db_guard.values()
            .map(|v| v.value.len() + 64)
            .sum();
        drop(db_guard);

        let hits = HIT_COUNTER.load(Ordering::Relaxed);
        let misses = MISS_COUNTER.load(Ordering::Relaxed);
        let total = hits + misses;
        let hit_ratio = if total > 0 { (hits as f64 / total as f64) * 100.0 } else { 0.0 };

        println!(
            "[MONITOR] items={}  memory={} bytes  expired={}  hits={}  misses={}  hit_ratio={:.1}%",
            items,
            memory,
            EXPIRED_COUNTER.load(Ordering::Relaxed),
            hits,
            misses,
            hit_ratio,
        );
    });
}

fn start_metrics_emitter(db: Db) {
    thread::spawn(move || {
        let client = reqwest::blocking::Client::new();
        
        loop {
            thread::sleep(Duration::from_secs(METRICS_INTERVAL));

            let db_guard = db.lock().unwrap();
            let items = db_guard.len();
            let memory: usize = db_guard.values()
                .map(|v| v.value.len() + 64)
                .sum();
            drop(db_guard);

            let hits = HIT_COUNTER.load(Ordering::Relaxed);
            let misses = MISS_COUNTER.load(Ordering::Relaxed);
            let total = hits + misses;
            let hit_ratio = if total > 0 { hits as f64 / total as f64 } else { 0.0 };

            let metrics = Metrics {
                timestamp: Utc::now().to_rfc3339(),
                hits,
                misses,
                hit_ratio,
                gets: GET_COUNTER.load(Ordering::Relaxed),
                sets: SET_COUNTER.load(Ordering::Relaxed),
                deletes: DEL_COUNTER.load(Ordering::Relaxed),
                expired: EXPIRED_COUNTER.load(Ordering::Relaxed),
                items,
                memory_bytes: memory,
            };

            let url = format!("{}/{}/_doc", get_es_url(), ES_INDEX);
            match client.post(&url)
                .header("Content-Type", "application/json")
                .json(&metrics)
                .send()
            {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        eprintln!("[ES] Failed to send metrics: {}", resp.status());
                    }
                }
                Err(e) => {
                    eprintln!("[ES] Error sending metrics: {}", e);
                }
            }
        }
    });
}

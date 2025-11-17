use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}},
    thread,
    time::{Duration, SystemTime},
};

use crossbeam_channel;

#[derive(Clone)]
struct Item {
    value: Vec<u8>,
    expires_at: Option<SystemTime>,
}

type Db = Arc<Mutex<HashMap<String, Item>>>;

const THREAD_POOL_SIZE: usize = 8;
const CLEAN_INTERVAL: u64 = 1;

// GLOBAL METRIC
static EXPIRED_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn main() {
    let db: Db = Arc::new(Mutex::new(HashMap::new()));

    start_cleaner(db.clone());
    start_monitor(db.clone());

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
                stream.write_all(b"OK\n").ok();
            }

            "GET" => {
                let key = parts.next().unwrap_or("").to_string();
                let mut db_guard = db.lock().unwrap();

                if let Some(item) = db_guard.get(&key) {
                    if let Some(exp) = item.expires_at {
                        if exp <= SystemTime::now() {
                            db_guard.remove(&key);
                            EXPIRED_COUNTER.fetch_add(1, Ordering::Relaxed);
                            stream.write_all(b"(expired)\n").ok();
                            continue;
                        }
                    }
                    let mut data = item.value.clone();
                    data.push(b'\n');
                    stream.write_all(&data).ok();
                } else {
                    stream.write_all(b"(nil)\n").ok();
                }
            }

            "DEL" => {
                let key = parts.next().unwrap_or("").to_string();
                let removed = db.lock().unwrap().remove(&key);
                if removed.is_some() { stream.write_all(b"1\n").ok(); }
                else { stream.write_all(b"0\n").ok(); }
            }

            "STATS" => {
                let db_guard = db.lock().unwrap();

                let items = db_guard.len();
                let memory: usize = db_guard.values()
                    .map(|v| v.value.len() + 64)
                    .sum();

                let msg = format!("items={} memory={}bytes expired={}\n",
                    items,
                    memory,
                    EXPIRED_COUNTER.load(Ordering::Relaxed),
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

        println!(
            "[MONITOR] items={}  memory={} bytes  expired={}",
            items,
            memory,
            EXPIRED_COUNTER.load(Ordering::Relaxed),
        );
    });
}

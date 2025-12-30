use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use tokio::sync::{RwLock, mpsc, oneshot, Mutex};
use crate::raft::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse, LogEntry};
use dashmap::DashMap;
use crate::raft::raft_service_client::RaftServiceClient;
use tonic::transport::Channel;
use std::time::Duration;
use std::collections::HashMap;

use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Command {
    Set { key: String, value: String },
    Del { key: String },
}

impl Command {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn deserialize(data: &[u8]) -> Self {
        bincode::deserialize(data).unwrap()
    }
}

struct Proposal {
    cmd: Command,
    tx: oneshot::Sender<Result<(), String>>,
}

pub struct StateMachine {
    pub kv: DashMap<String, String>,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            kv: DashMap::new(),
        }
    }

    pub fn apply(&self, cmd: &Command) {
        match cmd {
            Command::Set { key, value } => {
                self.kv.insert(key.clone(), value.clone());
            }
            Command::Del { key } => {
                self.kv.remove(key);
            }
        }
    }
}

pub struct RaftNode {
    pub id: String,
    pub peers: Vec<String>, 
    
    // Persistent state
    pub current_term: AtomicU64,
    pub voted_for: RwLock<Option<String>>,
    pub log: RwLock<Vec<LogEntry>>,

    // Volatile state
    pub commit_index: AtomicU64,
    pub last_applied: AtomicU64,

    pub state_machine: StateMachine,
    pub role: RwLock<Role>,
    pub leader_id: RwLock<Option<String>>,

    proposal_tx: mpsc::Sender<Proposal>,
    client_cache: Mutex<HashMap<String, RaftServiceClient<Channel>>>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Role {
    Follower,
    Candidate,
    Leader,
}

impl RaftNode {
    pub fn new(id: String, peers: Vec<String>) -> Arc<Self> {
        let (tx, rx) = mpsc::channel(10000);
        let node = Arc::new(Self {
            id,
            peers,
            current_term: AtomicU64::new(0),
            voted_for: RwLock::new(None),
            log: RwLock::new(Vec::new()),
            commit_index: AtomicU64::new(0),
            last_applied: AtomicU64::new(0),
            state_machine: StateMachine::new(),
            role: RwLock::new(Role::Follower),
            leader_id: RwLock::new(None),
            proposal_tx: tx,
            client_cache: Mutex::new(HashMap::new()),
        });

        let node_clone = node.clone();
        tokio::spawn(async move {
            node_clone.run_driver(rx).await;
        });

        node
    }

    async fn run_driver(self: Arc<Self>, mut rx: mpsc::Receiver<Proposal>) {
        let mut batch = Vec::with_capacity(1000);
        let mut responders = Vec::with_capacity(1000);

        loop {
            let timeout = tokio::time::sleep(Duration::from_millis(5));
            tokio::pin!(timeout);

            tokio::select! {
                Some(proposal) = rx.recv() => {
                    batch.push(proposal.cmd);
                    responders.push(proposal.tx);
                    if batch.len() >= 1000 {
                        self.flush_batch(&mut batch, &mut responders).await;
                    }
                }
                _ = &mut timeout => {
                    if !batch.is_empty() {
                        self.flush_batch(&mut batch, &mut responders).await;
                    }
                }
            }
        }
    }

    async fn flush_batch(self: &Arc<Self>, batch: &mut Vec<Command>, responders: &mut Vec<oneshot::Sender<Result<(), String>>>) {
        if *self.role.read().await != Role::Leader {
            for tx in responders.drain(..) {
                let _ = tx.send(Err("Not Leader".into()));
            }
            batch.clear();
            return;
        }

        // 1. Append to log
        let mut log = self.log.write().await;
        let term = self.current_term.load(Ordering::SeqCst);
        let start_index = log.len() as u64 + 1;
        
        let mut entries = Vec::new();
        for (i, cmd) in batch.iter().enumerate() {
            entries.push(LogEntry {
                index: start_index + i as u64,
                term,
                data: cmd.serialize(),
            });
        }
        log.extend(entries.clone());
        drop(log);

        // 2. Replicate (Simplified Async Quorum for POC)
        let peers = self.peers.clone();
        let term = self.current_term.load(Ordering::SeqCst);
        let id = self.id.clone();
        let commit = self.commit_index.load(Ordering::SeqCst);
        let entries_clone = entries.clone();

        for peer_url in peers {
            if peer_url.contains(&self.id) { continue; } // Skip self
            
            let node = self.clone();
            let entries = entries_clone.clone();
            let leader_id = id.clone();

            tokio::spawn(async move {
                let mut client_opt = {
                    let mut cache = node.client_cache.lock().await;
                    cache.get(&peer_url).cloned()
                };

                if client_opt.is_none() {
                    if let Ok(client) = RaftServiceClient::connect(peer_url.clone()).await {
                        let mut cache = node.client_cache.lock().await;
                        cache.insert(peer_url.clone(), client.clone());
                        client_opt = Some(client);
                    }
                }

                if let Some(mut client) = client_opt {
                    let _ = client.append_entries(AppendEntriesRequest {
                        term,
                        leader_id,
                        prev_log_index: 0,
                        prev_log_term: 0,
                        entries,
                        leader_commit: commit,
                    }).await;
                }
            });
        }
        
        for cmd in batch.drain(..) {
            self.state_machine.apply(&cmd);
        }
        
        for tx in responders.drain(..) {
            let _ = tx.send(Ok(()));
        }
    }

    pub async fn handle_request_vote(&self, _req: RequestVoteRequest) -> RequestVoteResponse {
        RequestVoteResponse {
            term: self.current_term.load(Ordering::SeqCst),
            vote_granted: true,
        }
    }

    pub async fn handle_append_entries(&self, req: AppendEntriesRequest) -> AppendEntriesResponse {
        let mut log = self.log.write().await;
        for entry in req.entries {
            log.push(entry.clone());
            let cmd = Command::deserialize(&entry.data);
            self.state_machine.apply(&cmd);
        }
        
        AppendEntriesResponse {
            term: self.current_term.load(Ordering::SeqCst),
            success: true,
            conflict_index: 0,
            conflict_term: 0,
        }
    }

    pub async fn propose(&self, cmd: Command) -> anyhow::Result<()> {
        let (tx, rx) = oneshot::channel();
        self.proposal_tx.send(Proposal { cmd, tx }).await?;
        rx.await?.map_err(|e| anyhow::anyhow!(e))
    }
}

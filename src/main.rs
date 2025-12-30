use anyhow::Result;
use axum::{
    routing::get,
    Router,
};
use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tonic::transport::Server;

mod api;
mod grpc;
mod raft;
mod raft_node;

use crate::api::AppState;
use crate::grpc::RaftServiceImpl;
use crate::raft::raft_service_server::RaftServiceServer;
use crate::raft_node::{RaftNode, Role};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// REST API Port
    #[arg(long, env = "HTTP_PORT", default_value_t = 8080)]
    http_port: u16,

    /// gRPC Port
    #[arg(long, env = "GRPC_PORT", default_value_t = 50051)]
    grpc_port: u16,

    /// Node ID (e.g., "node-1")
    #[arg(long, env = "NODE_ID", default_value = "node-1")]
    node_id: String,

    /// Peers (comma separated URIs, e.g., "http://node-2:50051,http://node-3:50051")
    #[arg(long, env = "PEERS", default_value = "")]
    peers: String,
    
    /// Initial Leader (for POC purposes)
    #[arg(long, env = "IS_LEADER", default_value_t = false)]
    is_leader: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Parse peers
    let mut peer_list: Vec<String> = args.peers
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    // Kubernetes Discovery
    if let Ok(svc_name) = std::env::var("K8S_SERVICE_NAME") {
        let namespace = std::env::var("K8S_NAMESPACE").unwrap_or_else(|_| "default".to_string());
        for i in 0..3 {
            let peer_uri = format!("http://{}-{}.{}.{}.svc.cluster.local:{}", svc_name, i, svc_name, namespace, args.grpc_port);
            if !peer_list.contains(&peer_uri) {
                peer_list.push(peer_uri);
            }
        }
    }

    let node = RaftNode::new(args.node_id.clone(), peer_list);
    
    if args.is_leader {
        *node.role.write().await = Role::Leader;
        println!("Node {} started as LEADER", args.node_id);
    } else {
        println!("Node {} started as FOLLOWER", args.node_id);
    }

    // 1. Start gRPC Server
    let grpc_addr = format!("0.0.0.0:{}", args.grpc_port).parse()?;
    let raft_service = RaftServiceImpl { node: node.clone() };
    
    println!("gRPC listening on {}", grpc_addr);
    let grpc_future = Server::builder()
        .add_service(RaftServiceServer::new(raft_service))
        .serve(grpc_addr);

    // 2. Start REST API
    let app_state = Arc::new(AppState { node: node.clone() });
    let app = Router::new()
        .route("/{key}", get(api::get_key).post(api::set_key).delete(api::delete_key))
        .with_state(app_state);

    let http_addr = SocketAddr::from(([0, 0, 0, 0], args.http_port));
    println!("HTTP listening on {}", http_addr);
    
    let listener = tokio::net::TcpListener::bind(http_addr).await?;
    let http_future = axum::serve(listener, app);

    // Run both
    tokio::select! {
        _ = grpc_future => println!("gRPC server exited"),
        _ = http_future => println!("HTTP server exited"),
        _ = signal::ctrl_c() => println!("Shutting down code"),
    }

    Ok(())
}

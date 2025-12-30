use std::time::Duration;
use tokio::time::sleep;
use reqwest::Client;
use std::process::{Command, Stdio};

#[tokio::test]
async fn test_cluster_integration() {
    // Compile debug build first
    let status = Command::new("cargo")
        .arg("build")
        .status()
        .expect("Failed to build");
    assert!(status.success());

    let exe_path = "target/debug/raft-kv.exe"; // Adjust extension for Windows

    // Spawn 3 nodes
    let mut children = Vec::new();
    let base_http = 8081;
    let base_grpc = 50051;
    
    // Peers string: http://127.0.0.1:50051,http://127.0.0.1:50052,http://127.0.0.1:50053
    // But our code currently uses the peer strings directly for connection?
    // In main.rs: "Peers (comma separated URIs, e.g., "http://node-2:50051...")"
    // And RaftNode connects to them. Tonic expects valid URIs.
    // We should use 127.0.0.1.
    
    let peer_1 = format!("http://127.0.0.1:{}", base_grpc);
    let peer_2 = format!("http://127.0.0.1:{}", base_grpc + 1);
    let peer_3 = format!("http://127.0.0.1:{}", base_grpc + 2);
    let all_peers = format!("{},{},{}", peer_1, peer_2, peer_3);

    for i in 0..3 {
        let http_port = base_http + i;
        let grpc_port = base_grpc + i;
        let id = format!("node-{}", i + 1);
        let mut cmd = Command::new(exe_path);
        cmd.arg("--http-port").arg(http_port.to_string())
           .arg("--grpc-port").arg(grpc_port.to_string())
           .arg("--node-id").arg(id)
           .arg("--peers").arg(&all_peers)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        if i == 0 {
            cmd.arg("--is-leader");
        }

        let child = cmd.spawn().expect("Failed to spawn node");
        
        children.push(child);
    }

    // Wait for startup
    sleep(Duration::from_secs(5)).await;

    let client = Client::new();

    // 1. Write to Leader (Node 1)
    let resp = client.post(format!("http://127.0.0.1:{}/user1", base_http))
        .json(&serde_json::json!({"value": "alice"}))
        .send()
        .await
        .expect("Failed to send request");
    
    assert_eq!(resp.status(), 200);

    // 2. Read from Follower (Node 2) - should eventually have data
    // In real Raft, follower might redirect or sync. Our simple implementation applies to local state machine on append.
    // So if replication worked, node 2 should have it.
    sleep(Duration::from_secs(1)).await;
    
    let resp = client.get(format!("http://127.0.0.1:{}/user1", base_http + 1))
        .send()
        .await
        .expect("Failed to get request");
    
    let body: serde_json::Value = resp.json().await.expect("Failed to parse json");
    assert_eq!(body["value"], "alice");

    // Cleanup
    for mut child in children {
        let _ = child.kill();
    }
}

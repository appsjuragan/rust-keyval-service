use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::raft_node::{RaftNode, Command};

pub struct AppState {
    pub node: Arc<RaftNode>,
}

#[derive(Serialize)]
struct GetResponse {
    value: Option<String>,
}

#[derive(Deserialize)]
pub struct SetRequest {
    value: String,
}

pub async fn get_key(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    let val = state.node.state_machine.kv.get(&key).map(|v| v.value().clone());
    Json(GetResponse { value: val })
}

pub async fn set_key(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
    Json(payload): Json<SetRequest>,
) -> impl IntoResponse {
    let cmd = Command::Set {
        key,
        value: payload.value,
    };

    match state.node.propose(cmd).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR, // TODO: Redirect to leader
    }
}

pub async fn delete_key(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    let cmd = Command::Del { key };
    match state.node.propose(cmd).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

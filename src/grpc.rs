use tonic::{Request, Response, Status};
use crate::raft::raft_service_server::RaftService;
use crate::raft::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse};
use std::sync::Arc;
use crate::raft_node::RaftNode;

pub struct RaftServiceImpl {
    pub node: Arc<RaftNode>,
}

#[tonic::async_trait]
impl RaftService for RaftServiceImpl {
    async fn request_vote(
        &self,
        request: Request<RequestVoteRequest>,
    ) -> Result<Response<RequestVoteResponse>, Status> {
        let req = request.into_inner();
        let reply = self.node.handle_request_vote(req).await;
        Ok(Response::new(reply))
    }

    async fn append_entries(
        &self,
        request: Request<AppendEntriesRequest>,
    ) -> Result<Response<AppendEntriesResponse>, Status> {
        let req = request.into_inner();
        let reply = self.node.handle_append_entries(req).await;
        Ok(Response::new(reply))
    }
}

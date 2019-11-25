use std::sync::Arc;
use std::thread;
use std::time::Duration;

use grpcio::{ChannelBuilder, EnvBuilder};
use log::*;
use raft::eraftpb::{ConfChange, ConfChangeType};

use crate::proto::indexpb_grpc::IndexClient;
use crate::proto::indexrpcpb::{
    ApplyReq, CommitReq, CommitResp, ConfChangeReq, DeleteReq, DeleteResp, GetReq, GetResp,
    MetricsReq, MetricsResp, PeersReq, PeersResp, PutReq, PutResp, RaftDone, ReqType, RespErr,
    RollbackReq, RollbackResp, SchemaReq, SchemaResp, SearchReq, SearchResp,
};

pub fn create_client(addr: &str) -> IndexClient {
    let env = Arc::new(EnvBuilder::new().build());
    let ch = ChannelBuilder::new(env).connect(&addr);
    debug!("create index client for {}", addr);
    IndexClient::new(ch)
}

pub struct Clerk {
    servers: Vec<IndexClient>,
    client_id: u64,
    request_seq: u64,
    leader_id: usize,
}

impl Clerk {
    pub fn new(servers: &Vec<IndexClient>, client_id: u64) -> Clerk {
        Clerk {
            servers: servers.clone(),
            client_id,
            request_seq: 0,
            leader_id: 0,
        }
    }

    pub fn join(&mut self, id: u64, ip: &str, port: u16) {
        let mut cc = ConfChange::new();
        cc.set_id(id);
        cc.set_node_id(id);
        cc.set_change_type(ConfChangeType::AddNode);

        let mut cc_req = ConfChangeReq::new();
        cc_req.set_cc(cc);
        cc_req.set_ip(ip.to_string());
        cc_req.set_port(port as u32);

        loop {
            let reply = self.servers[self.leader_id]
                .raft_conf_change(&cc_req)
                .unwrap_or_else(|e| {
                    error!("{:?}", e);
                    let mut resp = RaftDone::new();
                    resp.set_err(RespErr::ErrWrongLeader);
                    resp
                });
            match reply.err {
                RespErr::OK => return,
                RespErr::ErrWrongLeader => (),
                RespErr::ErrNoKey => return,
            }
            self.leader_id = (self.leader_id + 1) % self.servers.len();
            thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn leave(&mut self, id: u64) {
        let mut cc = ConfChange::new();
        cc.set_id(id);
        cc.set_node_id(id);
        cc.set_change_type(ConfChangeType::RemoveNode);
        let mut cc_req = ConfChangeReq::new();
        cc_req.set_cc(cc);

        loop {
            let reply = self.servers[self.leader_id]
                .raft_conf_change(&cc_req)
                .unwrap_or_else(|e| {
                    error!("{:?}", e);
                    let mut resp = RaftDone::new();
                    resp.set_err(RespErr::ErrWrongLeader);
                    resp
                });
            match reply.err {
                RespErr::OK => return,
                RespErr::ErrWrongLeader => (),
                RespErr::ErrNoKey => return,
            }
            self.leader_id = (self.leader_id + 1) % self.servers.len();
            thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn peers(&mut self) -> String {
        let mut req = PeersReq::new();
        req.set_client_id(self.client_id);
        req.set_seq(self.request_seq);
        self.request_seq += 1;

        loop {
            let reply = self.servers[self.leader_id]
                .peers(&req)
                .unwrap_or_else(|_e| {
                    let mut resp = PeersResp::new();
                    resp.set_err(RespErr::ErrWrongLeader);
                    resp
                });
            match reply.err {
                RespErr::OK => return reply.value,
                RespErr::ErrWrongLeader => (),
                RespErr::ErrNoKey => return String::from(""),
            }
            self.leader_id = (self.leader_id + 1) % self.servers.len();
            thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn metrics(&mut self) -> String {
        let mut req = MetricsReq::new();
        req.set_client_id(self.client_id);
        req.set_seq(self.request_seq);
        self.request_seq += 1;

        loop {
            let reply = self.servers[self.leader_id]
                .metrics(&req)
                .unwrap_or_else(|_e| {
                    let mut resp = MetricsResp::new();
                    resp.set_err(RespErr::ErrWrongLeader);
                    resp
                });
            match reply.err {
                RespErr::OK => return reply.value,
                RespErr::ErrWrongLeader => (),
                RespErr::ErrNoKey => return String::from(""),
            }
            self.leader_id = (self.leader_id + 1) % self.servers.len();
            thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn get(&mut self, doc_id: &str) -> String {
        let mut req = GetReq::new();
        req.set_client_id(self.client_id);
        req.set_seq(self.request_seq);
        req.set_doc_id(doc_id.to_owned());
        self.request_seq += 1;

        loop {
            let reply = self.servers[self.leader_id].get(&req).unwrap_or_else(|_e| {
                let mut resp = GetResp::new();
                resp.set_err(RespErr::ErrWrongLeader);
                resp
            });
            match reply.err {
                RespErr::OK => return reply.value,
                RespErr::ErrWrongLeader => (),
                RespErr::ErrNoKey => return String::from(""),
            }
            self.leader_id = (self.leader_id + 1) % self.servers.len();
            thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn put(&mut self, key: &str, value: &str) {
        let mut put_req = PutReq::new();
        put_req.set_client_id(self.client_id);
        put_req.set_seq(self.request_seq);
        put_req.set_doc_id(key.to_owned());
        put_req.set_fields(value.to_owned());

        let mut req = ApplyReq::new();
        req.set_client_id(self.client_id);
        req.set_req_type(ReqType::Put);
        req.set_put_req(put_req);

        self.request_seq += 1;

        loop {
            let reply = self.servers[self.leader_id].put(&req).unwrap_or_else(|_e| {
                let mut resp = PutResp::new();
                resp.set_err(RespErr::ErrWrongLeader);
                resp
            });
            match reply.err {
                RespErr::OK => return,
                RespErr::ErrWrongLeader => (),
                RespErr::ErrNoKey => return,
            }
            debug!("put redo: {}", self.leader_id);
            self.leader_id = (self.leader_id + 1) % self.servers.len();
            thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn delete(&mut self, key: &str) {
        let mut delete_req = DeleteReq::new();
        delete_req.set_client_id(self.client_id);
        delete_req.set_seq(self.request_seq);
        delete_req.set_doc_id(key.to_owned());

        let mut req = ApplyReq::new();
        req.set_client_id(self.client_id);
        req.set_req_type(ReqType::Delete);
        req.set_delete_req(delete_req);

        self.request_seq += 1;

        loop {
            let reply = self.servers[self.leader_id]
                .delete(&req)
                .unwrap_or_else(|_e| {
                    let mut resp = DeleteResp::new();
                    resp.set_err(RespErr::ErrWrongLeader);
                    resp
                });
            match reply.err {
                RespErr::OK => return,
                RespErr::ErrWrongLeader => (),
                RespErr::ErrNoKey => return,
            }
            self.leader_id = (self.leader_id + 1) % self.servers.len();
            thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn commit(&mut self) {
        let mut commit_req = CommitReq::new();
        commit_req.set_client_id(self.client_id);
        commit_req.set_seq(self.request_seq);

        let mut req = ApplyReq::new();
        req.set_client_id(self.client_id);
        req.set_req_type(ReqType::Commit);
        req.set_commit_req(commit_req);

        self.request_seq += 1;

        loop {
            let reply = self.servers[self.leader_id]
                .commit(&req)
                .unwrap_or_else(|_e| {
                    let mut resp = CommitResp::new();
                    resp.set_err(RespErr::ErrWrongLeader);
                    resp
                });
            match reply.err {
                RespErr::OK => return,
                RespErr::ErrWrongLeader => (),
                RespErr::ErrNoKey => return,
            }
            self.leader_id = (self.leader_id + 1) % self.servers.len();
            thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn rollback(&mut self) {
        let mut rollback_req = RollbackReq::new();
        rollback_req.set_client_id(self.client_id);
        rollback_req.set_seq(self.request_seq);

        let mut req = ApplyReq::new();
        req.set_client_id(self.client_id);
        req.set_req_type(ReqType::Rollback);
        req.set_rollback_req(rollback_req);

        self.request_seq += 1;

        loop {
            let reply = self.servers[self.leader_id]
                .rollback(&req)
                .unwrap_or_else(|_e| {
                    let mut resp = RollbackResp::new();
                    resp.set_err(RespErr::ErrWrongLeader);
                    resp
                });
            match reply.err {
                RespErr::OK => return,
                RespErr::ErrWrongLeader => (),
                RespErr::ErrNoKey => return,
            }
            self.leader_id = (self.leader_id + 1) % self.servers.len();
            thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn search(&mut self, query: &str) -> String {
        let mut req = SearchReq::new();
        req.set_client_id(self.client_id);
        req.set_seq(self.request_seq);
        req.set_query(query.to_owned());
        self.request_seq += 1;

        loop {
            let reply = self.servers[self.leader_id]
                .search(&req)
                .unwrap_or_else(|_e| {
                    let mut resp = SearchResp::new();
                    resp.set_err(RespErr::ErrWrongLeader);
                    resp
                });
            match reply.err {
                RespErr::OK => return reply.value,
                RespErr::ErrWrongLeader => (),
                RespErr::ErrNoKey => return String::from(""),
            }
            self.leader_id = (self.leader_id + 1) % self.servers.len();
            thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn schema(&mut self) -> String {
        let mut req = SchemaReq::new();
        req.set_client_id(self.client_id);
        req.set_seq(self.request_seq);
        self.request_seq += 1;

        loop {
            let reply = self.servers[self.leader_id]
                .schema(&req)
                .unwrap_or_else(|_e| {
                    let mut resp = SchemaResp::new();
                    resp.set_err(RespErr::ErrWrongLeader);
                    resp
                });
            match reply.err {
                RespErr::OK => return reply.value,
                RespErr::ErrWrongLeader => (),
                RespErr::ErrNoKey => return String::from(""),
            }
            self.leader_id = (self.leader_id + 1) % self.servers.len();
            thread::sleep(Duration::from_millis(100));
        }
    }
}

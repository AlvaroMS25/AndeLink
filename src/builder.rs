use std::sync::Arc;
use crate::{
    cluster::Cluster,
    node::UniversalNode
};

pub struct NodeBuilder {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) ssl: bool,
    pub(crate) pass: String,
    pub(crate) shards: u64,
    pub(crate) id: Option<u64>,
    pub(crate) node_id: Option<u8>
}

impl Default for NodeBuilder {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 2333,
            ssl: false,
            pass: "youshallnotpass".to_string(),
            shards: 1,
            id: None,
            node_id: None
        }
    }
}

impl NodeBuilder {
    pub fn set_host(&mut self, host: impl ToString) -> &mut Self {
        self.host = host.to_string();
        self
    }

    pub fn set_port(&mut self, port: u16) -> &mut Self {
        self.port = port;
        self
    }

    pub fn set_password(&mut self, password: impl ToString) -> &mut Self {
        self.pass = password.to_string();
        self
    }

    pub fn set_shards(&mut self, shards: u64) -> &mut Self {
        self.shards = shards;
        self
    }

    pub fn set_user_id(&mut self, id: impl Into<u64>) -> &mut Self {
        self.id = Some(id.into());
        self
    }

    pub fn set_ssl(&mut self, ssl: bool) -> &mut Self {
        self.ssl = ssl;
        self
    }

    pub(crate) fn build(mut self, cluster: Arc<Cluster>, node_id: u8) -> Arc<UniversalNode> {
        self.node_id = Some(node_id);

        assert!(self.node_id.is_some());
        assert!(self.id.is_some());

        UniversalNode::new(cluster, self)
    }
}
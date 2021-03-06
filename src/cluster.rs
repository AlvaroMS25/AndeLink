use std::sync::Arc;
use tokio::sync::RwLock;
use parking_lot::Mutex;
use typemap_rev::TypeMap;
use crate::{builder::NodeBuilder, events::EventHandler, node::UniversalNode};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::collections::HashMap;
use crate::error::{ClusterResult, ClusterError};

#[non_exhaustive]
pub struct Cluster {
    pub event_handler: Arc<dyn EventHandler>,
    pub nodes: DashMap<u8, Arc<UniversalNode>>,
    pub reconnect_attempts: u8,
    pub shared_data: Arc<RwLock<TypeMap>>,
    pub node_counter: AtomicU8,
    pub self_ref: Mutex<Option<Arc<Self>>>
}

impl Cluster {
    pub fn builder<H: EventHandler + 'static>(handler: H) -> ClusterBuilder {
        ClusterBuilder::new(handler)
    }
    fn new(builder: ClusterBuilder) -> ClusterResult<Arc<Self>> {
        let cluster = Arc::new(Self {
            event_handler: builder.event_handler,
            nodes: DashMap::new(),
            reconnect_attempts: builder.reconnect_attempts,
            shared_data: Arc::new(RwLock::new(builder.data)),
            node_counter: AtomicU8::new(0),
            self_ref: Mutex::new(None)
        });

        let clone = Arc::clone(&cluster);

        *cluster.self_ref.lock() = Some(clone);

        for node in builder.nodes {
            cluster.add_node(|f| {
                *f = node;
                f
            })?;
        }

        Ok(cluster)
    }

    pub async fn get_best(&self) -> ClusterResult<Arc<UniversalNode>>{
        //get the best node based on number of player every node has, the node with fewer players gets returned

        if self.nodes.len() == 1 {
            let node = self.nodes.iter().next().map(|item| Arc::clone(&item.value()));
            
            if node.is_none() { return Err(ClusterError::CannotFindNode) }

            return Ok(node.unwrap())
        }

        let mut counts: HashMap<u8, u32> = HashMap::new();
        let mut min: Option<(u8, u32)> = None;
        for item in self.nodes.iter() {
            counts.insert(item.key().clone(), item.value().read().await.players.len() as u32);
        }

        for (id, i) in counts.iter() {
            if min.is_none() { min = Some((id.clone(), i.clone())) }
            else {
                if let Some((_, r)) = min {
                    if i < &r {
                        min = Some((id.clone(), i.clone()));
                    }
                }
            }
        }

        if min.is_none() { return Err(ClusterError::CannotFindBestNode); }

        let nodes = &self.nodes;
        let best_node = nodes.get(&min.unwrap().0);

        if best_node.is_none() { return Err(ClusterError::CannotFindBestNode); }

        let node = Arc::clone(&best_node.unwrap());

        Ok(node)
    }

    pub async fn get_player_node(&self, guild: impl Into<u64>) -> ClusterResult<Arc<UniversalNode>> {
        //get the node iterating over cluster's nodes and checking if player loops contains a certain guild id
        let guild = guild.into();
        for iter in self.nodes.iter() {
            if iter.value().read().await.players.contains_key(&guild) {
                return Ok(Arc::clone(iter.value()))
            }
        }

        Err(ClusterError::CannotFindNode)
    }

    fn get_id(&self) -> u8 {
        let mut count = self.node_counter.fetch_add(1, Ordering::Relaxed);
        count += 1;
        count
    }

    pub fn add_node<F>(&self, func: F) -> ClusterResult<()>
    where
        F: FnOnce(&mut NodeBuilder) -> &mut NodeBuilder
    {
        let mut builder = NodeBuilder::default();

        func(&mut builder);

        let id = self.get_id();

        let cluster = {
            let lock = self.self_ref.lock();

            if let Some(c) = &*lock {
                Arc::clone(&c)
            } else {
                return Err(ClusterError::MissingSelfRef)
            }
        };

        let node = builder.build(cluster, id);

        UniversalNode::run(node);

        Ok(())
    }
}

impl typemap_rev::TypeMapKey for Cluster {
    type Value = Arc<Cluster>;
}

pub struct ClusterBuilder {
    pub event_handler: Arc<dyn EventHandler>,
    pub nodes: Vec<NodeBuilder>,
    pub data: TypeMap,
    pub reconnect_attempts: u8
}

impl ClusterBuilder {
    pub fn new<H: EventHandler + 'static>(handler: H) -> Self {
        Self {
            event_handler: Arc::new(handler),
            nodes: Vec::new(),
            data: TypeMap::new(),
            reconnect_attempts: 5
        }
    }

    pub fn add_node<F>(&mut self, func: F) -> &mut Self
    where
        F: FnOnce(&mut NodeBuilder) -> &mut NodeBuilder
    {
            let mut builder = NodeBuilder::default();

            func(&mut builder);

            self.nodes.push(builder);

            self
    }

    pub fn reconnect_attempts(&mut self, attempts: u8) -> &mut Self {
        self.reconnect_attempts = attempts;

        self
    }

    pub fn data_ref(&self) -> &TypeMap {
        &self.data
    }

    pub fn build(self) -> ClusterResult<Arc<Cluster>> {
        Cluster::new(self)
    }
}
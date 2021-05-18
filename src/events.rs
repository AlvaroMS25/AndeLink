use async_trait::async_trait;

use crate::{
    node::UniversalNode,
    model::gateway::*,
};
use std::sync::Arc;

#[async_trait]
pub trait EventHandler: Send + Sync + 'static {
    /// Periodic event that returns the statistics of the server.
    async fn stats(&self, _node: Arc<UniversalNode>, _event: Stats) {}
    /// Event that triggers when a player updates.
    async fn player_update(&self, _node: Arc<UniversalNode>, _event: PlayerUpdate) {}
    /// Event that triggers when a track starts playing.
    async fn track_start(&self, _node: Arc<UniversalNode>, _event: TrackStart) {}
    /// Event that triggers when a track finishes playing.
    async fn track_finish(&self, _node: Arc<UniversalNode>, _event: TrackFinish) {}
    /*///Event triggered when track gets stuck **NOT WORKING**
    async fn track_stuck(&self, _node: Arc<UniversalNode>) {}
    ///Event triggered when there is an exception playing the track **NOT WORKING**
    async fn track_exception(&self, _node: Arc<UniversalNode>) {}*/
    ///Event triggered when an audio web socket is disconnected from discord
    async fn socket_closed(&self, _node: Arc<UniversalNode>, _event: WebSocketClosed) {}
}

pub(crate) fn process(node: Arc<UniversalNode>, handler: Arc<dyn EventHandler>, event_type: EventType) {
    match event_type {
        EventType::Stats(e) => {
            tokio::spawn(async move {
                handler.stats(node, e).await;
            });
        },
        EventType::PlayerUpdate(e) => {
            tokio::spawn(async move {
                handler.player_update(node, e).await;
            });
        },
        EventType::TrackStart(e) => {
            tokio::spawn(async move {
                handler.track_start(node, e).await;
            });
        },
        EventType::TrackFinish(e) => {
            tokio::spawn(async move {
                handler.track_finish(node, e).await;
            });
        },
        EventType::WebSocketClosed(e) => {
            tokio::spawn(async move {
                handler.socket_closed(node, e).await;
            });
        }
    }
}

pub(crate) enum EventType {
    Stats(Stats),
    PlayerUpdate(PlayerUpdate),
    TrackStart(TrackStart),
    TrackFinish(TrackFinish),
    WebSocketClosed(WebSocketClosed)
}
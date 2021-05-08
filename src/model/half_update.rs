use serenity::{
    model::{
        voice::VoiceState,
        event::VoiceServerUpdateEvent,
        id::GuildId
    },
};

use tracing::info;
use std::sync::Arc;
use crate::node::UniversalNode;
use crate::model::events::{Event, VoiceUpdate};
use crate::error::{
    ClusterError,
    AndelinkError,
    AndelinkResult,
    ClusterResult
};

#[derive(Debug, Clone)]
pub enum HalfVoiceUpdate {
    Server(VoiceServerUpdateEvent),
    State(VoiceState)
}

impl HalfVoiceUpdate {
    pub async fn process(self, node: Arc<UniversalNode>) -> ClusterResult<()> {
        let node_read = node.read().await;

        let guild_id = match &self {
            HalfVoiceUpdate::Server(e) => e.guild_id,
            HalfVoiceUpdate::State(e) => {
                let client_id = node_read.id;

                if e.user_id != client_id {
                    return Ok(())
                }

                e.guild_id
            }
        };

        let guild_id = match guild_id {
            Some(id) => id,
            None => return Ok(())
        };

        info!("Processing HalfVoiceUpdate event: {:?}", &self);

        let update = {
            let existing_half = match node_read.waiting.get(&guild_id) {
                Some(half) => half.value().clone(),
                None => {
                    info!(
                        "guild {} is now waiting for other half; got: {:?}",
                        guild_id,
                        &self
                    );

                    node_read.waiting.insert(guild_id, self);

                    return Ok(());
                }
            };

            info!(
                "got both halves for {}: {:?}; {:?}",
                guild_id,
                &self,
                existing_half
            );

            match (existing_half, self) {
                (HalfVoiceUpdate::State(_), HalfVoiceUpdate::State(state)) => {
                    // Just like above, we got the same half twice...
                    info!(
                        "got the same state half twice for guild {}: {:?}",
                        guild_id,
                        state
                    );
                    node_read
                        .waiting
                        .insert(guild_id, HalfVoiceUpdate::State(state));

                    return Ok(());
                }
                (HalfVoiceUpdate::State(ref state), HalfVoiceUpdate::Server(ref server)) => {

                    node_read
                        .waiting
                        .remove(&guild_id);

                    let event = Event {
                        token: server.token.clone(),
                        endpoint: if let Some(e) = server.endpoint.clone() {e} else {return Err(ClusterError::CannotUpdateVoiceState)},
                        guild_id: guild_id.0.to_string()
                    };

                    VoiceUpdate {
                        session_id: state.session_id.clone(),
                        event
                    }
                },
                (HalfVoiceUpdate::Server(_), HalfVoiceUpdate::Server(server)) => {
                    // We got the same half twice... weird, but let's just replace
                    // the existing one.
                    info!(
                        "got the same server half twice for guild {}: {:?}",
                        guild_id,
                        server
                    );
                    node_read
                        .waiting
                        .insert(guild_id, HalfVoiceUpdate::Server(server));

                    return Ok(());
                }
                (HalfVoiceUpdate::Server(ref server), HalfVoiceUpdate::State(ref state)) => {

                    node_read
                        .waiting
                        .remove(&guild_id);

                    let event = Event {
                        token: server.token.clone(),
                        endpoint: if let Some(e) = server.endpoint.clone() {e} else {return Err(ClusterError::CannotUpdateVoiceState)},
                        guild_id: guild_id.0.to_string()
                    };

                    VoiceUpdate {
                        session_id: state.session_id.clone(),
                        event
                    }
                }
            }
        };

        drop(node_read);

        info!("sending voice update for guild {}: {:?}", guild_id, update);

        send(node.clone(), guild_id, update).await?;

        Ok(())
    }
}

async fn send(node: Arc<UniversalNode>, guild_id: GuildId, payload: VoiceUpdate) -> AndelinkResult<()> {
    let mut node_write = node.write().await;

    if let Some(socket) = &mut node_write.socket_write {
        crate::model::Codes::VoiceUpdate(payload).send(guild_id, socket).await
    } else {
        return Err(AndelinkError::NoWebsocket);
    }
}
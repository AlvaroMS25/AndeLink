pub mod events;
pub mod track;
pub mod gateway;
pub mod play_parameters;
pub mod player;
pub mod half_update;

use serde::{Serialize, Deserialize};

use events::*;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use crate::types::WebSocketConnection;
use serenity::model::id::GuildId as DiscordGuildId;
use serde_json::{
    json,
    Value
};
use crate::error::{AndelinkError, AndelinkResult};
use futures::SinkExt;

pub fn merge(a: &mut Value, b: Value) {
    match (a, b) {
        (a @ &mut Value::Object(_), Value::Object(b)) => {
            let a = a.as_object_mut().unwrap();
            for (k, v) in b {
                merge(a.entry(k).or_insert(Value::Null), v);
            }
        }

        (a, b) => *a = b,
    }
}


#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "camelCase")]
pub enum Codes {
    //Destroy the player
    Destroy,
    //Equalize the player
    Equalize(Equalize),
    //Pause the player
    Pause(Pause),
    //Play a track
    Play(Play),
    //Seek to a given position
    Seek(Seek),
    //Stop a player
    Stop,
    //Player connects to a given channel
    VoiceUpdate(VoiceUpdate),
    //Updates position information
    PlayerUpdate(PlayerUpdate),
    //Change the player's volume
    Volume(Volume)
}

impl Codes {
    pub async fn send(&self, guild_id: impl Into<DiscordGuildId>, socket: &mut WebSocketConnection) -> AndelinkResult<()>{
        let value = match self {
            Self::Destroy => {
                json!({
                    "op" : self,
                    "guildId" : &guild_id.into().0.to_string()
                })
            },
            Self::Stop => {
                json!({
                    "op" : self,
                    "guildId" : &guild_id.into().0.to_string()
                })
            },
            Self::Seek(data) => {
                let mut x = json!({
                    "op" : "seek",
                    "guildId" : &guild_id.into().0.to_string(),
                });
                merge(&mut x, serde_json::to_value(data).unwrap());
                x
            },
            Self::Pause(data) => {
                let mut x = json!({
                    "op" : "pause",
                    "guildId" : &guild_id.into().0.to_string(),
                });
                merge(&mut x, serde_json::to_value(data).unwrap());
                x
            },
            Self::Play(data) => {
                let mut x = json!({
                    "op" : "play",
                    "guildId" : &guild_id.into().0.to_string(),
                });
                merge(&mut x, serde_json::to_value(data).unwrap());
                x
            },
            Self::VoiceUpdate(data) => {
                let mut x = json!({
                    "op" : "voiceUpdate",
                    "guildId" : &guild_id.into().0.to_string(),
                });
                merge(&mut x, serde_json::to_value(data).unwrap());
                x
            },
            Self::Volume(data) => {
                let mut x = json!({
                    "op" : "volume",
                    "guildId" : &guild_id.into().0.to_string(),
                });
                merge(&mut x, serde_json::to_value(data).unwrap());
                x
            },
            Self::Equalize(data) => {
                let mut x = json!({
                    "op" : "equalizer",
                    "guildId" : &guild_id.into().0.to_string(),
                });
                merge(&mut x, serde_json::to_value(data).unwrap());
                x
            },
            Self::PlayerUpdate(data) => {
                let mut x = json!({
                    "op" : "playerUpdate",
                    "guildId" : &guild_id.into().0.to_string(),
                });
                merge(&mut x, serde_json::to_value(data).unwrap());
                x
            }
        };

        let payload = serde_json::to_string(&value).unwrap();

        {
            if let Err(why) = socket.send(TungsteniteMessage::text(&payload)).await {
                return Err(AndelinkError::ErrorSendingPayload(why));
            };
        }

        Ok(())
    }
}
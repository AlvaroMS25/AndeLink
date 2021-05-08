use serde::{Serialize, Deserialize};
use serde_aux::prelude::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GatewayEvent {
    pub op: String,
    #[serde(rename = "type")]
    pub event_type: Option<String>
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RawEvent {
    #[serde(rename = "playingPlayers")]
    pub playing_players: Option<i64>,
    pub op: String,
    pub memory: Option<Memory>,
    #[serde(rename = "frameStats")]
    pub frame_stats: Option<FrameStats>,
    pub players: Option<i64>,
    pub cpu: Option<Cpu>,
    pub uptime: Option<i64>,
    pub state: Option<State>,
    #[serde(rename = "guildId")]
    pub guild_id: Option<String>,
    #[serde(rename = "type")]
    pub raw_event_type: Option<String>,
    pub track: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Cpu {
    pub cores: i64,
    #[serde(rename = "systemLoad")]
    pub system_load: f64,
    #[serde(rename = "lavalinkLoad")]
    pub lavalink_load: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FrameStats {
    pub sent: i64,
    pub deficit: i64,
    pub nulled: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Memory {
    pub reservable: i64,
    pub used: i64,
    pub free: i64,
    pub allocated: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct State {
    pub position: u64,
    pub time: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Stats {
    #[serde(rename = "playingPlayers")]
    pub playing_players: i64,
    pub op: String,
    pub memory: Memory,
    #[serde(rename = "frameStats")]
    pub frame_stats: Option<FrameStats>,
    pub players: i64,
    pub cpu: Cpu,
    pub uptime: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayerUpdate {
    pub op: String,
    pub state: State,
    #[serde(rename = "guildId")]
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub guild_id: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackStart {
    pub op: String,
    #[serde(rename = "type")]
    pub track_start_type: String,
    pub track: String,
    #[serde(rename = "guildId")]
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub guild_id: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackFinish {
    pub op: String,
    pub reason: String,
    #[serde(rename = "type")]
    pub track_finish_type: String,
    pub track: String,
    #[serde(rename = "guildId")]
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub guild_id: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WebSocketClosed {
    pub op: String,
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    #[serde(rename = "guildId")]
    pub guild_id: Option<String>,
    pub code: u16,
    pub reason: String,
    #[serde(rename = "byRemote")]
    pub by_remote: bool
}
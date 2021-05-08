use serde::{
    Serialize,
    Deserialize
};
use serenity::model::id::{UserId as DiscordUserId, ChannelId as DiscordChannelId};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Tracks {
    pub playlist_info: PlaylistInfo,
    pub load_type: String,
    pub tracks: Vec<Track>,
    pub exception: Option<Exception>
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
pub struct Track {
    pub track: String,
    pub info: Option<TrackInfo>
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    pub identifier: String,
    pub is_seekable: bool,
    pub author: String,
    pub length: u64,
    pub is_stream: bool,
    pub position: u64,
    pub title: String,
    pub uri: String
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Exception {
    pub message: String,
    pub severity: String
}

//#[serde(rename_all = "camelCase")]

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
pub struct PlaylistInfo {
    pub name: Option<String>,
    #[serde(rename = "selectedTrack")]
    pub selected_track: Option<i64>
}

#[derive(Clone, Debug, Default)]
pub struct QueuedTrack {
    pub track: Track,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub requester: Option<TrackRequester>,
    pub channel: Option<DiscordChannelId>
}

#[derive(Debug, Clone)]
pub enum TrackSearch<'a> {
    Youtube(&'a str),
    Url(&'a str)
}

impl std::fmt::Display for TrackSearch<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Youtube(query) => write!(f, "ytsearch:{}", query),
            Self::Url(url) => write!(f, "{}", url)
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrackRequester {
    pub id: Option<DiscordUserId>,
    pub name: Option<String>
}

impl From<DiscordUserId> for TrackRequester {
    fn from(id: DiscordUserId) -> TrackRequester {
        TrackRequester{id: Some(id), name: None}
    }
}

impl From<String> for TrackRequester {
    fn from(name: String) -> TrackRequester {
        TrackRequester{id: None, name: Some(name)}
    }
}

impl From<(DiscordUserId, String)> for TrackRequester {
    fn from(data: (DiscordUserId, String)) -> TrackRequester {
        TrackRequester {id: Some(data.0), name: Some(data.1)}
    }
}

impl From<(u64, String)> for TrackRequester {
    fn from(data: (u64, String)) -> TrackRequester {
        TrackRequester{id: Some(DiscordUserId::from(data.0)), name: Some(data.1)}
    }
}
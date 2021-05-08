use serenity::{
    model::id::GuildId as DiscordGuildId
};
use super::track::QueuedTrack;

#[derive(Clone)]
pub struct Player {
    pub guild: DiscordGuildId,
    pub now_playing: Option<QueuedTrack>,
    pub paused: bool,
    pub volume: u16,
    pub queue: Vec<QueuedTrack>
}

impl Default for Player {
    fn default() -> Self {
        Self {
            guild: DiscordGuildId(0),
            now_playing: None,
            paused: false,
            volume: 100,
            queue: vec![]
        }
    }
}
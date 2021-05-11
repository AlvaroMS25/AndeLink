use serenity::{
    model::id::ChannelId as DiscordChannelId
};
use super::track::Track;
use crate::error::{AndelinkResult, AndelinkError};
use std::{
    time::Duration,
};
use crate::node::UniversalNode;
use crate::error::AndelinkError::PlayerNotFound;
use super::track::TrackRequester;

pub struct PlayParameters<'a> {
    node: &'a UniversalNode,
    pub track: Track,
    pub replace: bool,
    pub start: u64,
    pub finish: u64,
    pub guild_id: u64,
    pub requester: Option<TrackRequester>,
    pub channel: Option<DiscordChannelId>
}

impl<'a> PlayParameters<'a> {

    //Create a new play parameters object
    pub fn default(node: &'a UniversalNode) -> Self {
        Self {
            node,
            track: Default::default(),
            replace: Default::default(),
            start: Default::default(),
            finish: Default::default(),
            guild_id: Default::default(),
            requester: Default::default(),
            channel: Default::default()
        }
    }

    /// Starts playing the track.
    pub async fn start(self) -> AndelinkResult<()> {
        let mut write = self.node.write().await;

        let socket = if let Some(stream) = &mut write.socket_write { stream } else { return Err(AndelinkError::NoWebsocket) };

        let payload = crate::model::events::Play {
            track: self.track.track,
            no_replace: !self.replace,
            start_time: self.start,
            end_time: if self.finish == 0 { None } else { Some(self.finish) },
        };

        crate::model::Codes::Play(payload).send(self.guild_id, socket).await?;


        Ok(())
    }

    pub async fn queue(self) -> AndelinkResult<()> {
        let should_start = {
            let node_read = self.node.read().await;
            if let Some(player) = node_read.players.get(&self.guild_id) {
                player.now_playing.is_none() && player.queue.len() == 0
            } else {
                false
            }
        };

        let track = crate::model::track::QueuedTrack {
            track: self.track,
            start_time: self.start,
            end_time: if self.finish == 0 { None } else { Some(self.finish) },
            requester: self.requester,
            channel: self.channel
        };

        let mut node_write = self.node.write().await;

        if let Some(player) = node_write.players.get_mut(&self.guild_id) {
            player.queue.push(track);
        } else {
            return Err(PlayerNotFound);
        }

        if should_start {
            node_write.play_next(self.guild_id).await?;
        }

        Ok(())
    }

    /// Sets the person that requested the song
    pub fn requester(mut self, requester: impl Into<TrackRequester>) -> Self {
        self.requester = Some(requester.into());
        self
    }

    /// Sets if the current playing track should be replaced with this new one.
    pub fn replace(mut self, replace: bool) -> Self {
        self.replace = replace;
        self
    }

    /// Sets the time the track will start at.
    pub fn start_time(mut self, start: Duration) -> Self {
        self.start = start.as_millis() as u64;
        self
    }

    /// Sets the time the track will finish at.
    pub fn finish_time(mut self, finish: Duration) -> Self {
        self.finish = finish.as_millis() as u64;
        self
    }

    /// Sets the channel where the song was requested
    pub fn channel(mut self, channel: impl Into<DiscordChannelId>) -> Self {
        self.channel = Some(channel.into());

        self
    }

    /// Sets the person that requested the song taking a mutable reference
    pub fn requester_ref(&mut self, requester: impl Into<TrackRequester>) -> &mut Self {
        self.requester = Some(requester.into());
        self
    }

    /// Sets if the current playing track should be replaced with this new one taking a mutable reference
    pub fn replace_ref(&mut self, replace: bool) -> &mut Self {
        self.replace = replace;
        self
    }

    /// Sets the time the track will start at taking a mutable reference
    pub fn start_time_ref(&mut self, start: Duration) -> &mut Self {
        self.start = start.as_millis() as u64;
        self
    }

    /// Sets the time the track will finish at taking a mutable reference
    pub fn finish_time_ref(&mut self, finish: Duration) -> &mut Self {
        self.finish = finish.as_millis() as u64;
        self
    }

    /// Sets the channel where the song was requested taking a mutable reference
    pub fn channel_ref(&mut self, channel: impl Into<DiscordChannelId>) -> &mut Self {
        self.channel = Some(channel.into());

        self
    }
}
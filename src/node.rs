use lazy_static::lazy_static;
use reqwest::{Client as HttpClient, header::HeaderMap, Url, Result as ReqwestResult};
use serenity::model::id::{
    UserId as DiscordUserId,
    GuildId as DiscordGuildId
};
use crate::{builder::NodeBuilder, cluster::Cluster, error::{AndelinkError, AndelinkResult}, model::{gateway::{GatewayEvent, TrackStart, TrackFinish, WebSocketClosed, Stats}, play_parameters::PlayParameters, half_update::HalfVoiceUpdate, player::Player, track::{Track, Tracks, QueuedTrack}}, types::WebSocketConnection};
use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration
};
use tokio::sync::RwLock;
use typemap_rev::TypeMap;
use dashmap::DashMap;
use futures::StreamExt;
use tracing::{info, error, warn};
use http::Request;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use regex::Regex;
use songbird::ConnectionInfo;


lazy_static!(
    static ref URL_REGEX: Regex = Regex::new(r"https?://(?:www\.)?.+").unwrap();
);

pub struct UniversalNode {
    inner: RwLock<NodeInner>,
    http: HttpClient,
    rest_url: String
}

impl std::ops::Deref for UniversalNode {
    type Target = RwLock<NodeInner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct NodeInner {
    pub rest: String,
    pub socket: String,
    pub pass: String,
    pub shards: u64,
    pub id: DiscordUserId,
    pub socket_write: Option<WebSocketConnection>,
    pub http: HttpClient,
    pub players: HashMap<u64, Player>,
    pub stats: Option<Stats>,
    pub data: Arc<RwLock<TypeMap>>,
    pub node_id: u8,
    pub cluster: Arc<Cluster>,
    pub waiting: DashMap<DiscordGuildId, HalfVoiceUpdate>,
}

impl NodeInner {
    fn default(cluster: Arc<Cluster>, builder: NodeBuilder) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", builder.pass.clone().parse().expect("Failed parsing audio server password"));
        headers.insert("Num-Shards", builder.shards.to_string().parse().expect("Failed parsing user shards"));
        headers.insert("User-Id", builder.id.clone().unwrap().to_string().parse().expect("Failed parsing userid"));

        let http = HttpClient::builder().default_headers(headers).build().expect("Failed building http client");

        Self {
            rest: if builder.ssl { format!("https://{}:{}", builder.host, builder.port) } else { format!("http://{}:{}", builder.host, builder.port) },
            socket: if builder.ssl { format!("wss://{}:{}", builder.host, builder.port) } else { format!("ws://{}:{}", builder.host, builder.port) },
            pass: builder.pass,
            shards: builder.shards,
            id: builder.id.unwrap().into(),
            socket_write: None,
            players: Default::default(),
            http,
            stats: None,
            data: Arc::clone(&cluster.shared_data),
            cluster,
            node_id: builder.node_id.unwrap(),
            waiting: DashMap::new(),
        }
    }

    fn get_ws_request(&self) -> Request<()> {
        Request::builder()
            .uri(&self.socket)
            .header("Authorization", &self.pass)
            .header("Num-Shards", &self.shards.to_string())
            .header("User-Id", &self.id.to_string())
            .body(())
            .unwrap()
    }

    pub(crate) async fn play_next(&mut self, guild_id: u64) -> AndelinkResult<()> {
        return if let Some(player) = self.players.get_mut(&guild_id) {
            let track = player.queue[0].clone();

            player.now_playing = Some(player.queue[0].clone());

            let payload = crate::model::events::Play {
                track: track.track.track.clone(), // track
                no_replace: false,
                start_time: track.start_time,
                end_time: track.end_time,
            };

            if let Some(ref mut socket) = self.socket_write {
                crate::model::Codes::Play(payload).send(guild_id, socket).await?;
            } else {
                return Err(AndelinkError::NoWebsocket);
            }

            Ok(())
        } else {
            Err(AndelinkError::PlayerNotFound)
        }
    }

    async fn create_session(&mut self, guild_id: impl Into<DiscordGuildId>, conn_info: &ConnectionInfo) -> AndelinkResult<()> {
        let guild_id = guild_id.into();

        let socket = if let Some(s) = &mut self.socket_write {
            s
        } else {
            return Err(AndelinkError::NoWebsocket);
        };

        let token = if conn_info.token.is_empty() { return Err(AndelinkError::MissingHandlerToken); } else { conn_info.token.clone() };

        let endpoint = if conn_info.endpoint.is_empty() { return Err(AndelinkError::MissingHandlerEndpoint); } else { conn_info.endpoint.clone() };

        let session_id = if conn_info.session_id.is_empty() {return Err(AndelinkError::MissingHandlerSessionId); } else { conn_info.session_id.clone() };

        let event = crate::model::events::Event {
            token,
            endpoint,
            guild_id: guild_id.0.to_string(),
        };

        let payload = crate::model::events::VoiceUpdate {
            session_id,
            event
        };

        self.players.insert(guild_id.clone().0, Player::default());

        crate::model::Codes::VoiceUpdate(payload).send(guild_id, socket).await
    }

    async fn destroy(&mut self, guild_id: impl Into<DiscordGuildId>) -> AndelinkResult<()> {
        let guild_id = guild_id.into();

        let socket = if let Some(x) = &mut self.socket_write { x } else {
            return Err(AndelinkError::NoWebsocket);
        };

        let _ = self.players.remove(&guild_id.0);

        crate::model::Codes::Destroy.send(guild_id, socket).await?;

        Ok(())
    }

    async fn stop(&mut self, guild_id: impl Into<DiscordGuildId>) -> AndelinkResult<()> {
        let socket = if let Some(x) = &mut self.socket_write { x } else {
            return Err(AndelinkError::NoWebsocket);
        };

        crate::model::Codes::Stop.send(guild_id, socket).await?;

        Ok(())
    }

    async fn skip(&mut self, guild_id: impl Into<DiscordGuildId>) -> AndelinkResult<Option<QueuedTrack>> {
        let guild_id = guild_id.into();

        return if let Some(player) = self.players.get_mut(&guild_id.0) {
            player.now_playing = None;

            return if player.queue.len() == 0 {
                Ok(None)
            } else if player.queue.len() == 1 {
                let return_value = player.queue.remove(0);

                self.stop(guild_id).await?;

                Ok(Some(return_value))
            } else {
                let return_value = player.queue.remove(0);

                self.play_next(guild_id.0).await?;

                Ok(Some(return_value))
            }
        } else {
            Err(AndelinkError::PlayerNotFound)
        }
    }

    async fn set_pause(&mut self, guild_id: impl Into<DiscordGuildId>, pause: bool) -> AndelinkResult<()> {
        let socket = if let Some(x) = &mut self.socket_write { x } else {
            return Err(AndelinkError::NoWebsocket);
        };

        let payload = crate::model::events::Pause {
            pause,
        };

        crate::model::Codes::Pause(payload).send(guild_id, socket).await?;

        Ok(())
    }

    async fn seek(&mut self, guild_id: impl Into<DiscordGuildId>, time: Duration) -> AndelinkResult<()> {
        let socket = if let Some(x) = &mut self.socket_write { x } else {
            return Err(AndelinkError::NoWebsocket);
        };

        let payload = crate::model::events::Seek {
            position: time.as_millis() as u64,
        };

        crate::model::Codes::Seek(payload).send(guild_id, socket).await?;

        Ok(())
    }

    async fn volume(&mut self, guild_id: impl Into<DiscordGuildId>, volume: u16) -> AndelinkResult<()> {
        use std::cmp::{max, min};

        let socket = if let Some(x) = &mut self.socket_write { x } else {
            return Err(AndelinkError::NoWebsocket);
        };

        let good_volume = max(min(volume, 1000), 0);

        let payload = crate::model::events::Volume {
            volume: good_volume,
        };

        crate::model::Codes::Volume(payload).send(guild_id, socket).await?;

        Ok(())
    }

    async fn equalize_all(&mut self, guild_id: impl Into<DiscordGuildId>, bands: [f64; 15]) -> AndelinkResult<()> {
        let socket = if let Some(x) = &mut self.socket_write { x } else {
            return Err(AndelinkError::NoWebsocket);
        };

        let bands = bands.iter().enumerate().map(|(index, i)| {
            crate::model::events::Band {
                band: index as u8,
                gain: *i,
            }
        }).collect::<Vec<_>>();

        let payload = crate::model::events::Equalize {
            bands,
        };

        crate::model::Codes::Equalize(payload).send(guild_id, socket).await?;

        Ok(())
    }

    async fn equalize_band(&mut self, guild_id: impl Into<DiscordGuildId>, band: crate::model::events::Band) -> AndelinkResult<()> {
        let socket = if let Some(x) = &mut self.socket_write { x } else {
            return Err(AndelinkError::NoWebsocket);
        };

        let payload = crate::model::events::Equalize {
            bands: vec![band],
        };

        crate::model::Codes::Equalize(payload).send(guild_id, socket).await?;

        Ok(())
    }

    async fn equalize_reset(&mut self, guild_id: impl Into<DiscordGuildId>) -> AndelinkResult<()> {
        let socket = if let Some(x) = &mut self.socket_write { x } else {
            return Err(AndelinkError::NoWebsocket);
        };

        let bands = (0..=14).map(|i| {
            crate::model::events::Band {
                band: i as u8,
                gain: 0.,
            }
        }).collect::<Vec<_>>();

        let payload = crate::model::events::Equalize {
            bands,
        };

        crate::model::Codes::Equalize(payload).send(guild_id, socket).await?;

        Ok(())
    }
}

impl UniversalNode {
    pub fn new(cluster: Arc<Cluster>, builder: NodeBuilder) -> Arc<Self> {
        let inner = NodeInner::default(cluster, builder);
        let http_client = inner.http.clone();
        let rest = inner.rest.clone();

        Arc::new(Self {
            inner: RwLock::new(inner),
            http: http_client,
            rest_url: rest
        })
    }

    pub fn run(node: Arc<Self>) {
        use crate::events::{process, EventType};

        tokio::spawn(async move {
            let (node_id, cluster) = {
                let read = node.read().await;

                (read.node_id, Arc::clone(&read.cluster))
            };

            let mut actual_reconnection_attempt = 1u8;
            let max_reconnect_attempts = cluster.reconnect_attempts;

            while !(actual_reconnection_attempt > max_reconnect_attempts) {
                info!("Node id {} trying to connect to server, attempt {}", node_id, actual_reconnection_attempt);

                let url = node.read().await.get_ws_request();

                let stream = tokio_tungstenite::connect_async(url).await;

                if let Err(_) = stream {
                    actual_reconnection_attempt += 1;

                    warn!("Node id {} failed to reconnect to server (attempt {}/{}), waiting 5s before reconnecting", node_id, actual_reconnection_attempt - 1, max_reconnect_attempts);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                } else {
                    let (write, mut read) = stream.unwrap().0.split();

                    info!("Node id {} connected successfully to server", node_id);

                    actual_reconnection_attempt = 1;

                    Self::add_to_cluster(&cluster, node_id, Arc::clone(&node));

                    {
                        let mut node_write = node.write().await;

                        node_write.socket_write = Some(write);
                    }

                    while let Some(Ok(msg)) = read.next().await {
                        match msg {
                            TungsteniteMessage::Text(t) => {
                                if let Ok(payload) = serde_json::from_str::<GatewayEvent>(&t) {
                                    match payload.op.as_str() {
                                        "stats" => {
                                            if let Ok(stats) = serde_json::from_str::<Stats>(&t) {

                                                // Set last stats
                                                node.write().await.stats = Some(stats.clone());
                                                
                                                // Dispatch the event
                                                process(Arc::clone(&node), Arc::clone(&cluster.event_handler), EventType::Stats(stats));
                                            }
                                        },
                                        "playerUpdate" => {
                                            if let Ok(player_update) = serde_json::from_str::<crate::model::gateway::PlayerUpdate>(&t) {
                                                {
                                                    let mut node_write = node.write().await;

                                                    if let Some(player) = node_write.players.get_mut(&player_update.guild_id) {
                                                        if let Some(current_track) = player.now_playing.as_mut() {
                                                            if let Some(mut info) = current_track.track.info.as_mut() {

                                                                // Set the new position provided by lavalink/andesite server
                                                                info.position = player_update.state.position;
                                                            }
                                                        }
                                                    }
                                                }

                                                // Dispatch the event
                                                process(Arc::clone(&node), Arc::clone(&cluster.event_handler), EventType::PlayerUpdate(player_update));
                                            }
                                        },
                                        "event" => {
                                            match payload.event_type.unwrap().as_str() {
                                                "TrackStartEvent" => {
                                                    if let Ok(track_start) = serde_json::from_str::<TrackStart>(&t) {
                                                        
                                                        // Dispatch the event
                                                        process(Arc::clone(&node), Arc::clone(&cluster.event_handler), EventType::TrackStart(track_start));
                                                    }
                                                },
                                                "TrackEndEvent" => {
                                                    if let Ok(track_end) = serde_json::from_str::<TrackFinish>(&t) {

                                                        if track_end.reason == "FINISHED" {
                                                            let mut should_play_next = false;

                                                            let mut node_write = node.write().await;

                                                            if let Some(player) = node_write.players.get_mut(&track_end.guild_id) {

                                                                // Remove track from queue
                                                                player.queue.remove(0);

                                                                // Set now playing
                                                                player.now_playing = None;

                                                                // Check if we should play next track
                                                                if player.queue.len() >= 1 { should_play_next = true; }
                                                            }

                                                            if should_play_next {
                                                                // Play next track
                                                                if let Err(why) = node_write.play_next(track_end.guild_id).await {
                                                                    error!("Error playing on guild id: {}, error: {}", track_end.guild_id, why.to_string())
                                                                }
                                                            }
                                                        }

                                                        // Dispatch the event
                                                        process(Arc::clone(&node), Arc::clone(&cluster.event_handler), EventType::TrackFinish(track_end));
                                                    }
                                                },
                                                "WebSocketClosedEvent" => {
                                                    if let Ok(socket_closed) = serde_json::from_str::<WebSocketClosed>(&t) {
                                                        
                                                        // Distpatch the event
                                                        process(Arc::clone(&node), Arc::clone(&cluster.event_handler), EventType::WebSocketClosed(socket_closed));
                                                    }
                                                },
                                                _ => (),
                                            }
                                        },
                                        _ => ()
                                    }
                                }
                            },
                            TungsteniteMessage::Close(_) => break,
                            _ => ()
                        }
                    }

                    // Temporarily delete the node from cluster so we won't try to play anithing on it until reconnect
                    if cluster.nodes.contains_key(&node_id) {
                        info!("Temporarily removing node id {} from cluster due to disconnection", node_id);

                        Self::remove_from_cluster(&cluster, node_id);
                    }
                }
            }

            // If node reaches max attempts, exit the task and remove it from cluster
            info!("Node id {} reached max connection attempts, removing from cluster and disconnecting", node_id);
            Self::remove_from_cluster(&cluster, node_id);
        });
    }

    fn remove_from_cluster(cluster: &Arc<Cluster>, id: u8) {
        cluster.nodes.remove(&id);

        info!("Node id {} removed from cluster successfully", id);
    }

    fn add_to_cluster(cluster: &Arc<Cluster>, id: u8, node: Arc<Self>) {
        cluster.nodes.insert(id, node);

        info!("Node id {} added to cluster successfully", id);
    }

    pub async fn get_tracks<Q: ToString>(&self, query: Q) -> ReqwestResult<Tracks> {

        let url = Url::parse_with_params(&format!("{}/loadtracks", self.rest_url), &[("identifier", &query.to_string())]).expect("Error while formatting query into an url");

        let response = self.http.get(url)
            .send()
            .await?
            .json::<Tracks>()
            .await?;

        Ok(response)
    }

    pub async fn auto_search<Q: ToString>(&self, query: Q) -> ReqwestResult<Tracks> {
        if URL_REGEX.is_match(&query.to_string()) {
            self.get_tracks(query).await
        } else {
            self.get_tracks(format!("ytsearch:{}", query.to_string())).await
        }
    }

    /// Method to create a session and be able to connect the server to discord
    pub async fn create_session(&self, guild_id: impl Into<DiscordGuildId>, conn_info: &ConnectionInfo) -> AndelinkResult<()> {
        let mut node_write = self.inner.write().await;

        node_write.create_session(guild_id, conn_info).await
    }

    /// Constructor for playing a track.
    pub fn play(&self, guild_id: impl Into<DiscordGuildId>, track: Track) -> PlayParameters {
        let mut p = PlayParameters::default(&self);
        p.track = track;
        p.guild_id = guild_id.into().0;
        p
    }

    /// Constructor shortcut to add entire playlists to queue, **map** will be called for every track converted to play parameters
    pub fn play_playlist<'a, F>(&'a self, guild: impl Into<DiscordGuildId>, tracks: Vec<Track>, map: F) -> Vec<PlayParameters<'a>>
    where
        F: for<'b> Fn(&'b mut PlayParameters<'a>) -> &'b mut PlayParameters<'a>
    {
        let guild = guild.into();
        let mut playlist = Vec::new();

        for track in tracks {
            let mut p = self.play(guild, track);

            map(&mut p);

            playlist.push(p);
        }

        playlist
    }

    /// Destroys the current player.
    /// When this is run, `create_session()` needs to be run again.
    pub async fn destroy(&self, guild_id: impl Into<DiscordGuildId>) -> AndelinkResult<()> {
        let mut node_write = self.inner.write().await;

        node_write.destroy(guild_id).await
    }

     /// Stops the current player.
     pub async fn stop(&self, guild_id: impl Into<DiscordGuildId>) -> AndelinkResult<()> {
         let mut node_write = self.inner.write().await;

         node_write.stop(guild_id).await
     }

      /// Skips the current playing track to the next item on the queue.
    ///
    /// If nothing is in the queue, player will automatically be stopped
    pub async fn skip(&self, guild_id: impl Into<DiscordGuildId>) -> AndelinkResult<Option<QueuedTrack>> {
        let mut node_write = self.inner.write().await;

        node_write.skip(guild_id).await
    }

    /// Sets the pause status.
    pub async fn set_pause(&self, guild_id: impl Into<DiscordGuildId>, pause: bool) -> AndelinkResult<()> {
        let mut node_write = self.inner.write().await;

        node_write.set_pause(guild_id, pause).await
    }

    /// Sets pause status to `True`
    pub async fn pause(&self, guild_id: impl Into<DiscordGuildId>) -> AndelinkResult<()> {
        self.set_pause(guild_id, true).await
    }

    /// Sets pause status to `False`
    pub async fn resume(&self, guild_id: impl Into<DiscordGuildId>) -> AndelinkResult<()> {
        self.set_pause(guild_id, false).await
    }

    /// Jumps to a specific time in the currently playing track.
    pub async fn seek(&self, guild_id: impl Into<DiscordGuildId>, time: Duration) -> AndelinkResult<()> {
        let mut node_write = self.inner.write().await;

        node_write.seek(guild_id, time).await
    }

    /// Sets the volume of the player.
    pub async fn volume(&self, guild_id: impl Into<DiscordGuildId>, volume: u16) -> AndelinkResult<()> {
        let mut node_write = self.inner.write().await;

        node_write.volume(guild_id, volume).await
    }

    /// Sets all equalizer levels.
    ///
    /// There are 15 bands (0-14) that can be changed.
    /// The floating point value is the multiplier for the given band. The default value is 0.
    /// Valid values range from -0.25 to 1.0, where -0.25 means the given band is completely muted, and 0.25 means it is doubled.
    /// Modifying the gain could also change the volume of the output.
    pub async fn equalize_all(&self, guild_id: impl Into<DiscordGuildId>, bands: [f64; 15]) -> AndelinkResult<()> {
        let mut node_write = self.inner.write().await;

        node_write.equalize_all(guild_id, bands).await
    }

    /// Equalizes a specific band.
    pub async fn equalize_band(&self, guild_id: impl Into<DiscordGuildId>, band: crate::model::events::Band) -> AndelinkResult<()> {
        let mut node_write = self.inner.write().await;

        node_write.equalize_band(guild_id, band).await
    }

    /// Resets all equalizer levels.
    pub async fn equalize_reset(&self, guild_id: impl Into<DiscordGuildId>) -> AndelinkResult<()> {
        let mut node_write = self.inner.write().await;

        node_write.equalize_reset(guild_id).await
    }

}
use serenity::{
    model::channel::Message,
    model::id::{GuildId, UserId, ChannelId},
    client::Context,
    cache::Cache
};
use songbird::error::JoinResult;

pub async fn is_on_same_vc(cache: impl AsRef<Cache>, guild: impl Into<GuildId>, user: impl Into<UserId>) -> VoiceLocationState {
    let cache = cache.as_ref();
    let user_id = user.into();
    let guild_id = guild.into();
    let bot_id = cache.current_user_id().await;

    let bot_channel = match cache.guild(&guild_id).await {
        None => None,
        Some(guild) => {
            guild.voice_states.get(&bot_id)
                .and_then(|vs| vs.channel_id)
        }
    };

    let user_channel = match cache.guild(&guild_id).await {
        None => None,
        Some(guild) => {
            guild.voice_states.get(&user_id)
                .and_then(|vs| vs.channel_id)
        }
    };

    if bot_channel.is_none() { return VoiceLocationState::ClientDisconnected }
    if user_channel.is_none() { return VoiceLocationState::UserDisconnected }

    return if bot_channel.unwrap() == user_channel.unwrap() {
        VoiceLocationState::OnSameChannel
    } else {
        VoiceLocationState::OnDifferentChannel
    }
}

pub enum VoiceLocationState {
    /// The bot is not connected to any VC
    ClientDisconnected,
    /// The message author is not connected to any VC
    UserDisconnected,
    /// Bot and message author are on the same VC
    OnSameChannel,
    /// Bot and message author are on different VC's
    OnDifferentChannel
}

pub async fn connect_to(ctx: &Context, guild: impl Into<GuildId>, channel: impl Into<ChannelId>) -> UtilResult<()> {
    let guild_id = guild.into();

    let channel_id = channel.into();

    let manager = match songbird::get(&ctx).await {
        Some(m) => m.clone(),
        None => return Err(UtilError::MissingSongbird),
    };

    let(_, handler) = manager.join_gateway(guild_id.clone(), channel_id).await;

    match handler {
        Ok(conn_info) => {
            let lavalink_cluster = {
                let data = ctx.data.read().await;
                data.get::<crate::cluster::Cluster>().expect("Unable to find andelink cluster").clone()
            };
            let lavalink_node = lavalink_cluster.get_best().await?;

            lavalink_node.create_session(&guild_id, &conn_info).await?;
        },
        Err(why) => return Err(UtilError::Songbird(why))
    }

    Ok(())

}

pub async fn disconnect_from(ctx: &Context, guild_id: impl Into<GuildId>) -> UtilResult<()> {
    let guild_id = guild_id.into();

    let manager = match songbird::get(&ctx).await {
        Some(m) => m.clone(),
        None => return Err(()),
    };

    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        manager.remove(guild_id).await?;

        {
            let data = ctx.data.read().await;
            let lavalink_cluster = data.get::<crate::cluster::Cluster>().expect("Unable to find andelink cluster").clone();

            let lavalink_node = lavalink_cluster.get_player_node(guild_id.clone().0).await?;

            lavalink_node.destroy(guild_id).await?;

            Ok(())
        }
    }

    Ok(())
}

type UtilResult<T> = Result<T, UtilError>;

#[derive(Debug)]
pub enum UtilError {
    Songbird(songbird::error::JoinError),
    Cluster(crate::error::ClusterError),
    AndeLink(crate::error::AndelinkError),
    MissingSongbird,
}

impl std::fmt::Display for UtilError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Songbird(why) => why.fmt(f),
            Self::Cluster(why) => why.fmt(f),
            Self::AndeLink(why) => why.fmt(f),
            Self::MissingSongbird => write!("Missing songbird instance, be sure to install it before"),
        }
    }
}

impl std::error::Error for UtilError {}
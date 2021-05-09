use std::{
    error::Error,
    fmt::{
        Display,
        Formatter,
        Result,
    },
};
use tokio_tungstenite::tungstenite::error::Error as TungsteniteError;

pub type AndelinkResult<T> = ::std::result::Result<T, AndelinkError>;

#[derive(Debug)]
pub enum AndelinkError {
    NoWebsocket,
    PlayerNotFound,
    MissingHandlerToken,
    MissingHandlerEndpoint,
    MissingHandlerSessionId,
    InvalidDataToVoiceUpdate,
    InvalidDataToPlay,
    InvalidDataToStop,
    InvalidDataToDestroy,
    InvalidDataToPause,
    InvalidDataToVolume,
    InvalidDataToSeek,
    ErrorSendingPayload(TungsteniteError),
}

impl Error for AndelinkError {}

impl Display for AndelinkError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            AndelinkError::NoWebsocket => write!(f, "There is no initialized websocket."),
            AndelinkError::MissingHandlerToken => write!(f, "No `token` was found on the handler."),
            AndelinkError::MissingHandlerEndpoint => write!(f, "No `endpoint` was found on the handler."),
            AndelinkError::MissingHandlerSessionId => write!(f, "No `session_id` was found on the hander"),
            AndelinkError::InvalidDataToPlay => write!(f, "Invalid data was provided to the `play` json."),
            AndelinkError::InvalidDataToStop => write!(f, "Invalid data was provided to the `stop` json."),
            AndelinkError::InvalidDataToDestroy => write!(f, "Invalid data was provided to the `destroy` json."),
            AndelinkError::InvalidDataToPause => write!(f, "Invalid data was provided to the `pause` json."),
            AndelinkError::InvalidDataToVolume => write!(f, "Invalid data was provided to the `volume` json."),
            AndelinkError::InvalidDataToSeek => write!(f, "Invalid data was provided to the `seek` json."),
            AndelinkError::InvalidDataToVoiceUpdate => write!(f, "Invalid data was provided to the `voiceUpdate` json."),
            AndelinkError::ErrorSendingPayload(why) => write!(f, "Error while sending payload, json => {:?}", why),
            AndelinkError::PlayerNotFound => write!(f, "Player not found"),
            //_ => write!(f, "Unhandled error occurred."),
        }
    }
}

impl From<TungsteniteError> for AndelinkError {
    fn from(e: TungsteniteError) -> AndelinkError {
        Self::ErrorSendingPayload(e)
    }
}

pub type ClusterResult<T> = ::std::result::Result<T, ClusterError>;

#[derive(Debug)]
pub enum ClusterError {
    MissingSelfRef,
    CannotFindNode,
    CannotFindBestNode,
    CannotAddNode,
    CannotUpdateVoiceState,
    Tungstenite(TungsteniteError),
    Andelink(AndelinkError)
}

impl Error for ClusterError {}

impl Display for ClusterError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            ClusterError::MissingSelfRef => write!(f, "Missing self arc reference to initialize the node"),
            ClusterError::CannotFindNode => write!(f, "Cannot find node, this can mean the node you're trying to search is not available or not exists"),
            ClusterError::CannotFindBestNode => write!(f, "Cannot find the best node, this is caused because there are no nodes added"),
            ClusterError::CannotAddNode => write!(f, "Cannot add node to cluster"),
            ClusterError::Tungstenite(e) => write!(f, "{:#?}", e),
            ClusterError::CannotUpdateVoiceState => write!(f, "Failed to update node voice state"),
            ClusterError::Andelink(e) => e.fmt(f)
        }
    }
}

impl From<AndelinkError> for ClusterError {
    fn from(e: AndelinkError) -> ClusterError {
        ClusterError::Andelink(e)
    }
}

impl From<TungsteniteError> for ClusterError {
    fn from(e: TungsteniteError) -> ClusterError {
        Self::Tungstenite(e)
    }
}
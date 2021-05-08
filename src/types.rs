use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::Message
};
use tokio::net::TcpStream;
use futures::stream::SplitSink;

pub type WebSocketConnection = SplitSink<WebSocketStream<TcpStream>, Message>;
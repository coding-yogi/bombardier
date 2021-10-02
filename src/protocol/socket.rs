
use futures::{SinkExt, StreamExt, stream::{SplitSink, SplitStream}};
use log::*;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::{
    connect_async,
    MaybeTlsStream,
    WebSocketStream as TTWebSocketStream
};
use tungstenite::Message;

use crate::report::stats;

pub struct WebSocketSink<T> where T: AsyncRead + AsyncWrite + Unpin {
    sink: SplitSink<TTWebSocketStream<T>, Message>
}

impl <T> WebSocketSink<T> where T: AsyncRead + AsyncWrite + Unpin {

    pub fn new(sink: SplitSink<TTWebSocketStream<T>, Message>) -> Self {
        WebSocketSink { sink }
    }

    pub async fn write(&mut self, message: String) {
        match self.sink.send(Message::Text(message)).await {
            Ok(_) => (),
            Err(err) => error!("Error occured while writing to socket: {}", err)
        }
    }

    pub async fn write_stats(&mut self, stats: &[stats::Stats]) {
        self.write(serde_json::to_string(&stats).unwrap()).await //check why json and not comma separated
    }
}

pub struct WebSocketStream<T> where T: AsyncRead + AsyncWrite +  {
    stream: SplitStream<TTWebSocketStream<T>>
}

impl <T> WebSocketStream<T> where T: AsyncRead + AsyncWrite + Unpin {
    pub fn new(stream: SplitStream<TTWebSocketStream<T>>) -> Self {
        WebSocketStream { stream }
    }

    pub async fn read(&mut self) -> Result<tungstenite::protocol::Message, tungstenite::error::Error> {
        match self.stream.next().await {
            Some(some) => some,
            None => Err(tungstenite::error::Error::AlreadyClosed)
        }
    }
}

pub async fn connect(url: String) -> Result<TTWebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>, Box<dyn std::error::Error>> {
     match connect_async(url::Url::parse(&url).unwrap()).await {
         Ok((ws, _)) => Ok(ws),
         Err(err) => Err(format!("Connection failed: {}", err).into())
     }
}
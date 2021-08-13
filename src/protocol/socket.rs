
use futures::{SinkExt, StreamExt, stream::{SplitSink, SplitStream}};
use log::*;
use tungstenite::{
    Message,
};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::{
    connect_async,
    MaybeTlsStream,
    WebSocketStream as TTWebSocketStream
};

use crate::report::stats;

pub struct WebSocketSink<T> where T: AsyncRead + AsyncWrite + Unpin {
    sink: SplitSink<TTWebSocketStream<T>, Message>
}

impl <T> WebSocketSink<T> where T: AsyncRead + AsyncWrite + Unpin {

    pub fn new(sink: SplitSink<TTWebSocketStream<T>, Message>) -> Self {
        WebSocketSink { sink }
    }

    pub async fn write(&mut self, message: String) {
        match self.sink.send(Message::Text(message).into()).await {
            Ok(_) => (),
            Err(err) => error!("Error occured while writing to socket: {}", err)
        }
    }

    pub async fn close(&mut self) {
        match self.sink.send(Message::Close(None)).await {
            Ok(_) => (),
            Err(err) => error!("Error occured while sending close message to socket: {}", err)
        }
    }
}

impl <T> stats::StatsWriter for WebSocketSink<T> where T: AsyncRead + AsyncWrite + Unpin {
    fn write_stats(&mut self, stats: &Vec<stats::Stats>) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(
            async {
                self.write(serde_json::to_string(&stats).unwrap()).await //check why json and not comma separated
            }
        )
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
        self.stream.next().await.unwrap()
    }
}

pub async fn connect(url: String) -> Result<TTWebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>, Box<dyn std::error::Error>> {
     match connect_async(url::Url::parse(&url).unwrap()).await {
         Ok((ws, _)) => Ok(ws),
         Err(err) => Err(format!("Connection failed: {}", err).into())
     }
}
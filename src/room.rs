use std::net::SocketAddr;
use tokio::sync::broadcast;
use std::error::Error;

use crate::client::Client;

pub struct Room {
    pub tx: broadcast::Sender<(String, SocketAddr)>,
}

impl Room {
    pub fn new() -> Self {
        Room {
            tx: broadcast::channel(8).0
        }
    }

    pub async fn join_broadcast(&self, client: &mut Client) -> Result<(), Box<dyn Error>> {
        client.join_broadcast(self.tx.clone()).await?;
        return Ok(());
    }
}

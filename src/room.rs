use std::net::SocketAddr;
use tokio::sync::broadcast;

pub struct Room {
    pub tx: broadcast::Sender<(String, SocketAddr)>,
    pub clients: Vec<SocketAddr>,
}

impl Room {
    pub fn new() -> Self {
        Room {
            tx: broadcast::channel(8).0,
            clients: Vec::new(),
        }
    }

    pub fn get_broadcast(&self) -> broadcast::Sender<(String, SocketAddr)> {
        return self.tx.clone();
    }

    pub fn add_client(&mut self, addr: SocketAddr) {
        self.clients.push(addr);
    }

    pub fn remove_client(&mut self, addr: SocketAddr) {
        self.clients.retain(|&x| x != addr);
    }
}

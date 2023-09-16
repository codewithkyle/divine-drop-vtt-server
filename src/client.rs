use tokio::{net::TcpStream, io::{BufReader, AsyncBufReadExt, AsyncWriteExt}, sync::Mutex};
use std::{net::SocketAddr, collections::HashMap, sync::Arc};
use tokio::sync::broadcast;
use std::error::Error;

use crate::room::Room;

pub struct Client {
    socket: TcpStream,
    pub addr: SocketAddr,
    pub room_code: String,
    rooms: Arc<Mutex<HashMap<String, Room>>>,
    tx: broadcast::Sender<(String, SocketAddr)>,
    rx: broadcast::Receiver<(String, SocketAddr)>,
}

impl Client {
    pub fn new(socket: TcpStream, addr: SocketAddr) -> Self {
        Client {
            socket,
            addr,
            room_code: "".to_string(),
            rooms: Arc::new(Mutex::new(HashMap::new())),
            tx: broadcast::channel(8).0,
            rx: broadcast::channel(8).1,
        }
    } 

    pub async fn wait_for_room(&mut self) -> Result<String, Box<dyn Error>> {
        let (reader, _) = self.socket.split();
        let mut reader = BufReader::new(reader);

        let mut line = String::new();
        loop {
            println!("{} waiting for room type", self.addr);
            reader.read_line(&mut line).await?;
            if !line.trim().is_empty() {
                println!("{} sent {}", self.addr, line);
            }
            if line.trim().starts_with("dd::core::") {
                break;
            }
            line.clear();
        }

        return Ok(line.trim().to_string());
    }

    pub fn join_room(&mut self, rooms: Arc<Mutex<HashMap<String, Room>>>) {
        self.rooms = rooms;
    }

    pub fn set_room_code(&mut self, room_code: String) {
        self.room_code = room_code;
    }

    pub fn join_broadcast(&mut self, tx: broadcast::Sender<(String, SocketAddr)>) {
        self.rx = tx.subscribe();
        self.tx = tx;
    }

    pub async fn listen(&mut self) -> Result<(), Box<dyn Error>> {
        let (reader, mut writer) = self.socket.split();

        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            tokio::select! {
                result = reader.read_line(&mut line) => {
                    if result? == 0 {
                        break;
                    }
                    line = line.trim().to_string();
                    self.tx.send((line.clone(), self.addr))?;
                    println!("{} sent {}", self.addr, line);
                    line.clear();
                },
                result = self.rx.recv() => {
                    let (msg, other_addr) = result?;
                    if self.addr != other_addr {
                        writer.write_all(msg.as_bytes()).await?;
                    }
                }
            }
        }
        return Ok(());
    }
}

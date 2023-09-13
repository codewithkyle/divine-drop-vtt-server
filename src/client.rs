use tokio::{net::TcpStream, io::{BufReader, AsyncBufReadExt, AsyncWriteExt}, sync::Mutex};
use std::{net::SocketAddr, collections::HashMap, sync::Arc};
use tokio::sync::broadcast;
use std::error::Error;

use crate::room::Room;

pub struct Client {
    socket: TcpStream,
    addr: SocketAddr,
    room_code: String,
    rooms: Arc<Mutex<HashMap<String, Room>>>,
}

impl Client {
    pub fn new(socket: TcpStream, addr: SocketAddr) -> Self {
        Client {
            socket,
            addr,
            room_code: "".to_string(),
            rooms: Arc::new(Mutex::new(HashMap::new())),
        }
    } 

    pub async fn wait_for_room(&mut self) -> Result<String, Box<dyn Error>> {
        let (reader, _) = self.socket.split();
        let mut reader = BufReader::new(reader);

        let mut line = String::new();
        reader.read_line(&mut line).await?;
        return Ok(line.trim().to_string());
    }

    pub fn join_room(&mut self, room_code: String, rooms: Arc<Mutex<HashMap<String, Room>>>) {
        self.room_code = room_code;
        self.rooms = rooms;
    }

    pub async fn join_broadcast(&mut self, tx: broadcast::Sender<(String, SocketAddr)>) -> Result<(), Box<dyn Error>> {
        let mut rx = tx.subscribe();

        let (reader, mut writer) = self.socket.split();

        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            tokio::select! {
                result = reader.read_line(&mut line) => {
                    if result? == 0 {
                        break;
                    }
                    tx.send((line.clone(), self.addr))?;
                    println!("{} sent {}", self.addr, line);
                    line.clear();
                },
                result = rx.recv() => {
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

use std::collections::HashMap;
use std::{error::Error, sync::Arc};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use rand::Rng;

use crate::room::Room;
use crate::client::Client;

mod room;
mod client;

type ROOMS = Arc<Mutex<HashMap<String, Room>>>;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let rooms: ROOMS = Arc::new(Mutex::new(HashMap::new()));

    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    println!("WebSocket server is running on ws://127.0.0.1:8080");

    while let Ok((socket, addr)) = listener.accept().await {
        let rooms = rooms.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, addr, rooms).await {
                println!("an error occurred; error = {}", e);
            }
        });
    }

    return Ok(());
}

async fn handle_connection(
    socket: TcpStream, 
    addr: SocketAddr, 
    rooms: Arc<Mutex<HashMap<String, Room>>>
) -> Result<(), Box<dyn Error>> {
    println!("{} connected", addr);

    let mut client = Client::new(socket, addr);
    let msg = client.wait_for_room().await?;
    println!("{} sent {}", addr, msg);

    let op: String = msg.split("::").skip(2).collect();

    let room_code: String;
    match op.to_uppercase().as_str() {
        "NEW_ROOM" => {
            room_code = generate_room_code();
            println!("{} created room {}", addr, room_code);
        },
        "JOIN_ROOM" => {
            room_code = msg.split("::").skip(3).collect();
            println!("{} joined room {}", addr, room_code);
        }
        _ => {
            println!("{} sent an invalid message", addr);
            return Ok(());
        }
    }
  
    let mut room_map = rooms.lock().await;
    let room = room_map.entry(room_code.to_lowercase().clone()).or_insert_with(|| Room::new());
    client.join_room(room_code, rooms.clone());
    room.join_broadcast(&mut client).await?;

    return Ok(());
}

fn generate_room_code() -> String {
    let random_string: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();

    return random_string;
}

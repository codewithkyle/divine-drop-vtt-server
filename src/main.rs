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
        let rooms_a = rooms.clone();
        let client = tokio::spawn(async move {
            let mut client = handle_connection(socket, addr).await.unwrap();
            let mut room_map = rooms_a.lock().await;
            let room = room_map.entry(client.room_code.to_lowercase().clone()).or_insert_with(|| Room::new());
            room.add_client(client.addr);
            client.join_room(rooms_a.clone());
            client.join_broadcast(room.get_broadcast());
            return client;
        });
        let rooms_b = rooms.clone();
        tokio::spawn(async move {
            let mut client:Client = client.await.unwrap();
            client.listen().await.unwrap();
            println!("{} disconnected", client.addr);
            let mut room_map = rooms_b.lock().await;
            match room_map.get_mut(&client.room_code) {
                Some(room) => {
                    println!("{} left room {}", client.addr, client.room_code);
                    if room.clients.len() == 1 {
                        room_map.remove(&client.room_code);
                        println!("closed room {}", client.room_code);
                    } else {
                        room.remove_client(client.addr);
                    }
                },
                None => {}
            }
        });
    }
    return Ok(());
}

async fn handle_connection(
    socket: TcpStream, 
    addr: SocketAddr, 
) -> Result<Client, Box<dyn Error>> {
    println!("{} connected", addr);

    let mut client = Client::new(socket, addr);
    let msg = client.wait_for_room().await?;
    println!("{} sent {}", addr, msg);

    let room_code: String;
    match msg.split("::").nth(2).unwrap() {
        "NEW_ROOM" => {
            room_code = generate_room_code().to_lowercase();
            println!("{} created room {}", addr, room_code);
        },
        "JOIN_ROOM" => {
            room_code = msg.split("::").nth(3).unwrap().to_lowercase();
            println!("{} joined room {}", addr, room_code);
        }
        _ => {
            println!("{} sent an invalid message", addr);
            return Err("Invalid message".into());
        }
    }
    client.set_room_code(room_code.clone());

    return Ok(client);
}

fn generate_room_code() -> String {
    let random_string: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();

    return random_string;
}

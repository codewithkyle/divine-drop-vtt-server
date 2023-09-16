use std::collections::HashMap;
use std::{error::Error, sync::Arc};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter, AsyncBufReadExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use rand::Rng;
use crypto::digest::Digest;

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
        if let Ok(client) = client.await {
            let rooms_b = rooms.clone();
            tokio::spawn(async move {
                let mut client:Client = client;
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
    }
    return Ok(());
}

async fn handle_connection(
    mut socket: TcpStream, 
    addr: SocketAddr, 
) -> Result<Client, Box<dyn Error>> {
    println!("{} connected", addr);

    handle_handshake(&mut socket).await?;

    let mut client = Client::new(socket, addr);
    let msg = client.wait_for_room().await?;
    println!("{} sent {}", addr, msg);

    let room_code: String;
    match msg.split("::").nth(2).unwrap_or("") {
        "NEW_ROOM" => {
            room_code = generate_room_code().to_lowercase();
            println!("{} created room {}", addr, room_code);
        },
        "JOIN_ROOM" => {
            room_code = msg.split("::").nth(3).unwrap_or("").to_lowercase();
            println!("{} joined room {}", addr, room_code);
        }
        _ => {
            println!("{} sent an invalid message", addr);
            return Err("Invalid message".into());
        }
    }
    if room_code.len() != 8 {
        println!("{} sent an invalid room code", addr);
        return Err("Invalid room code".into());
    }
    client.set_room_code(room_code.clone());

    return Ok(client);
}

async fn handle_handshake(socket: &mut TcpStream) -> Result<(), Box<dyn Error>> {
    println!("Handling handshake"); 
    let mut request = String::new();
    let (reader, writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    // Read the request
    let mut line = String::new();
    loop {
        line.clear();
        reader.read_line(&mut line).await?;
        if line == "\r\n" {
            break;
        }
        request.push_str(&line);
    }

    // Parse the request to get the value of Sec-WebSocket-Key
    println!("{}", request);
    let sec_websocket_key = request
        .lines()
        .find(|line| line.starts_with("Sec-WebSocket-Key:"))
        .map(|line| line.trim_start_matches("Sec-WebSocket-Key:").trim())
        .expect("WebSocket handshake request missing Sec-WebSocket-Key");

    println!("Sec-WebSocket-Key: {}", sec_websocket_key);
    let sec_websocket_accept = calculate_accept_key(sec_websocket_key);

    // Create the WebSocket handshake response
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {}\r\n\r\n",
        sec_websocket_accept
    );

    println!("{}", response);
    writer.write_all_buf(&mut response.as_bytes()).await?;
    writer.flush().await?;
    
    Ok(())
}

fn generate_room_code() -> String {
    let random_string: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();

    return random_string;
}

fn calculate_accept_key(key: &str) -> String {
    const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let combined_key = format!("{}{}", key, GUID);
    let mut hasher1 = crypto::sha1::Sha1::new();
    hasher1.input_str(&combined_key);
    let mut result = [0u8; 20];
    hasher1.result(&mut result);
    base64::encode(&result)
}

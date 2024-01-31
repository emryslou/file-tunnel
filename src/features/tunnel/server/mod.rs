use async_std::{io::ReadExt as _, stream::StreamExt as _};
use tide_websockets::{Message, WebSocket, WebSocketConnection};
use tide::Request;

use super::websocket_channel;

pub fn binding(app: &mut tide::Server<()>) {
    app.at("/server").nest({
        let mut srv = tide::new();
        srv.at("/registe").post(registe_share_key);
        srv.at("/ws").with(WebSocket::new(srv_ws_handler))
                    .get(|_| async move { Ok(format!("request as websocket")) });
        srv
    });
}

async fn registe_share_key(mut req: Request<()>) -> tide::Result {
    let _share_key = req.header("X-Share-Key").unwrap();
    let share_key = _share_key.get(0).unwrap().to_string();
    match websocket_channel::get(&share_key).await {
        Some(conn) => {
            conn.send(Message::from("hello registe")).await.unwrap();
        },
        None => {},
    };
    let mut buf: [u8; 12] = [0; 12];
    req.read(&mut buf).await.unwrap();
    Ok(format!("demo {share_key}").into())
}

async fn srv_ws_handler(req: Request<()>, mut stream: WebSocketConnection) -> tide::Result<()> {
    let mut share_key = "";
    match req.header("X-Share-Key") {
        Some(_share_key) => {
            share_key = _share_key.get(0).unwrap().as_str();
            websocket_channel::add(share_key, stream.clone()).await;
        },
        None => {}
    };
    if share_key != "" {
        println!("online: {}", share_key);
        stream.send_string(format!("hi {}", share_key).into()).await.unwrap();
        loop {
            match stream.next().await {
                Some(result) => {
                    match result {
                        Ok(message) => {
                            match message {
                                Message::Text(input) => {
                                    let (client_key_size_str, next_data) = input.split_once(":").unwrap();
                                    let client_key_size = client_key_size_str.to_string().parse::<usize>().unwrap();
                                    let (client_key, next_data) = next_data.split_at(client_key_size);
                                    websocket_channel::proxy_send(share_key, client_key, next_data.as_bytes().to_vec()).await.unwrap();
                                },
                                Message::Binary(input) => {
                                    let client_key_size = input[0] as usize;
                                    let client_key_chars: Vec<char> = input[1..(client_key_size + 1)].iter().map(|c| *c as char).collect();
                                    let client_key = String::from_iter(client_key_chars);
                                    let msg = input[(client_key_size + 1)..].to_vec();
                                    websocket_channel::proxy_send(&share_key, &client_key, msg).await.unwrap();
                                }
                                Message::Close(_static) => {
                                    println!("exit {}", &share_key);
                                    break ;
                                }
                                _ => {}
                            }
                        },
                        Err(e) => {
                            eprintln!("message handler error, {}", e);
                            break ;
                        }
                    }
                },
                None => break
            }
        }
        websocket_channel::del(&share_key).await;
    }
    else {
        stream.send_string("share key has be required".into()).await.unwrap();
    }
    Ok(())
}

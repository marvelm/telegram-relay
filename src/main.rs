extern crate rustc_serialize;
extern crate docopt;
extern crate rand;
extern crate hyper;

use std::net::{TcpListener, TcpStream, Shutdown};
use std::thread;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::io::{Read, Write, BufReader, BufRead};
use std::sync::{Arc, Mutex};

use hyper::client::Client;

use rand::random;

use rustc_serialize::json::Json;
use rustc_serialize::json;

use docopt::Docopt;

const USAGE: &'static str = "
Telegram Relay

Usage:
  telegram-relay start <token>
";

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_token: String
}

enum RelayMessage {
    Stop,
    Message(Json)
}

fn listen(mut stream: TcpStream, rx: Receiver<RelayMessage>) {
    'listening: loop {
        let relay_message = rx.recv().unwrap();
        match relay_message {
            RelayMessage::Message(json_message) => {
                let as_raw_json = json::encode(&json_message)
                    .expect("Encoding message");
                stream.write_all(&as_raw_json.into_bytes()[..])
                    .expect("Writing encoded message to socket");
                stream.write_all(b"\n").expect("Writing line");
            },
            RelayMessage::Stop => {
                stream.shutdown(Shutdown::Both);
                break 'listening;
            },
        };
    }
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    let tcp_listener = TcpListener::bind("127.0.0.1:9001").unwrap();

    let listeners = Arc::new(Mutex::new(HashMap::<i64, Sender<RelayMessage>>::new()));
    let user_to_stream = Arc::new(Mutex::new(HashMap::<i64, i64>::new()));

    let listeners_mutex = listeners.clone();
    thread::spawn(move || {
        let user_to_stream = user_to_stream.clone();

        // Load-balances and determines which Stream should handle a new user.
        // It's incremented when a message from a new user is received
        // and reset to 0 when every Stream has taken responsibility for a user in every iteration.
        let mut counter = 0;

        let client = Client::new();
        let mut last_update = 0;

        'get_updates: loop {
            let timeout = 5;
            let mut res = client.get(&format!("https://api.telegram.org/bot{}/getUpdates?timeout={}&offset={}",
                                              args.arg_token,
                                              timeout,
                                              last_update + 1)[..])
                .send().expect("Accessing API");

            let mut body = String::new();
            res.read_to_string(&mut body).expect("Reading API response");
            let json = Json::from_str(&body[..]).expect("Parsing JSON");
            let obj = json.as_object().expect("JSON update should be an object");

            if obj.get("ok").expect("Checking if API result has OK flag").as_boolean().unwrap() {
                let result = obj.get("result").expect("Getting result")
                    .as_array().expect("'result' should be an array");
                for update in result {
                    let update = update.as_object().expect("Getting update");
                    last_update = update.get("update_id").expect("Getting update id")
                        .as_i64().expect("'update_id' should be an int");

                    match update.get("message") {
                        Some(message) => {
                            // 'from' is a User object
                            let from = message.as_object().expect("Getting message")
                                .get("from").expect("Getting from")
                                .as_object().expect("'from' should be an object");
                            let user_id = from.get("id").expect("Getting id")
                                .as_i64().expect("User.'id' should be an int");

                            let mut user_to_stream = user_to_stream.lock().expect("Getting user_stream");
                            let listeners = listeners_mutex.lock().expect("Getting listeners");

                            match user_to_stream.clone().get(&user_id) {
                                Some(listener_id) => {
                                    let tx = listeners.get(listener_id)
                                        .expect(&format!("Getting sender for listener_id {}", listener_id)[..]);
                                    tx.send(RelayMessage::Message(message.clone()))
                                        .expect(&format!("Sending message to a listener: {}", listener_id)[..]);
                                }
                                None => {
                                    let mut listener_id = listeners.keys().nth(counter);
                                    if listener_id.is_none() && counter == 0 {
                                        println!("No listeners are connected\n {}", message);
                                        continue 'get_updates;
                                    } else {
                                        counter = 0;
                                        listener_id = listeners.keys().nth(counter);
                                    }

                                    user_to_stream.insert(user_id, listener_id.expect("'listener_id' should be defined").clone());

                                    counter += 1;
                                }
                            }
                        }
                        None => {}
                    }
                }
            }
        }
    });

    let listeners_mutex = listeners.clone();
    for stream in tcp_listener.incoming() {
        match stream {
            Ok(stream: mut Stream) => {
                let mut reader = BufReader::new(stream);
                let mut line = String::new();
                reader.read_line(&mut line).expect("Listeners should send a line");

                let listener_id =
                    if line == "NEW_LISTENER\n" {
                        random::<i64>()
                    } else if line.starts_with("LISTENER_ID"){
                        line.split(' ').nth(1).expect("LISTENER_ID id")
                            .parse::<i64>()
                            .expect("LISTENER_ID should be followed by a listener id.")
                    } else {
                        println!("Invalid initial line");
                        random::<i64>()
                    };

                let mut listeners = listeners_mutex.lock().unwrap();
                match listeners.get(&listener_id) {
                    Some(tx) => { tx.send(RelayMessage::Stop); } ,
                    None => {},
                };

                let (tx, rx) = channel();
                listeners.insert(listener_id, tx);
                stream.write_all(&format!("LISTENER_ID: {}", listener_id).into_bytes()[..]);

                thread::spawn(move|| {
                    listen(stream, rx);
                });
            }
            Err(_) => {
                println!("Failed to connect");
            }
        }
    }
}

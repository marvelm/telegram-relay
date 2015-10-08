extern crate rustc_serialize;
extern crate docopt;
extern crate rand;
extern crate hyper;

use std::net::{TcpListener, TcpStream};
use std::thread;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use hyper::client::Client;

use rand::random;
use rustc_serialize::json::Json;

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

fn listen(mut stream: TcpStream, rx: Receiver<Json>) {
    loop {
        let message = rx.recv().unwrap();
        let as_raw_json = format!("{}", message.pretty());
        stream.write_all(&as_raw_json.into_bytes()[..]).unwrap();
    }
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    let tcp_listener = TcpListener::bind("127.0.0.1:9001").unwrap();

    let listeners: Arc<Mutex<HashMap<i64, Sender<Json>>>> = Arc::new(Mutex::new(HashMap::new()));
    let user_to_stream: Arc<Mutex<HashMap<i64, i64>>> = Arc::new(Mutex::new(HashMap::new()));

    let listeners_mutex = listeners.clone();
    thread::spawn(move || {
        let user_to_stream = user_to_stream.clone();

        let mut counter = 0;
        let client = Client::new();

        loop {
            let mut res = client.get(&format!("https://api.telegram.org/bot{}/getUpdates", args.arg_token)[..]).send().unwrap();

            let mut body = String::new();
            res.read_to_string(&mut body).unwrap();
            let json = Json::from_str(&body[..]).unwrap();
            let obj = json.as_object().unwrap();

            if obj.get("ok").unwrap().as_boolean().unwrap() {
                let result = obj.get("result").unwrap().as_array().unwrap();
                for update in result {
                    match update.as_object().unwrap().get("message") {
                        Some(message) => {
                            let from = message.as_object().unwrap().get("from").unwrap().as_object().unwrap();
                            let user_id = from.get("id").unwrap().as_i64().unwrap();

                            let mut user_to_stream = user_to_stream.lock().unwrap();
                            let listeners = listeners_mutex.lock().unwrap();

                            match user_to_stream.clone().get(&user_id) {
                                Some(listener_id) => {
                                    let tx = listeners.get(listener_id).unwrap();
                                    tx.send(message.clone()).unwrap();
                                }
                                None => {
                                    counter += 1;
                                    let mut listener_id = listeners.keys().nth(counter);
                                    if listener_id.is_none() {
                                        counter = 0;
                                    }
                                    listener_id = listeners.keys().nth(counter);
                                    user_to_stream.insert(user_id, listener_id.unwrap().clone());
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
            Ok(stream) => {
                let mut listeners = listeners_mutex.lock().unwrap();
                let (tx, rx) = channel();
                listeners.insert(random::<i64>(), tx);
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

extern crate encoding;

use crate::config::{ServerConfig, BUFFER_SIZE};

use encoding::{Encoding, EncoderTrap};
use encoding::all::ASCII;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::io::{Read, Write, Result, BufWriter};
use std::str;
use std::fs::File;
use std::fs;
use console::style;

pub fn run(c: ServerConfig) {
    let listener = TcpListener::bind(&c.address.clone()).unwrap();
    println!("Listening on addr: {}", style(&c.address).green());
    let mut children = vec![];
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let storage = c.storage.clone();
        let builder = thread::Builder::new();
        children.push(builder.spawn(move || {
            println!("New client {} connected",
                     style(stream.peer_addr().unwrap().to_string()).green());
            handle_event(storage, stream);
        }).unwrap());
    }

    for child in children {
        let _ = child.join().unwrap();
    }
}

fn check_ack(mut ack_buf: &mut [u8]) -> String {
    let ack_slice: &str = str::from_utf8(&mut ack_buf).unwrap();
    let mut ack_str = ack_slice.to_string();
    let index: usize = ack_str.rfind('\r').unwrap();
    format!("{:?}", ack_str.split_off(index));
    if ack_str != "ACK"{
        return String::from("error")
    }
    String::from("ACK")
}

fn send_message_size(cmd: &str) -> Result<Vec<u8>> {
    let mut message_size = cmd.len();
    message_size = message_size + 1;
    let message_size_str = message_size.to_string();
    let mut message_size_bytes = ASCII.encode(&message_size_str,
                                              EncoderTrap::Strict).map_err(|x| x.into_owned())
                                                                  .unwrap();
    message_size_bytes.push('\r' as u8);

    Ok(message_size_bytes)
}

fn send_message(cmd: &str) -> Result <Vec<u8>> {
    let message_str = cmd.to_string();
    let mut message_bytes = ASCII.encode(&message_str,
                                         EncoderTrap::Strict).map_err(|x| x.into_owned())
                                                             .unwrap();
    message_bytes.push('\r' as u8);

    Ok(message_bytes)
}

fn receive_message_len(mut msg: &mut [u8]) -> String {
    let mut msg_len = str::from_utf8(&mut msg).unwrap().to_string();

    let mut index = 0;
    for c in msg_len.chars() {
        if c.is_numeric() == true {
            index = index + 1;
        }
    }
    msg_len.truncate(index);
    msg_len
}

fn receive_message(msg_len: &str, stream: &mut TcpStream) -> String {
    let mut remaining_data = msg_len.parse::<i32>().unwrap();
    let mut msg: String = String::new();
    let mut buf = [0u8; BUFFER_SIZE];

    while remaining_data > 0 {
        let r = stream.read(&mut buf);
        match r {
            Ok(n) => {
                msg.push_str(str::from_utf8(&mut buf).unwrap());
                remaining_data = remaining_data - n as i32;
            },
            _ => { break; },
        }
    }

    let index = msg.rfind('\r').unwrap();
    msg.truncate(index);
    msg
}

fn receive_file_impl(file_name: &str, storage: &str,
                     stream: &mut TcpStream) {
    println!("[receive_file_impl] file_name = {}", file_name);
    let mut buf = [0u8; BUFFER_SIZE];

    stream.read(&mut buf).unwrap();
    let file_size = receive_message_len(&mut buf);
    println!("[receive_file_impl] file_size = {}", file_size);

    let ack = send_message("ACK").unwrap();
    stream.write_all(&ack).unwrap();

    let mut fullname = String::from(storage);
    fullname.push_str("/");
    fullname.push_str(&file_name);

    let mut file_buffer = BufWriter::new(File::create(fullname).unwrap());
    let mut remaining_data = file_size.parse::<i32>().unwrap();
    while remaining_data > 0 {
        let slab = stream.read(&mut buf);
        match slab {
            Ok(n) => {
                file_buffer.write(&mut buf).unwrap();
                file_buffer.flush().unwrap();
                println!("wrote {} bytes to {}", n, file_name);
                remaining_data = remaining_data - n as i32;
            }
            _ => { break; }
        }
    }
}

fn send_file(file_name: &str, storage: &str,
             stream: &mut TcpStream) -> String {
    let mut file_exists = false;
    let mut file_size = 0;
    let mut fullpath = String::new();

    for entry in fs::read_dir(storage).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if !path.is_dir() {
            fullpath = String::from(entry.path().to_string_lossy());
            let filename = String::from(str::replace(&fullpath, storage, ""));

            if &filename[1..] == file_name {
                let file = File::open(&fullpath).unwrap();
                file_size = file.metadata().unwrap().len();
                file_exists = true;
            }
        }
    }

    let mut buf = [0u8; BUFFER_SIZE];
    match file_exists {
        true => {
            println!("[send_file] File found");
            let message = "prepare transfer file";
            let send_msg_size = send_message_size(message).unwrap();
            let send_msg = send_message(message).unwrap();

            stream.write_all(&send_msg_size).unwrap();

            stream.read(&mut buf).unwrap();
            if check_ack(&mut buf) != "ACK" {
                println!("[send_file] ACK Failed");
                return String::from("NOK");
            }

            stream.write_all(&send_msg).unwrap();

            stream.read(&mut buf).unwrap();
            if check_ack(&mut buf) != "ACK" {
                println!("[send_file] ACK Failed");
                return String::from("NOK");
            }

            let send_file_size = send_message(&file_size.to_string()).unwrap();
            stream.write_all(&send_file_size).unwrap();

            stream.read(&mut buf).unwrap();
            if check_ack(&mut buf) != "ACK" {
                println!("[send_file] ACK Failed");
                return String::from("NOK");
            }

            let mut remaining_data = file_size as i32;
            let mut file = File::open(fullpath).unwrap();
            while remaining_data != 0 {
                let r = file.read(&mut buf);
                match r {
                    Ok(n) => {
                        if n == 0 {
                            println!("Read file is 0");
                            break;
                        }
                        stream.write_all(&buf).unwrap();
                        println!("Sent {} bytes", n);
                        remaining_data = remaining_data - n as i32;
                    }
                    _ => {}
                }
            }
        },
        false => {
            println!("[send_file] File not found");
            let message = "cannot transfer file";
            let send_msg_size = send_message_size(message).unwrap();
            let send_msg = send_message(message).unwrap();

            stream.write_all(&send_msg_size).unwrap();

            stream.read(&mut buf).unwrap();
            if check_ack(&mut buf) != "ACK" {
                println!("[send_file] ACK Failed");
                return String::from("NOK");
            }

            stream.write_all(&send_msg).unwrap();
        }
    }

    String::from("Ok")
}

fn receive_file(file_name: &str, storage: &str,
             mut stream: &mut TcpStream) {
    let mut buf = [0u8; BUFFER_SIZE];
    stream.read(&mut buf).unwrap();
    let msg_len = receive_message_len(&mut buf);

    let ack = send_message("ACK").unwrap();
    stream.write_all(&ack).unwrap();

    let msg_str = receive_message(&msg_len, &mut stream);
    stream.write_all(&ack).unwrap();
    match msg_str.as_ref(){
        "cannot transfer file" => {
            println!("[receive_file] Cannot transfer file");
        }
        "prepare transfer file" => {
            println!("[receive_file] Prepare transfer file");
            receive_file_impl(file_name, storage, &mut stream);
        }
        _ => {
            println!("[receive_file] Unknown message");
        }
    }
}

fn ls_server(storage: &str, stream: &mut TcpStream) {
    let mut msg = String::new();
    for entry in fs::read_dir(storage).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if !path.is_dir() {
            let fullpath = String::from(entry.path().to_string_lossy());
            let filename = String::from(str::replace(&fullpath, storage, ""));
            let file_name = &filename[1..];

            let file = File::open(fullpath).unwrap();
            let file_size = file.metadata().unwrap().len();
            let file_info = format!("{}  [{} bytes]", file_name, file_size);
            msg.push_str(&file_info);
            msg.push('\n');
        }
    }
    msg.push('\r');

    let send_msg_size = send_message_size(&msg).unwrap();
    let send_msg = send_message(&msg).unwrap();

    stream.write_all(&send_msg_size).unwrap();

    let mut buf = [0u8; BUFFER_SIZE];
    stream.read(&mut buf).unwrap();
    if check_ack(&mut buf) != "ACK" {
        println!("[ls_server] Could not receive ACK from client");
    } else {
        stream.write_all(&send_msg).unwrap();
    }
}

fn handle_event(storage: String, mut stream: TcpStream) {
    loop {
        let mut buf = [0u8; BUFFER_SIZE];

        stream.read(&mut buf).unwrap();
        let msg_len = receive_message_len(&mut buf);

        let ack = send_message("ACK").unwrap();
        stream.write_all(&ack).unwrap();

        let msg_str = receive_message(&msg_len, &mut stream);
        if msg_str.starts_with("get ") {
            send_file(&msg_str[4..], &storage, &mut stream);
        } else if msg_str.starts_with("put ") {
            receive_file(&msg_str[4..], &storage, &mut stream);
        } else {
            match msg_str.as_ref() {
                "ls" => {
                    ls_server(&storage, &mut stream);
                },
                _ => {
                    println!("Unknown command: {}", msg_str);
                },
            }
        }
    }
}


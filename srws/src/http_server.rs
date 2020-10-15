extern crate encoding;

use crate::config::{ServerConfig, ServerFile, BUFFER_SIZE};
use crate::config::{ACK_MESSAGE,
                    PREPARE_TRANSFER_MESSAGE,
                    CANNOT_FIND_FILE_MESSAGE,
                    REMOVED_OK_MESSAGE,
                    REMOVED_NOK_MESSAGE,
                    GOOD,
                    BAD};
use crate::error::{SrwscError, ErrorCode};

use encoding::{Encoding, EncoderTrap};
use encoding::all::ASCII;
use std::net::{TcpListener, TcpStream, Shutdown};
use std::thread;
use std::io::BufWriter;
use std::io::prelude::*;
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

fn check_ack(mut ack_buf: &mut [u8]) -> Result<String, SrwscError> {
    let ack_slice: &str = str::from_utf8(&mut ack_buf).unwrap();
    let mut ack_str = ack_slice.to_string();
    let index: usize = ack_str.rfind('\r').unwrap();
    ack_str.truncate(index);
    if ack_str != ACK_MESSAGE {
        return Err(SrwscError::new(ErrorCode::ErrorAck,
                                   String::from("ACK failed")));
    }
    Ok(String::from(GOOD))
}

fn check_file(file_name: &str, storage: &str) -> ServerFile {
    let mut f = ServerFile::new();

    for entry in fs::read_dir(storage).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if !path.is_dir() {
            let fullpath = String::from(entry.path().to_string_lossy());
            let filename = String::from(str::replace(&fullpath, storage, ""));

            if &filename[1..] == file_name {
                let file = File::open(&fullpath).unwrap();
                f.size = file.metadata().unwrap().len();
                f.name = file_name.to_string();
                f.fullpath = fullpath;
                break;
            }
        }
    }

    f
}

fn encoded_message_size(cmd: &str) -> Result<Vec<u8>, SrwscError> {
    let mut msg_size = cmd.len();
    msg_size = msg_size + 1;
    let msg_size_str = msg_size.to_string();
    let mut msg_size_bytes = ASCII.encode(&msg_size_str, EncoderTrap::Strict)
                                  .map_err(|x| x.into_owned())
                                  .unwrap();
    msg_size_bytes.push('\r' as u8);

    Ok(msg_size_bytes)
}

fn encoded_message(cmd: &str) -> Result <Vec<u8>, SrwscError> {
    let msg_str = cmd.to_string();
    let mut msg_bytes = ASCII.encode(&msg_str, EncoderTrap::Strict)
                             .map_err(|x| x.into_owned())
                             .unwrap();
    msg_bytes.push('\r' as u8);

    Ok(msg_bytes)
}

fn send_ack_message(stream: &mut TcpStream) {
    let ack = encoded_message("ACK").unwrap();
    stream.write_all(&ack).unwrap();
}

fn send_normal_message(msg: &str, stream: &mut TcpStream)
        -> Result<String, SrwscError> {
    let mut buf = [0u8; BUFFER_SIZE];
    let send_msg_size = encoded_message_size(msg).unwrap();
    let send_msg = encoded_message(msg).unwrap();

    stream.write_all(&send_msg_size).unwrap();

    stream.read(&mut buf).unwrap();
    if let Err(e) = check_ack(&mut buf) {
        println!("[send_normal_message] check_ack for size: {}", e);
        return Err(e);
    }

    stream.write_all(&send_msg).unwrap();

    stream.read(&mut buf).unwrap();
    if let Err(e) = check_ack(&mut buf) {
        println!("[send_normal_message] check_ack for message: {}", e);
        return Err(e);
    }

    Ok(String::from(GOOD))
}

fn decoded_message_len(mut msg: &mut [u8]) -> String {
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

fn decoded_message(msg_len: &str, stream: &mut TcpStream) -> String {
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

fn get_message(stream: &mut TcpStream) -> String {
    let mut buf = [0u8; BUFFER_SIZE];
    stream.read(&mut buf).unwrap();
    let msg_len = decoded_message_len(&mut buf);
    if msg_len.is_empty() == true {
        return String::from(BAD);
    }
    send_ack_message(stream);
    let msg = decoded_message(&msg_len, stream);
    send_ack_message(stream);
    msg
}

fn receive_file_impl(file_name: &str, storage: &str,
                     stream: &mut TcpStream) {
    println!("[receive_file_impl] file_name = {}", file_name);
    let mut buf = [0u8; BUFFER_SIZE];

    stream.read(&mut buf).unwrap();
    let file_size = decoded_message_len(&mut buf);
    println!("[receive_file_impl] file_size = {}", file_size);

    send_ack_message(stream);

    let mut fullname = String::from(storage);
    fullname.push_str("/");
    fullname.push_str(&file_name);

    let mut file_buffer = BufWriter::new(File::create(fullname).unwrap());
    let mut remaining_data = file_size.parse::<u64>().unwrap();
    let mut written: i32;
    while remaining_data > 0 {
        let slab = stream.read(&mut buf);
        match slab {
            Ok(n) => {
                if remaining_data < BUFFER_SIZE as u64 {
                    let sbuf = &buf[0 .. remaining_data as usize];
                    file_buffer.write(sbuf).unwrap();
                    written = remaining_data as i32;
                } else {
                    file_buffer.write(&mut buf).unwrap();
                    written = n as i32;
                }
                file_buffer.flush().unwrap();
                remaining_data = remaining_data - written as u64;
            }
            _ => { break; }
        }
    }
    println!("receive file impl end");
}

fn send_file_impl(fullpath: &str, file_size: u64, stream: &mut TcpStream)
        -> String {
    let mut buf = [0u8; BUFFER_SIZE];
    let send_file_size = encoded_message(&file_size.to_string()).unwrap();
    stream.write_all(&send_file_size).unwrap();

    stream.read(&mut buf).unwrap();
    if let Err(e) = check_ack(&mut buf) {
        println!("[send_file_impl] check_ack: {}", e);
        return String::from(BAD);
    }

    let mut remaining_data = file_size;
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
                remaining_data = remaining_data - n as u64;
            }
            _ => {}
        }
    }
    String::from(GOOD)
}

fn send_file(file_name: &str, storage: &str,
             stream: &mut TcpStream) -> String {
    let file_info = check_file(file_name, storage);
    if file_info.size > 0 {
        println!("[send_file] File found");
        match send_normal_message(PREPARE_TRANSFER_MESSAGE, stream) {
            Err(e) => {
                println!("[send_file] Error with {:?}", e);
                return String::from(BAD);
            },
            _ => {},
        }
        send_file_impl(&file_info.fullpath, file_info.size, stream)
    } else {
        println!("[send_file] File is not found");
        let _ = send_normal_message(CANNOT_FIND_FILE_MESSAGE, stream);
        return String::from(BAD);
    }
}

fn receive_file(file_name: &str, storage: &str,
             mut stream: &mut TcpStream) {
    match get_message(stream).as_ref(){
        CANNOT_FIND_FILE_MESSAGE => {
            println!("[receive_file] Cannot transfer file");
        }
        PREPARE_TRANSFER_MESSAGE => {
            println!("[receive_file] Prepare transfer file");
            receive_file_impl(file_name, storage, &mut stream);
        }
        _ => {
            println!("[receive_file] Unknown message");
        }
    }
}

fn remove_file(filename: &str, storage: &str, stream: &mut TcpStream) {
    println!("Not impl");
    let mut msg = String::new();
    let f = check_file(filename, storage);
    if f.size > 0 {
        println!("[remove_file] File found");
        match fs::remove_file(&f.fullpath) {
            Ok(_) => {
                println!("[remove_file] {} is removed", f.name);
                msg.push_str(REMOVED_OK_MESSAGE);
            },
            Err(e) => {
                println!("[remove_file] An error occured: {:?}", e);
                msg.push_str(REMOVED_NOK_MESSAGE);
            },
        }
    } else {
        println!("[remove_file] File is not found");
        msg.push_str(CANNOT_FIND_FILE_MESSAGE);
    }
    let _ = send_normal_message(&msg, stream);
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

    let _ = send_normal_message(&msg, stream);
}

fn handle_event(storage: String, mut stream: TcpStream) {
    loop {
        let msg = get_message(&mut stream);

        if msg.starts_with("get ") {
            send_file(&msg[4..], &storage, &mut stream);
        } else if msg.starts_with("put ") {
            receive_file(&msg[4..], &storage, &mut stream);
        } else if msg.starts_with("rm ") {
            remove_file(&msg[3..], &storage, &mut stream);
        } else {
            match msg.as_ref() {
                BAD => {
                    println!("Received bad message. exit");
                    break;
                }
                "ls" => {
                    ls_server(&storage, &mut stream);
                },
                _ => {
                    println!("Unknown command: {}", msg);
                },
            }
        }
    }
}


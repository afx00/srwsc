extern crate encoding;
extern crate regex;
extern crate console;
extern crate pbr;

use crate::config::{ClientConfig, ServerFile, BUFFER_SIZE};

use encoding::{Encoding, EncoderTrap};
use encoding::all::ASCII;
use std::net::TcpStream;
use std::io::{Error, ErrorKind, BufWriter};
use std::str;
use std::io::prelude::*;
use std::fs;
use std::fs::File;
use regex::Regex;
use console::{Term, style};

fn srwc_prompt() {
    let term = Term::stdout();
    term.write_str("SRWC> ").unwrap();
}

fn help(){
    println!("{}", style("Available SRWC commands:").magenta());
    println!("{} {}\t-> {}", style("get").green(), style("\"filename\"").blue(), style("Download file from server").cyan());
    println!("{} {}\t-> {}", style("put").green(), style("\"filename\"").blue(), style("Upload file to server").cyan());
    println!("{} {}\t-> {}", style("rm").green(), style("\"filename\"").blue(), style("Remove file in server").cyan());
    println!("{}\t\t-> {}", style("ls").green(), style("Show files in server").cyan());
    println!("{}\t\t-> {}", style("help").green(), style("Show available commands").cyan());
    println!("{}\t\t-> {}", style("quit").green(), style("Quit SRWC").cyan());
}

fn ls_response(response: &String) -> Vec<ServerFile>{
    let mut file_list: Vec<ServerFile> = Vec::new();
    let file_name_regex: Regex = Regex::new(r"(.*)(?:\s\s\[.*\sbytes\][\n\r])").unwrap();
    let file_size_regex: Regex = Regex::new(r"(\d+)(?:\sbytes\][\n\r])").unwrap();

    for cp in file_name_regex.captures_iter(response).enumerate() {
        let mut f = ServerFile::new();
        f.name = String::from(&cp.1[1]);
        file_list.push(f);
    }

    for (i, cap) in file_size_regex.captures_iter(response).enumerate() {
        file_list[i].size = String::from(&cap[1]);
    }

    file_list
}

fn send_message_size(cmd: &str) -> Result<Vec<u8>, Error> {
    let mut message_size = cmd.len();
    message_size = message_size + 1;
    let message_size_str = message_size.to_string();
    let mut message_size_bytes = ASCII.encode(&message_size_str,
                                              EncoderTrap::Strict).map_err(|x| x.into_owned()).unwrap();
    message_size_bytes.push('\r' as u8);

    Ok(message_size_bytes)
}

fn send_message(cmd: &str) -> Result <Vec<u8>, Error> {
    let message_str = cmd.to_string();
    let mut message_bytes = ASCII.encode(&message_str,
                                         EncoderTrap::Strict).map_err(|x| x.into_owned()).unwrap();
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

fn receive_message(msg_len: String, stream: &mut TcpStream) -> String{
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

fn check_ack(mut buf: &mut [u8]) -> String {
    let ack_slice: &str = str::from_utf8(&mut buf).unwrap();
    let mut ack_str = ack_slice.to_string();
    let index: usize = ack_str.rfind('\r').unwrap();
    ack_str.truncate(index);
    if ack_str != "ACK"{
        return String::from("error")
    }
    String::from("ACK")
}

fn receive_file(file_name: String, storage: &str,
                stream: &mut TcpStream) {
    println!("[receive_file] file_name = {}", file_name);
    let mut buf = [0u8; BUFFER_SIZE];

    stream.read(&mut buf).unwrap();
    let file_size = receive_message_len(&mut buf);
    println!("[receive_file] file_size = {}", file_size);

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

fn send_file(fullpath: &str, size: i32, stream: &mut TcpStream) {
    let mut remaining_data = size;
    let mut buf = [0u8; BUFFER_SIZE];
    let mut file = File::open(fullpath).unwrap();
    while remaining_data != 0 {
        let r = file.read(&mut buf);
        match r {
            Ok(n) => {
                stream.write_all(&buf).unwrap();
                println!("sent {} file bytes", n);
                remaining_data = remaining_data - n as i32;
            }
            _ => { break; }
        }
    }
}

fn put_get_cmd(command: &str, stream: &mut TcpStream) -> String {
    let mut buf = [0u8; BUFFER_SIZE];
    let filename = String::from(&command[4..]);
    let send_msg_size = send_message_size(command).unwrap();
    let send_msg = send_message(command).unwrap();

    stream.write_all(&send_msg_size).unwrap();

    stream.read(&mut buf).unwrap();
    if check_ack(&mut buf) != "ACK" {
        println!("[put_get_cmd] ACK Failed");
        return String::from("");
    }

    stream.write_all(&send_msg).unwrap();

    filename
}

fn other_cmd(command: &str, stream: &mut TcpStream) -> String {
    let mut buf = [0u8; BUFFER_SIZE];
    let send_msg_size = send_message_size(command).unwrap();
    let send_msg = send_message(command).unwrap();

    stream.write_all(&send_msg_size).unwrap();

    stream.read(&mut buf).unwrap();
    if check_ack(&mut buf) != "ACK" {
        println!("[other_cmd] ACK Failed");
        return String::from("");
    }

    stream.write_all(&send_msg).unwrap();
    check_ack(&mut buf)
}

fn download(command: &str, storage: &str, mut stream: &mut TcpStream)
        -> Result<String, Error> {
    let filename = put_get_cmd(command, stream);
    if filename == "" {
        return Result::Err(Error::new(ErrorKind::Other,
                                      "Filename is empty for download"));
    }
    println!("Try to download as {}", filename);

    let mut buf = [0u8; BUFFER_SIZE];
    stream.read(&mut buf).unwrap();
    let msg_len = receive_message_len(&mut buf);

    let ack = send_message("ACK").unwrap();
    stream.write_all(&ack).unwrap();

    let msg_str = receive_message(msg_len, &mut stream);
    stream.write_all(&ack).unwrap();
    match msg_str.as_ref(){
        "cannot transfer file" => {
            println!("[download] Cannot transfer file");
        }
        "prepare transfer file" => {
            println!("[download] Prepare transfer file");
            receive_file(String::from(filename), storage, &mut stream);
        }
        _ => {
            println!("[download] Unknown message");
        }
    }
    Ok(String::from("Ok"))
}

fn upload(command: &str, storage: &str, stream: &mut TcpStream)
        -> Result<String, std::io::Error> {
    let filename = put_get_cmd(command, stream);
    if filename == "" {
        return Result::Err(Error::new(ErrorKind::Other,
                                      "Filename is empty for upload"));
    }
    println!("Try to upload as {}", filename);

    let mut file_exists = false;
    let mut file_size = 0;
    let mut fullpath = String::new();
    let mut buf = [0u8; BUFFER_SIZE];

    for entry in fs::read_dir(storage).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if !path.is_dir() {
            fullpath = String::from(entry.path().to_string_lossy());
            let file_name = String::from(str::replace(&fullpath, storage, ""));

            if &file_name[1..] == filename {
                let file = File::open(&fullpath).unwrap();
                file_size = file.metadata().unwrap().len();
                file_exists = true;
            }
        }
    }

    match file_exists {
        true => {
            let message = "prepare transfer file";
            let send_msg_size = send_message_size(message).unwrap();
            let send_msg = send_message(message).unwrap();

            stream.write_all(&send_msg_size).unwrap();

            stream.read(&mut buf).unwrap();
            if check_ack(&mut buf) != "ACK" {
                println!("[upload] ACK failed");
                return Result::Err(Error::new(ErrorKind::Other,
                                              "ACK failed"));
            }

            stream.write_all(&send_msg).unwrap();

            stream.read(&mut buf).unwrap();
            if check_ack(&mut buf) != "ACK" {
                println!("[upload] ACK failed");
                return Result::Err(Error::new(ErrorKind::Other,
                                              "ACK failed"));
            }

            let send_file_size = send_message(&file_size.to_string()).unwrap();
            stream.write_all(&send_file_size).unwrap();

            stream.read(&mut buf).unwrap();
            if check_ack(&mut buf) != "ACK" {
                println!("[upload] ACK failed");
                return Result::Err(Error::new(ErrorKind::Other,
                                              "ACK failed"));
            }

            send_file(&fullpath, file_size as i32, stream);
        },
        false => {
            println!("File not found");
            let message = "cannot transfer file";
            let send_msg_size = send_message_size(message).unwrap();
            let send_msg = send_message(message).unwrap();

            stream.write_all(&send_msg_size).unwrap();

            stream.read(&mut buf).unwrap();
            if check_ack(&mut buf) != "ACK" {
                println!("[upload] ACK failed");
                return Result::Err(Error::new(ErrorKind::Other,
                                              "ACK failed"));
            }

            stream.write_all(&send_msg).unwrap();
        }
    }

    Ok(String::from("Ok"))
}

fn ls_server(command: &str, mut stream: &mut TcpStream)
        -> Result<String, Error> {
    if other_cmd(command, stream) != "ACK" {
        println!("[ls_server] ACK failed");
        return Result::Err(Error::new(ErrorKind::Other,
                                      "ACK failed"));
    }

    let mut buf = [0u8; BUFFER_SIZE];
    stream.read(&mut buf).unwrap();
    let msg_len = receive_message_len(&mut buf);

    let ack = send_message("ACK").unwrap();
    stream.write_all(&ack).unwrap();

    let msg_str = receive_message(msg_len, &mut stream);

    Ok(msg_str)
}

pub fn run(c: ClientConfig) {
    let mut stream = TcpStream::connect(&c.address)
        .expect("Could not connect to the server...");
    println!("Successful connection to server({})", style(&c.address).yellow());
    loop {
        srwc_prompt();
        let cmd = std::io::stdin();
        for line in cmd.lock().lines() {
            let command = line.unwrap();

            if command.starts_with("get ") {
                println!("user is trying to download a file");
                match download(&command, &c.storage, &mut stream) {
                    Ok(_) => println!("Download completed"),
                    Err(err) => println!("Download error: {}", err),
                }
            } else if command.starts_with("put ") {
                println!("user is trying to upload a file");
                match upload(&command, &c.storage, &mut stream) {
                    Ok(response) => println!("response: {}", response),
                    Err(err) => println!("An error occurred: {}", err),
                }
            } else {
                match command.as_ref() {
                    "ls" => {
                        match ls_server(&command, &mut stream) {
                            Ok(response) => {
                                let res = ls_response(&response);
                                println!("{}", style("Server files: ").magenta());
                                for entry in res.iter() {
                                    println!("{}  [{} bytes]", style(&entry.name).green(),
                                                               style(&entry.size).cyan());
                                }
                            },
                            Err(err) => println!("{}", err),
                        }
                    },
                    "help" => help(),
                    _ => println!("Unknown command: {}", command),
                }
            }
            break;
        }
    }
}

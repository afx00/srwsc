extern crate encoding;
extern crate regex;
extern crate console;
extern crate pbr;

use crate::config::{ClientConfig, ServerFile, BUFFER_SIZE};
use crate::config::{ACK_MESSAGE,
                    PREPARE_TRANSFER_MESSAGE,
                    CANNOT_FIND_FILE_MESSAGE,
                    REMOVED_OK_MESSAGE,
                    REMOVED_NOK_MESSAGE,
                    GOOD};
use crate::error::{SrwscError, ErrorCode};

use encoding::{Encoding, EncoderTrap};
use encoding::all::ASCII;
use std::net::TcpStream;
use std::io::BufWriter;
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

fn encoded_message_size(cmd: &str) -> Result<Vec<u8>, SrwscError> {
    let mut message_size = cmd.len();
    message_size = message_size + 1;
    let message_size_str = message_size.to_string();
    let mut message_size_bytes = ASCII.encode(&message_size_str, EncoderTrap::Strict)
                                      .map_err(|x| x.into_owned())
                                      .unwrap();
    message_size_bytes.push('\r' as u8);

    Ok(message_size_bytes)
}

fn encoded_message(cmd: &str) -> Result <Vec<u8>, SrwscError> {
    let message_str = cmd.to_string();
    let mut message_bytes = ASCII.encode(&message_str, EncoderTrap::Strict)
                                 .map_err(|x| x.into_owned())
                                 .unwrap();
    message_bytes.push('\r' as u8);

    Ok(message_bytes)
}

fn send_ack_message(stream: &mut TcpStream) {
    let ack = encoded_message("ACK").unwrap();
    stream.write_all(&ack).unwrap();
}

fn send_normal_message(msg: &str, stream: &mut TcpStream)
        -> Result<String, SrwscError> {
    println!("send normal message");
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

fn decoded_message(msg_len: String, stream: &mut TcpStream) -> String {
    let mut remaining_data = msg_len.parse::<i32>().unwrap();
    let mut msg: String = String::new();
    let mut buf = [0u8; BUFFER_SIZE];

    while remaining_data > 0 {
        match stream.read(&mut buf) {
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
    send_ack_message(stream);
    let msg = decoded_message(msg_len, stream);
    send_ack_message(stream);
    msg
}

fn check_ack(mut buf: &mut [u8]) -> Result<String, SrwscError> {
    let ack_slice: &str = str::from_utf8(&mut buf).unwrap();
    let mut ack_str = ack_slice.to_string();
    let index: usize = ack_str.rfind('\r').unwrap();
    ack_str.truncate(index);
    if ack_str != ACK_MESSAGE {
        return Err(SrwscError::new(ErrorCode::ErrorAck,
                                   String::from("ACK failed")));
    }
    Ok(String::from(GOOD))
}

fn receive_file(file_name: &str, storage: &str,
                stream: &mut TcpStream) {
    println!("[receive_file] file_name = {}", file_name);
    let mut buf = [0u8; BUFFER_SIZE];

    stream.read(&mut buf).unwrap();
    let file_size = decoded_message_len(&mut buf);
    println!("[receive_file] file_size = {}", file_size);

    send_ack_message(stream);

    let mut fullname = String::from(storage);
    fullname.push_str("/");
    fullname.push_str(&file_name);

    let mut file_buffer = BufWriter::new(File::create(fullname).unwrap());
    let mut remaining_data = file_size.parse::<u64>().unwrap();
    let mut written: i32;
    while remaining_data > 0 {
        match stream.read(&mut buf) {
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
                println!("wrote {} bytes, remaining {} bytes",
                         written, remaining_data);
            }
            _ => { break; }
        }
    }
}

fn send_file(fullpath: &str, file_size: u64, stream: &mut TcpStream)
        -> Result<String, SrwscError> {
    let send_file_size = encoded_message(&file_size.to_string()).unwrap();
    stream.write_all(&send_file_size).unwrap();

    let mut buf = [0u8; BUFFER_SIZE];
    stream.read(&mut buf).unwrap();
    if let Err(e) = check_ack(&mut buf) {
        println!("[send_file] check_ack: {}", e);
        return Err(e);
    }

    let mut remaining_data = file_size;
    let mut file = File::open(fullpath).unwrap();
    while remaining_data != 0 {
        match file.read(&mut buf) {
            Ok(n) => {
                stream.write_all(&buf).unwrap();
                println!("sent {} file bytes", n);
                remaining_data = remaining_data - n as u64;
            }
            _ => { break; }
        }
    }
    Ok(String::from(GOOD))
}

fn download(command: &str, storage: &str, mut stream: &mut TcpStream)
        -> Result<String, SrwscError> {
    match get_message(stream).as_ref() {
        CANNOT_FIND_FILE_MESSAGE => {
            println!("[download] Cannot transfer file");
            return Err(SrwscError::new(ErrorCode::NotExistFile,
                                       String::from("Not exist file")));
        },
        PREPARE_TRANSFER_MESSAGE => {
            let filename = String::from(&command[4..]);
            println!("[download] Try to download as {}", filename);
            receive_file(&filename, storage, &mut stream);
        },
        _ => {
            println!("[download] Unknown message");
            return Err(SrwscError::new(ErrorCode::ErrorRequest,
                                       String::from("Unknown message")));
        },
    }
    Ok(String::from(GOOD))
}

fn upload(command: &str, storage: &str, stream: &mut TcpStream)
        -> Result<String, SrwscError> {
    let filename = String::from(&command[4..]);
    println!("Try to upload as {}", filename);

    let mut file_exists = false;
    let mut file_size = 0;
    let mut fullpath = String::new();

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
            match send_normal_message(PREPARE_TRANSFER_MESSAGE, stream) {
                Err(e) => return Err(e),
                _ => {},
            }
            return send_file(&fullpath, file_size, stream);
        },
        false => {
            println!("File not found");
            let _ = send_normal_message(CANNOT_FIND_FILE_MESSAGE, stream);
        }
    }

    Err(SrwscError::new(ErrorCode::NotExistFile,
                        String::from("File not found")))
}

fn ls_server(stream: &mut TcpStream) -> Result<String, SrwscError> {
    Ok(get_message(stream))
}

fn rm_server(stream: &mut TcpStream) -> Result<String, SrwscError> {
    match get_message(stream).as_ref() {
        REMOVED_OK_MESSAGE => {
            println!("[rm_server] Removed successfully");
        },
        REMOVED_NOK_MESSAGE => {
            println!("[rm_server] Removed unsuccessfully");
            return Err(SrwscError::new(ErrorCode::ErrorRequest,
                                       String::from("Removed unsuccessflly")))
        },
        CANNOT_FIND_FILE_MESSAGE => {
            println!("[rm_server] Could not file file");
            return Err(SrwscError::new(ErrorCode::NotExistFile,
                                       String::from("File not found")))
        },
        _ => {
            println!("[rm_server] Unknown message");
            return Err(SrwscError::new(ErrorCode::ErrorRequest,
                                       String::from("Unknown message")))
        },
    }
    Ok(String::from(GOOD))
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

            println!("go command");
            match send_normal_message(&command, &mut stream) {
                Err(e) => {
                    println!("Error occured during sending command: {:?}", e);
                    continue;
                },
                _ => {},
            }

            if command.starts_with("get ") {
                match download(&command, &c.storage, &mut stream) {
                    Ok(response) => println!("response: {}", response),
                    Err(err) => println!("Download error: {}", err),
                }
            } else if command.starts_with("put ") {
                match upload(&command, &c.storage, &mut stream) {
                    Ok(response) => println!("response: {}", response),
                    Err(err) => println!("An error occurred: {}", err),
                }
            } else if command.starts_with("rm ") {
                match rm_server(&mut stream) {
                    Ok(response) => println!("response: {}", response),
                    Err(err) => println!("An error occurred: {}", err),
                }
            } else {
                match command.as_ref() {
                    "ls" => {
                        match ls_server(&mut stream) {
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

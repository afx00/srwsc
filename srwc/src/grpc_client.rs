mod pb {
    tonic::include_proto!("srwsc.pb");
}

extern crate console;

use crate::config;
use crate::config::ClientConfig;
use crate::error::{SrwscError, ErrorCode};
use crate::misc;

use pb::srwsc_client::SrwscClient;
use pb::{FileStream, SrwscRequest};

use std::io::prelude::*;
use console::style;
use std::fs::File;
use futures::stream;
use tonic::metadata::MetadataValue;
use std::io::BufWriter;

async fn download(filename: &str,
                  storage: &str,
                  client: &mut SrwscClient<tonic::transport::Channel>)
                  -> Result<(), Box<dyn std::error::Error>> {
    let mut fullname = String::from(storage);
    fullname.push_str("/");
    fullname.push_str(filename);

    let request = tonic::Request::new(
        SrwscRequest {
            filename: String::from(filename)
        },
    );

    let mut stream = client
        .get(request)
        .await?
        .into_inner();

    let mut file = BufWriter::new(File::create(fullname).unwrap());
    while let Some(file_stream) = stream.message().await? {
        file.write(&file_stream.data).unwrap();
        file.flush().unwrap();
    }
    Ok(())
}

async fn upload(filename: &str,
                storage: &str,
                client: &mut SrwscClient<tonic::transport::Channel>)
                -> Result<(), Box<dyn std::error::Error>> {
    let file = misc::check_file(filename, storage);

    println!("file = {:?}", file);
    if file.size > 0 {
        let mut buf = [0u8; config::BUFFER_SIZE];
        let mut f = File::open(&file.fullpath).unwrap();
        let mut read_size: u64 = 0;
        let mut msg: Vec<FileStream> = Vec::new();
        while read_size < file.size {
            match f.read(&mut buf) {
                Ok(n) => {
                    read_size += n as u64;
                    msg.push(FileStream{data: buf[..n].to_vec()});
                }
                _ => { break; }
            }
        }
        let mut request = tonic::Request::new(stream::iter(msg));
        let header_value = MetadataValue::from_str(&file.name).unwrap();
        request.metadata_mut()
            .insert(config::GRPC_METADATA_FILENAME, header_value.clone());
        match client.put(request).await {
            Ok(response) => println!("response = {:?}", response.into_inner()),
            Err(e) => println!("Something wrong: {:?}", e),
        }
    } else {
        return Err(Box::new(SrwscError::new(ErrorCode::NotExistFile,
                                               String::from("Not exist file"))));
    }
    Ok(())
}

async fn rm_file(filename: &str,
                 client: &mut SrwscClient<tonic::transport::Channel>)
                 -> Result<(), Box<dyn std::error::Error>> {
    let request = tonic::Request::new(
        SrwscRequest {
            filename: String::from(filename)
        },
    );
    let response = client
        .remove(request)
        .await?
        .into_inner();
    println!("RESPONSE={:?}", response);
    Ok(())
}

async fn ls_server(client: &mut SrwscClient<tonic::transport::Channel>)
                 -> Result<String, Box<dyn std::error::Error>> {
    let request = tonic::Request::new(
        pb::Empty{},
    );
    let response = client
        .file_list(request)
        .await?
        .into_inner();
    println!("RESPONSE={:?}", response);
    Ok(response.message)
}

#[tokio::main]
pub async fn run(c: ClientConfig)
    -> Result<(), Box<dyn std::error::Error>> {
    let mut addr = String::from(config::GRPC_URL_SCHEMA);
    addr.push_str(&c.address.to_string());
    println!("Connecting on address: {}", style(&addr).green());
    let channel = tonic::transport::Channel::from_shared(
        addr.into_bytes())
        .unwrap()
        .connect()
        .await?;

    let mut client = SrwscClient::new(channel);

    loop {
        misc::srwc_prompt();
        let cmd = std::io::stdin();
        for line in cmd.lock().lines() {
            let command = line.unwrap();

            if command.starts_with("get ") {
                match download(&command[4..], &c.storage, &mut client).await {
                    Ok(_) => println!("Download is completed"),
                    Err(err) => println!("Download error: {}", err),
                }
            } else if command.starts_with("put ") {
                match upload(&command[4..], &c.storage, &mut client).await {
                    Ok(_) => println!("Upload is completed"),
                    Err(err) => println!("An error occurred: {}", err),
                }
            } else if command.starts_with("rm ") {
                match rm_file(&command[3..], &mut client).await {
                    Ok(_) => println!("Remove is Ok"),
                    Err(err) => println!("An error occurred: {}", err),
                }
            } else {
                match command.as_ref() {
                    "ls" => {
                        match ls_server(&mut client).await {
                            Ok(msg) => {
                                let res = misc::file_list_response(&msg);
                                println!("{}", style("Server files: ").magenta());
                                for entry in res.iter() {
                                    println!("{}  [{} bytes]", style(&entry.name).green(),
                                             style(&entry.size).cyan());
                                }
                            },
                            Err(err) => println!("An error occurred: {}", err),
                        }
                    },
                    "help" => misc::srwc_help(),
                    _ => println!("Unknown command: {}", command),
                }
            }
            break;
        }
    }
}
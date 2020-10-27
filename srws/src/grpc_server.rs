mod pb {
    tonic::include_proto!("srwsc.pb");
}

use crate::config;
use crate::misc;

use tonic::{transport::Server, Request, Response, Status, Streaming};
use pb::srwsc_server::{Srwsc, SrwscServer};
use pb::{SrwscRequest, SrwscResponse, FileStream};

use console::style;
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::io::prelude::*;
use futures::{stream, StreamExt};
use tokio::sync::mpsc;

static mut STORAGE: &str = config::DEFAULT_STORAGE;

fn send_file(filename: &str)
    -> Vec<FileStream> {
    let f: config::ServerFile;
    unsafe {
        f = misc::check_file(filename, STORAGE);
    }

    let mut msg: Vec<FileStream> = Vec::new();
    let mut buf = [0u8; config::BUFFER_SIZE];
    let mut remaining_data = f.size;
    let mut file = File::open(f.fullpath).unwrap();
    while remaining_data != 0 {
        match file.read(&mut buf) {
            Ok(n) => {
                if n == 0 {
                    println!("Read file is 0");
                    break;
                }
                msg.push(FileStream{data: buf[..n].to_vec()});
                remaining_data = remaining_data - n as u64;
            },
            _ => break,
        }
    }
    msg
}

async fn receive_file(filename: &str, mut stream: Streaming<FileStream>)
    -> String {
    let mut fullname = String::new();
    unsafe  {
        fullname.push_str(STORAGE);
    }
    fullname.push_str("/");
    fullname.push_str(&filename);
    println!("enter receive_file");
    let mut file_buffer = BufWriter::new(File::create(fullname).unwrap());

    while let Some(msg) = stream.next().await {
        let msg = msg.unwrap();
        let _ = file_buffer.write(&msg.data).unwrap();
    }

    String::from("Ok")
}

fn remove_file(filename: &str)
    -> String {
    let f: config::ServerFile;
    unsafe {
        f = misc::check_file(filename, STORAGE);
    }
    if f.size > 0 {
        println!("[remove_file] File found");
        match fs::remove_file(&f.fullpath) {
            Ok(_) => {
                println!("[remove_file] {} is removed", f.name);
                config::REMOVED_OK_MESSAGE.to_string()
            },
            Err(e) => {
                println!("[remove_file] An error occured: {:?}", e);
                config::REMOVED_NOK_MESSAGE.to_string()
            },
        }
    } else {
        println!("[remove_file] File is not found");
        config::CANNOT_FIND_FILE_MESSAGE.to_string()
    }
}

#[derive(Default)]
pub struct ServerImpl {}

#[tonic::async_trait]
impl Srwsc for ServerImpl {

    type ServerFileStream = mpsc::Receiver<Result<FileStream, Status>>;

    async fn get(&self, request: Request<SrwscRequest>)
        -> Result<Response<Self::ServerFileStream>, Status> {
        let filename = &request.get_ref().filename;
        let file_streams = send_file(filename);
        let (mut tx, rx) = mpsc::channel(8);
        tokio::spawn(async move {
            for file_stream in &file_streams[..] {
                println!("  => send {:?}", file_stream);
                tx.send(Ok(file_stream.clone())).await.unwrap();
            }

            println!("Done sending");
        });
        Ok(Response::new(rx))
    }

    async fn put(&self, stream: Request<Streaming<FileStream>>)
        -> Result<Response<SrwscResponse>, Status> {
        let filename = stream.metadata()
                                               .get(config::GRPC_METADATA_FILENAME)
                                               .cloned()
                                               .unwrap();
        Ok(Response::new(SrwscResponse{
            message: receive_file(filename.to_str().unwrap(),
                                  stream.into_inner()).await,
        }))
    }

    async fn remove(&self, request: Request<SrwscRequest>)
        -> Result<Response<SrwscResponse>, Status> {
        let filename = &request.get_ref().filename;
        Ok(Response::new(SrwscResponse{
            message: remove_file(filename),
        }))
    }

    async fn file_list(&self, _: Request<pb::Empty>)
        -> Result<Response<SrwscResponse>,Status> {
        unsafe {
            Ok(Response::new(SrwscResponse {
                message: misc::get_file_list(STORAGE),
            }))
        }
    }
}

#[tokio::main]
pub async fn run(c: config::ServerConfig)
    -> Result<(), Box<dyn std::error::Error>> {
    let s = ServerImpl::default();
    unsafe {
        STORAGE = Box::leak(c.storage.into_boxed_str());
    }
    println!("Listening on address: {}", style(&c.address).green());
    Server::builder()
        .add_service(SrwscServer::new(s))
        .serve(c.address)
        .await?;
    Ok(())
}

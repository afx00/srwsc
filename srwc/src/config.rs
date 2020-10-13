use std::net::SocketAddr;
  
pub const CLIENT_NAME: &str = "srwc";
pub const VERSION: &str = "0.1.0";

pub const DEFAULT_TYPE: ServerType = ServerType::HTTP;
pub const DEFAULT_ADDR: &str = "0.0.0.0:1417";
pub const DEFAULT_STORAGE: &str = "/tmp/srwc";
pub const BUFFER_SIZE: usize = 8;


#[derive(Debug)]
pub enum ServerType {
    HTTP,
    HTTPS,
    GRPC,
}

#[derive(Debug)]
pub struct ClientConfig {
    pub server_type: ServerType,
    pub address: SocketAddr,
    pub storage: String,
}

impl ClientConfig {
    pub fn new() -> Self {
        ClientConfig {
            server_type: DEFAULT_TYPE,
            address: DEFAULT_ADDR.parse().expect("Unable to parse socket address"),
            storage: DEFAULT_STORAGE.to_string(),
        }
    }
}

pub struct ServerFile {
    pub name: String,
    pub size: String,
}

impl ServerFile {
    pub fn new() -> Self {
        ServerFile {
            name: String::from(""),
            size: String::from(""),
        }
    }
}

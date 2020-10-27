use std::net::SocketAddr;
  
pub const CLIENT_NAME: &str = "srwc";
pub const VERSION: &str = "0.1.0";

pub const DEFAULT_TYPE: ServerType = ServerType::HTTP;
pub const DEFAULT_ADDR: &str = "0.0.0.0:1417";
pub const DEFAULT_STORAGE: &str = "/tmp/srwc";
pub const BUFFER_SIZE: usize = 8;

pub const ACK_MESSAGE: &str              = "ACK";
pub const PREPARE_TRANSFER_MESSAGE: &str = "prepare transfer file";
pub const CANNOT_FIND_FILE_MESSAGE: &str = "cannot find file";
pub const REMOVED_OK_MESSAGE: &str       = "removed ok";
pub const REMOVED_NOK_MESSAGE: &str      = "removed nok";

pub const GRPC_METADATA_FILENAME: &str  = "filename";

pub const GOOD: &str = "OK";
#[allow(dead_code)]
pub const BAD: &str  = "NOK";

pub const GRPC_URL_SCHEMA: &str = "http://";

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

#[derive(Debug)]
pub struct ServerFile {
    pub fullpath: String,
    pub name: String,
    pub size: u64,
}

impl ServerFile {
    pub fn new() -> Self {
        ServerFile {
            fullpath: String::from(""),
            name: String::from(""),
            size: 0,
        }
    }
}

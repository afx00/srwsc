use std::net::SocketAddr;

pub const SERVER_NAME: &str = "srws";
pub const VERSION: &str = "0.1.0";

pub const DEFAULT_TYPE: ServerType = ServerType::HTTP;
pub const DEFAULT_ADDR: &str = "0.0.0.0:1417";
pub const DEFAULT_STORAGE: &str = "/tmp/srws";
pub const BUFFER_SIZE: usize = 8;

pub const ACK_MESSAGE: &str              = "ACK";
pub const PREPARE_TRANSFER_MESSAGE: &str = "prepare transfer file";
pub const CANNOT_FIND_FILE_MESSAGE: &str = "cannot find file";
pub const REMOVED_OK_MESSAGE: &str       = "removed ok";
pub const REMOVED_NOK_MESSAGE: &str      = "removed nok";

pub const GOOD: &str = "OK";
pub const BAD: &str  = "NOK";


#[derive(Debug)]
pub enum ServerType {
    HTTP,
    HTTPS,
    GRPC,
}

#[derive(Debug)]
pub struct ServerConfig {
    pub server_type: ServerType,
    pub address: SocketAddr,
    pub storage: String,
}

impl ServerConfig {
    pub fn new() -> Self {
        ServerConfig {
            server_type: DEFAULT_TYPE,
            address: DEFAULT_ADDR.parse().expect("Unable to parse socket address"),
            storage: DEFAULT_STORAGE.to_string(),
        }
    }
}

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


use std::net::SocketAddr;

pub const SERVER_NAME: &str = "srws";
pub const VERSION: &str = "0.1.0";

pub const DEFAULT_TYPE: ServerType = ServerType::HTTP;
pub const DEFAULT_ADDR: &str = "0.0.0.0:1417";
pub const DEFAULT_STORAGE: &str = "/tmp/srws";
pub const BUFFER_SIZE: usize = 8;


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

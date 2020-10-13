mod config;
mod http_client;

use std::process;
use ace::App;

fn main() {
    let c = get_start_info();
    match c {
        Some(info) => {
            println!("{:?}", info);
            match &info.server_type {
                config::ServerType::HTTP => http_client::run(info),
                _ => println!("Not implement"),
            }
        },
        None => println!("argument is none"),
    }
}

fn get_start_info() -> Option<config::ClientConfig> {
    let app = App::new()
        .config(config::CLIENT_NAME, config::VERSION)
        .cmd("start", "Start client with user config")
        .cmd("help", "Print help information")
        .cmd("version", "Print version information")
        .opt("-t", "Set server type (Use one of http, https, grpc)")
        .opt("-a", "Set the connecting address and port on server")
        .opt("-r", "Set the root directory");

    if let Some(cmd) = app.command() {
        match cmd.as_str() {
            "start" => {
                let mut c = config::ClientConfig::new();
                let server_type = app
                    .value("-t")
                    .map(|values| {
                        if values.len() != 1 {
                            println!("-t value: [SERVER TYPE(http, https, grpc)]");
                            process::exit(-1);
                        }
                        values[0].clone()
                    });
                match server_type {
                    Some(t) => {
                        match t.as_str() {
                            "http" => c.server_type = config::ServerType::HTTP,
                            "https" => c.server_type = config::ServerType::HTTPS,
                            "grpc" => c.server_type = config::ServerType::GRPC,
                            _ => {
                                println!("1 -t value: [SERVER TYPE(http, https, grpc)]");
                                process::exit(-1);
                            }
                        }
                    },
                    None => println!("Use default value for server type"),
                }

                let addr = app
                    .value("-a")
                    .map(|values| {
                        if values.len() != 1 {
                            println!("-a value: [ADDRESS:PORT]");
                            process::exit(-1);
                        }
                        values[0].clone()
                    });
                match addr {
                    Some(a) => c.address = a.parse()
                                            .expect("Unable to parse socket address"),
                    None => println!("Use default value for connecting address"),
                }

                let storage = app
                    .value("-r")
                    .map(|values| {
                        if values.len() != 1 {
                            println!("-r value: [DIR]");
                            process::exit(-1);
                        }
                        values[0].clone()
                    });
                match storage {
                    Some(s) => c.storage = s,
                    None => println!("Use default value for storage"),
                }
                Some(c)
            }
            "help" => {
                app.print_help();
                None
            }
            "version" => {
                app.print_version();
                None
            }
            _ => {
                app.print_error_try("help");
                None
            }
        }
    } else {
        None
    }
}

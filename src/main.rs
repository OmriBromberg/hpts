use std::collections::HashMap;
use clap::{Command, Arg};
use futures::FutureExt;
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::TcpListener;

use env_logger::Builder;
use log::LevelFilter;
use log::{debug, error, info};

mod hpts;
use hpts::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut logger_builder = Builder::new();

    let matches = Command::new("hpts")
        .version("1.0")
        .author("Yongsheng Xu")
        .about("Turn your socks proxies into http proxy")
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .help("Port number to use")
                .required(true),
        )
        .arg(
            Arg::new("verbosity")
                .short('v')
                .help("Sets the level of verbosity")
                .action(clap::ArgAction::Count),
        )
        .arg(
            Arg::new("socks")
                .short('s')
                .long("socks")
                .help("Map of environment name to address (e.g., env1=127.0.0.1:8080)")
                .required(true)
                .action(clap::ArgAction::Append),
        )
        .get_matches();

    let port: u16 = matches
        .get_one::<String>("port")
        .expect("Port is required")
        .parse()
        .expect("Port must be a valid integer");

    // Parse the verbosity argument
    let verbosity = matches.get_count("verbosity");

    // Parse the socks argument into a HashMap
    let socks: HashMap<String, SocketAddr> = matches
        .get_many::<String>("socks")
        .unwrap_or_default()
        .map(|pair| {
            let parts: Vec<&str> = pair.split('=').collect();
            if parts.len() != 2 {
                panic!("Invalid format for socks: {}. Expected format is key=value.", pair);
            }
            let addr: SocketAddr = parts[1].parse().expect("Invalid socket address format");
            (parts[0].to_string(), addr)
        })
        .collect();

    let config = Arc::new(HptsConfig { socks5_addrs: socks });

    let http_proxy_sock = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
    let level = match verbosity {
        0 => LevelFilter::Error,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        3 | _ => LevelFilter::Trace,
    };

    logger_builder.filter_level(level);
    logger_builder.init();

    info!("http server listening on port {}", port);

    let mut listener = TcpListener::bind(http_proxy_sock).await?;
    loop {
        let (socket, _addr) = listener.accept().await?;
        debug!("accept from client: {}", _addr);
        let ctx = HptsContext::new(config.clone(), socket);
        let task = hpts_bridge(ctx).map(|r| {
            if let Err(e) = r {
                error!("{}", e);
            }
        });
        tokio::spawn(task);
    }
}

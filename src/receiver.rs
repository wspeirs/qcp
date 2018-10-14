use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream};

use config::Configuration;
use transport::Transport;

pub struct Receiver {
}

impl Receiver {
    pub fn new(config: Configuration) -> impl Transport {
        let bind_stream = TcpListener::bind("127.0.0.1:1234").unwrap();

        let (stream, addr) = bind_stream.accept().unwrap();

        info!("Got connection from: {}", addr);

        return stream;
    }

}
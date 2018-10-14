use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};

use config::Configuration;
use transport::Transport;

pub struct Sender {
}

impl Sender {
    pub fn new(config: Configuration) -> impl Transport {
        let stream = TcpStream::connect("127.0.0.1:1234").unwrap();
        info!("Opened connection to: {}", stream.peer_addr().unwrap());

        return stream
    }

}
use std::io::{Read, Write, Error as IOError};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream, TcpListener};

use config::Configuration;
use transport::Transport;


impl Transport for TcpStream {
    fn read(&mut self, buf: &mut[u8]) -> Result<usize, IOError> {
        return Read::read(self, buf);
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), IOError> {
        return Write::write_all(self, buf);
    }
}

pub struct Sender { }

impl Sender {
    pub fn new(config: Configuration) -> impl Transport {
        let stream = TcpStream::connect("127.0.0.1:1234").unwrap();
        info!("Opened connection to: {}", stream.peer_addr().unwrap());

        return stream
    }

}

pub struct Receiver { }

impl Receiver {
    pub fn new(config: Configuration) -> impl Transport {
        let bind_stream = TcpListener::bind("127.0.0.1:1234").unwrap();

        let (stream, addr) = bind_stream.accept().unwrap();

        info!("Got connection from: {}", addr);

        return stream;
    }

}
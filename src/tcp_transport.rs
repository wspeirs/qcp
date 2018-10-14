use transport::Transport;
use std::net::TcpStream;
use std::io::{Read, Write, Error as IOError};


impl Transport for TcpStream {
    fn read(&mut self, buf: &mut[u8]) -> Result<usize, IOError> {
        return Read::read(self, buf);
    }

    fn write_all(&mut self, buf: &mut[u8]) -> Result<(), IOError> {
        return Write::write_all(self, buf);
    }
}
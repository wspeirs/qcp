use std::net::{UdpSocket, SocketAddr};
use std::io::{Error as IOError, ErrorKind};
use std::time::Duration;

use transport::Transport;

const MAX_BUFFER_SIZE :usize = 1500;  // max size of a buffer

use flatbuffers::FlatBufferBuilder;
use message_generated::bbr::{get_root_as_message, Message, MessageArgs, Type};

pub struct BBRTransport {
    socket: UdpSocket,
    seq_num: u64
}

impl BBRTransport {
    pub fn connect(addr: SocketAddr) -> Result<BBRTransport, IOError> {
        let local_addr = SocketAddr::new("0.0.0.0".parse().unwrap(), addr.port());
        let socket = UdpSocket::bind(local_addr)?;

        // set the read and write timeouts to 3s
        socket.set_read_timeout(Some(Duration::new(3, 0)));
        socket.set_write_timeout(Some(Duration::new(3, 0)));

        // "connect" the socket to the end-point address
        socket.connect(addr).expect("Error connecting UDP socket");

        // construct the Connect message
        let mut fbb = FlatBufferBuilder::new_with_capacity(MAX_BUFFER_SIZE);

        let msg = Message::create(&mut fbb, &MessageArgs { msg_type: Type::Connect, seq_num: 0, payload: None });

        fbb.finish(msg, None);
        let msg_data = fbb.finished_data();

        // send the connection message
        socket.send(msg_data).expect("Could not send connect message");

        let mut buf = vec![0; MAX_BUFFER_SIZE];
        socket.recv(&mut buf)?;

        let ack = get_root_as_message(&buf);

        if ack.msg_type() != Type::Acknowledge {
            return Err(IOError::new(ErrorKind::ConnectionAborted, "Did not get Acknowledge on Connect"));
        }

        if ack.seq_num() != 0 {
            return Err(IOError::new(ErrorKind::InvalidData, "Acknowledged wrong sequence number"));
        }

        return Ok(BBRTransport { socket, seq_num: 1 });
    }
}

/*
impl Transport for BBRTransport {
    /// Read up to buf.len() bytes from the underlying transport
    fn read(&mut self, buf: &mut[u8]) -> Result<usize, IOError> {

    }

    /// Write all buf.len() bytes to the underlying transport
    fn write_all(&mut self, buf: &mut[u8]) -> Result<usize, IOError> {

    }
}
*/


#[cfg(test)]
mod tests {
    use std::u64::MAX;

    use flatbuffers::FlatBufferBuilder;
    use message_generated::bbr::{Message, MessageArgs, Type};


    pub fn buf2string(buf: &[u8]) -> String {
        let mut ret = String::new();

        for &b in buf {
            ret.push_str(format!("{:02X} ", b).as_str());
        }

        ret
    }


    #[test]
    fn encode() {
        let mut fbb = FlatBufferBuilder::new_with_capacity(1500);

        let buf = fbb.create_vector(&[0xBBu8; 1465]);

        let msg = Message::create(&mut fbb, &MessageArgs { msg_type: Type::Message, seq_num: 0, payload: Some(buf) });

        fbb.finish(msg, None);

        let msg_buf = fbb.finished_data();

        println!("LEN: {}", msg_buf.len());
        println!("{}", buf2string(msg_buf));
    }
}

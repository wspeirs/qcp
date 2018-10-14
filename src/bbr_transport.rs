use std::net::{UdpSocket, SocketAddr};
use std::io::{Error as IOError, ErrorKind};
use std::time::Duration;

use rmp_serde::encode::to_vec;
use rmp_serde::decode::from_slice;

use transport::Transport;

const MAX_BUFFER_SIZE :usize = 1500;  // max size of a buffer

use message_generated::bbr::Message;

pub struct BBRTransport {
    socket: UdpSocket
}

impl BBRTransport {
    pub fn connect(addr: SocketAddr) -> Result<BBRTransport, IOError> {
        let local_addr = SocketAddr::new("0.0.0.0".parse().unwrap(), addr.port());
        let socket = UdpSocket::bind(local_addr)?;

        socket.connect(addr).expect("Error connecting UDP socket");

//        let buf = to_vec(&Message::Connect(0)).expect("Error serializing message");
//
//        socket.send(&buf).expect("Could not send connect message");
//
//        socket.set_read_timeout(Some(Duration::new(3, 0)));
//
//        let mut buf = vec![0; MAX_BUFFER_SIZE];
//        socket.recv(&mut buf)?;
//
//        let ack = from_slice(&buf);
//
//        if let Result::Err(e) = ack {
//            return Err(IOError::new(ErrorKind::InvalidData, format!("Error decoding message: {:?}", e)));
//        }
//
//        if let Message::Acknowledge(s) = ack.unwrap() {
//            if s != 0 {
//                return Err(IOError::new(ErrorKind::InvalidData, "Acknowledged wrong sequence number"));
//            }
//        } else {
//            return Err(IOError::new(ErrorKind::ConnectionAborted, "Did not get Acknowledge on Connect"));
//        }

        return Ok(BBRTransport { socket });
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
    use rmp_serde::encode::to_vec;
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

        let msg = Message::create(&mut fbb, &MessageArgs { type_: Type::Message, seq_num: 0, payload: Some(buf) });

        fbb.finish(msg, None);

        let msg_buf = fbb.finished_data();

        println!("LEN: {}", msg_buf.len());
        println!("{}", buf2string(msg_buf));
    }
}

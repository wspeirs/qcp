use std::net::{UdpSocket, SocketAddr};
use std::io::{Error as IOError, ErrorKind};
use std::time::{Instant, Duration};
use std::sync::{Mutex, Arc};
use std::thread;

use transport::Transport;
use sliding_window::SlidingWindow;

const MAX_PACKET_SIZE :usize = 1500;    // max size of a packet to be sent over the wire
const MAX_PAYLOAD_SIZE :usize = 1465;   // max payload size to ensure the packet is <= MAX_PACKET_SIZE

use flatbuffers::FlatBufferBuilder;
use message_generated::bbr::{get_root_as_message, Message, MessageArgs, Type};

pub fn buf2string(buf: &[u8]) -> String {
    let mut ret = String::new();

    for &b in buf {
        ret.push_str(format!("{:02X} ", b).as_str());
    }

    ret
}

pub struct Sender {
    socket: UdpSocket,
    remote_addr: SocketAddr,
    seq_num: u64,
    window: Arc<SlidingWindow<(Instant, Vec<u8>)>>
}

pub struct Receiver {
    socket: UdpSocket,
    remote_addr: SocketAddr,
    window: Arc<SlidingWindow<Vec<u8>>>
}

/// Constructs a simple message w/out a payload
fn construct_message<'a>(msg_type: Type, seq_num: u64) -> FlatBufferBuilder<'a> {
    let mut fbb = FlatBufferBuilder::new_with_capacity(MAX_PACKET_SIZE);

    let msg = Message::create(&mut fbb, &MessageArgs { msg_type, seq_num, payload: None });

    fbb.finish(msg, None);

    // TODO: Remove the need to return the FBB
    return fbb;
}

impl Sender {
    /// Connect, via BBR, to a remote host
    pub fn connect(remote_addr: SocketAddr) -> Result<impl Transport, IOError> {
        let local_addr = SocketAddr::new("0.0.0.0".parse().unwrap(), remote_addr.port());
        let socket = UdpSocket::bind(local_addr)?;

        // set the read and write timeouts to 3s
        socket.set_read_timeout(Some(Duration::new(3, 0)))?;
        socket.set_write_timeout(Some(Duration::new(3, 0)))?;

        // construct the Connect message
        let msg_data = construct_message(Type::Connect, 0);
        let msg_data = msg_data.finished_data();

        if msg_data.len() > MAX_PACKET_SIZE {
            panic!("Packet size too large: {}", msg_data.len());
        }

        // send the connection message
        socket.send_to(&msg_data, remote_addr).expect("Could not send connect message");

        let mut buf = vec![0; MAX_PACKET_SIZE];

        for i in 0..3 {
            let ret = socket.recv_from(&mut buf);

            debug!("{}: {:?}", i, ret);

            if let Result::Err(e) = ret {
                // check for other errors than a blocking one
                if e.kind() != ErrorKind::WouldBlock {
                    return Err(e);
                    // check to see if we've tried enough time
                } else if i >= 2 {
                    return Err(IOError::new(ErrorKind::ConnectionAborted, "Did not get Acknowledge on Connect"));
                }
            } else {
                break; // it all worked!
            }
        }

        debug!("RET: {}", buf2string(&buf));

        let ack = get_root_as_message(&buf);

        if ack.msg_type() != Type::Acknowledge {
            return Err(IOError::new(ErrorKind::ConnectionAborted, "Got other message type than Acknowledge on Connect"));
        }

        if ack.seq_num() != 0 {
            return Err(IOError::new(ErrorKind::InvalidData, "Acknowledged wrong sequence number"));
        }

        let window = Arc::new(SlidingWindow::new(1024));

        let recv_socket = socket.try_clone()?;
        let recv_window = window.clone();

        thread::spawn(move || {
            // we'll only wait for 1s for an Ack
            recv_socket.set_read_timeout(Some(Duration::from_secs(1))).expect("Could not set read timeout");

            let mut buf = vec![0; MAX_PACKET_SIZE];

            loop {
                // attempt to read an ack
                let res = recv_socket.recv_from(&mut buf);

                // waited for an Ack, but didn't come
                if let Err(e) = res {
                    if e.kind() != ErrorKind::WouldBlock {
                        panic!("Unknown error reading ACK: {:?}", e);
                    }

                    // find the first one that matches the predicate
                    let loc = recv_window.find_first(|t :&(Instant, Vec<u8>)| t.0.elapsed() > Duration::from_secs(3));

                    // we're able to find any old enough, loop back around
                    if loc.is_none() {
                        continue;
                    }

                    let loc = loc.unwrap() as u64;

                    // remove the packet from the window, so we can update the time
                    let (_, packet) = recv_window.remove(loc).expect("Error removing item we previously found");

                    // re-send the packet
                    recv_socket.send_to(&packet, remote_addr);

                    // re-insert the packet with an updated timeout
                    recv_window.insert(loc, (Instant::now(), packet));
                } else if res.is_ok() {
                    // otherwise, we got a message
                    let (amt, _) = res.unwrap();
                    let ack = get_root_as_message(&buf[0..amt]);

                    if ack.msg_type() != Type::Acknowledge {
                        panic!("Got non-ack message");
                    }

                    // remove it from the sliding window
                    let (sent_time, _) = recv_window.remove(ack.seq_num()).expect("Acknowledging bad sequence number");

                    // TODO: deal with the instant values
                }
            }
        });

        return Ok(Sender { socket, remote_addr, seq_num: 1, window });
    }
}

impl Receiver {
    /// Listens for an incoming connection
    pub fn listen(port: u16) -> Result<impl Transport, IOError> {
        let socket = UdpSocket::bind(SocketAddr::new("0.0.0.0".parse().unwrap(), port))?;

        // set the write timeouts to 3s
        socket.set_write_timeout(Some(Duration::new(3, 0)))?;

        let mut buf = vec![0; MAX_PACKET_SIZE];
        let (buf_size, remote_addr) = socket.recv_from(&mut buf)?;

        let msg = get_root_as_message(&buf);

        if msg.msg_type() != Type::Connect {
            return Err(IOError::new(ErrorKind::ConnectionAborted, "Got non-connect message"));
        }

        // construct the ACK message
        let ack_data = construct_message(Type::Acknowledge, msg.seq_num());
        let ack_data = ack_data.finished_data();

        // send the ACK message
        socket.send_to(ack_data, remote_addr);

        let window = Arc::new(SlidingWindow::new(1024));

        return Ok(Receiver { socket, remote_addr, window });
    }
}

impl Transport for Sender {
    fn read(&mut self, buf: &mut[u8]) -> Result<usize, IOError> {
        panic!("Not implemented");
    }

    fn write_all(&mut self, buf: &mut[u8]) -> Result<(), IOError> {
        let chunk_it = buf.chunks(MAX_PAYLOAD_SIZE);

        for chunk in chunk_it {
            // construct the message w/the payload
            let mut fbb = FlatBufferBuilder::new_with_capacity(MAX_PACKET_SIZE);
            let payload = Some(fbb.create_vector(chunk));
            let msg = Message::create(&mut fbb, &MessageArgs { msg_type: Type::Message, seq_num: self.seq_num, payload });

            fbb.finish(msg, None);
            let msg_buf = fbb.finished_data().to_vec();

            let mut end = { self.window.window().1 };

            // wait for a slot in the window
            while end <= self.seq_num {
                thread::yield_now();

                end = { self.window.window().1 };
            }

            {
                self.socket.send_to(&msg_buf, self.remote_addr);

                self.window.insert(self.seq_num, (Instant::now(), msg_buf));
            }

        }

        return Ok( () );
    }
}

impl Transport for Receiver {
    fn read(&mut self, buf: &mut[u8]) -> Result<usize, IOError> {
        return Ok(1);
    }

    fn write_all(&mut self, buf: &mut[u8]) -> Result<(), IOError> {
        panic!("Not implemented");
    }
}

//impl Transport for BBRTransport {
//    /// Read up to buf.len() bytes from the underlying transport
//    fn read(&mut self, buf: &mut[u8]) -> Result<usize, IOError> {
//        return Ok(1);
//    }
//
//    /// Write all buf.len() bytes to the underlying transport
//    fn write_all(&mut self, buf: &mut[u8]) -> Result<(), IOError> {
//
//        return Ok( () );
//    }
//}


#[cfg(test)]
mod tests {
    use std::u64::MAX;
    use simplelog::{TermLogger, LevelFilter, Config};

    use bbr_transport::{Sender, Receiver, buf2string};
    use std::net::SocketAddr;

    use flatbuffers::FlatBufferBuilder;
    use message_generated::bbr::{get_root_as_message, Message, MessageArgs, Type};


    #[test]
    fn connect() {
        TermLogger::init(LevelFilter::Debug, Config::default()).unwrap();

        let t = Sender::connect("192.168.1.123:1234".parse().unwrap());

        assert!(t.is_err());

        println!("{:?}", t.err());
    }

    #[test]
    fn listen() {
        TermLogger::init(LevelFilter::Debug, Config::default()).unwrap();

        let t = Receiver::listen(1234);
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

    #[test]
    fn decode_fail() {
        let msg_buf = vec![0; 1500];

        let msg = get_root_as_message(&msg_buf);

        println!("{:?}", msg.msg_type());
    }
}

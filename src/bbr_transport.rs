use std::net::{UdpSocket, SocketAddr};
use std::io::{Error as IOError, ErrorKind};
use std::time::{Instant, Duration};
use std::sync::{Mutex, Arc};
use std::thread;

use transport::Transport;
use sliding_window::SlidingWindow;
use config::Configuration;
use socket::Socket;

const MAX_PACKET_SIZE :usize = 1500;    // max size of a packet to be sent over the wire
pub const MAX_PAYLOAD_SIZE :usize = 1452;   // max payload size to ensure the packet is <= MAX_PACKET_SIZE

use flatbuffers::FlatBufferBuilder;
use message_generated::bbr::{get_root_as_message, Message, MessageArgs, Type};

pub fn buf2string(buf: &[u8]) -> String {
    let mut ret = String::new();

    for &b in buf {
        ret.push_str(format!("{:02X} ", b).as_str());
    }

    ret
}

pub struct Sender<T> {
    socket: T,
    remote_addr: SocketAddr,
    seq_num: u64,
    window: Arc<SlidingWindow<(Instant, Vec<u8>)>>
}

pub struct Receiver<T> {
    socket: T,
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

impl <T: 'static> Sender<T> where T: Socket + Send + Sync {
    /// Connect, via BBR, to a remote host
    pub fn connect(socket: T, config: &Configuration) -> Result<impl Transport, IOError> {
        let remote_addr = config.addr();

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

        let window = Arc::new(SlidingWindow::new(config.window_size()));

        let recv_socket :T = socket.try_clone()?;
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

        return Ok(Sender { socket, remote_addr, seq_num: 0, window });
    }
}

impl <T: 'static> Receiver<T> where T: Socket + Send + Sync {
    /// Listens for an incoming connection
    pub fn listen(socket: T, config: &Configuration) -> Result<impl Transport, IOError> {
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

        let window = Arc::new(SlidingWindow::new(config.window_size()));

        let socket_clone :T = socket.try_clone()?;
        let recv_window = window.clone();

        thread::spawn(move || {
            socket_clone.set_read_timeout(None).expect("Could not set read timeout");

            let mut buf = vec![0; MAX_PACKET_SIZE];

            loop {
                // read a message
                let res = socket_clone.recv_from(&mut buf);

                if let Err(e) = res {
                    panic!("Error reading message: {:?}", e);
                }

                let (amt, _) = res.expect("Error unwrapping OK");
                let message = get_root_as_message(&buf[0..amt]);

                if message.msg_type() != Type::Message {
                    panic!("Unexpected message type: {:?}", message.msg_type());
                }

                let seq_num = message.seq_num();

                let (start, end) = recv_window.window();

                // check to see if the message is old
                // messages that are >= end, we'll simply block on insert waiting for the
                // reader to pick-up everything else
                if seq_num < start {
                    continue;
                }

                let payload = message.payload().expect("No payload for message");

                debug!("RECV PACKET: {} at {}", payload.len(), seq_num);

                // insert the packet into the window
                recv_window.insert(seq_num, payload.to_vec());

                let mut fbb = FlatBufferBuilder::new_with_capacity(MAX_PACKET_SIZE);
                let ack = Message::create(&mut fbb, &MessageArgs { msg_type: Type::Acknowledge, seq_num, payload: None });

                fbb.finish(ack, None);

                let ack_buf = fbb.finished_data().to_vec();

                if ack_buf.len() > MAX_PACKET_SIZE {
                    panic!("About to send ACK packet larger than max packet: {} > {}", ack_buf.len(), MAX_PACKET_SIZE);
                }

                socket_clone.send_to(&ack_buf, remote_addr);
            }
        });

        return Ok(Receiver { socket, remote_addr, window });
    }
}

impl <T> Transport for Sender<T> where T: Socket {
    fn read(&mut self, buf: &mut[u8]) -> Result<usize, IOError> {
        panic!("Not implemented");
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), IOError> {
        let chunk_it = buf.chunks(MAX_PAYLOAD_SIZE);

        for chunk in chunk_it {
            debug!("CHUNK LEN: {}", chunk.len());

            // construct the message w/the payload
            let mut fbb = FlatBufferBuilder::new_with_capacity(MAX_PACKET_SIZE);
            let payload = Some(fbb.create_vector(chunk));
            let msg = Message::create(&mut fbb, &MessageArgs { msg_type: Type::Message, seq_num: self.seq_num, payload });

            fbb.finish(msg, None);
            let msg_buf = fbb.finished_data().to_vec();

            if msg_buf.len() > MAX_PACKET_SIZE {
                panic!("About to send a packet larger than max packet: {} > {}", msg_buf.len(), MAX_PACKET_SIZE);
            }

            debug!("SENDING SEQ: {} LEN: {}", self.seq_num, msg_buf.len());
            trace!("PACKET: {}", buf2string(msg_buf.as_slice()));

            let mut end = { self.window.window().1 };

//            // wait for a slot in the window
//            while end <= self.seq_num {
//                let window = self.window.window();
//                panic!("Yielding on write_all: {} -> {}; {}", window.0, window.1, self.seq_num);
//                thread::yield_now();
//
//                end = { self.window.window().1 };
//            }

//            {
                self.socket.send_to(&msg_buf, self.remote_addr); // send the packet
                self.window.insert(self.seq_num, (Instant::now(), msg_buf)); // insert into the window
                self.seq_num += 1; // bump our sequence number
//            }

        }

        return Ok( () );
    }
}

impl <T> Transport for Receiver<T> where T: Socket {
    fn read(&mut self, buf: &mut[u8]) -> Result<usize, IOError> {
        let packet = self.window.pop();

        buf[..packet.len()].copy_from_slice(packet.as_slice());

        debug!("READ: {} length buf", packet.len());

        return Ok(packet.len());
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), IOError> {
        panic!("Not implemented");
    }
}


#[cfg(test)]
mod tests {
    use simplelog::{TermLogger, LevelFilter, Config};

    use bbr_transport::{Sender, Receiver, buf2string, MAX_PAYLOAD_SIZE, MAX_PACKET_SIZE};
    use config::Configuration;
    use socket::Socket;
    use transport::Transport;
    use std::net::{SocketAddr, UdpSocket};
    use std::thread;

    use flatbuffers::FlatBufferBuilder;
    use message_generated::bbr::{get_root_as_message, Message, MessageArgs, Type};

    use socket::mocks::PacketDroppingSocket;
    use rand::{thread_rng, Rng};

    #[test]
    fn udp_connect() {
        TermLogger::init(LevelFilter::Debug, Config::default()).unwrap();
        let local_addr = SocketAddr::new("0.0.0.0".parse().unwrap(), 1234);
        let socket = UdpSocket::bind(local_addr).expect("Couldn't bind socket");

        let t = Sender::<UdpSocket>::connect(socket, &Default::default());

        assert!(t.is_err());

        println!("{:?}", t.err());
    }

    #[test]
    fn udp_listen() {
        TermLogger::init(LevelFilter::Debug, Config::default()).unwrap();
        let local_addr = SocketAddr::new("0.0.0.0".parse().unwrap(), 1234);
        let socket = UdpSocket::bind(local_addr).expect("Couldno't bind socket");

        let t = Receiver::<UdpSocket>::listen(socket, &Default::default());
    }

    fn encode_decode(seq_num: u64) {
        let mut fbb = FlatBufferBuilder::new_with_capacity(MAX_PACKET_SIZE);
        let payload = thread_rng().gen_iter::<u8>().take(MAX_PAYLOAD_SIZE).collect::<Vec<u8>>();

        let buf = fbb.create_vector(&payload);

        let msg = Message::create(&mut fbb, &MessageArgs { msg_type: Type::Message, seq_num, payload: Some(buf) });

        fbb.finish(msg, None);

        let msg_buf = fbb.finished_data();

        println!("PACKET: {} {}", msg_buf.len(), buf2string(msg_buf));
        assert!(msg_buf.len() <= MAX_PACKET_SIZE);

        let msg = get_root_as_message(&msg_buf);
        let payload = msg.payload().expect("Error getting payload");

        println!("TYPE: {:?}", msg.msg_type());
        println!("SEQ NUM: {:x}", msg.seq_num());
        println!("PAYLOAD: {} {}", payload.len(), buf2string(payload));
        assert!(payload.len() <= MAX_PAYLOAD_SIZE);
    }

    #[test]
    fn multiple_encode_decode() {
        TermLogger::init(LevelFilter::Debug, Config::default()).unwrap();

        encode_decode(0); println!();
        encode_decode(0xAA); println!();
        encode_decode(0xAABB); println!();
        encode_decode(0xAABBCC); println!();
        encode_decode(0xAABBCCDD); println!();
        encode_decode(0xAABBCCDD_11); println!();
        encode_decode(0xAABBCCDD_1122); println!();
        encode_decode(0xAABBCCDD_112233); println!();
        encode_decode(0xAABBCCDD_11223344); println!();
    }

    #[test]
    fn packet_drops() {
        TermLogger::init(LevelFilter::Debug, Config::default()).unwrap();

        let mock_socket = PacketDroppingSocket::new();
        let duplex_socket = mock_socket.duplex();

        let send_handle = thread::Builder::new().name("send".into()).spawn(move || {
            let config = Configuration::default();
            let mut sender = Sender::<PacketDroppingSocket>::connect(mock_socket, &config).expect("Couldn't call connect");
            let mut buf = vec![0xAA; MAX_PAYLOAD_SIZE];

            for _ in 0..100 {
                sender.write_all(&buf).expect("Error calling write_all");
            }
        }).expect("Error spawning send thread");

        let recv_handle = thread::Builder::new().name("recv".into()).spawn(move || {
            let config = Configuration::default();
            let mut recver = Receiver::<PacketDroppingSocket>::listen(duplex_socket, &config).expect("Couldn't create receiver");
            let mut buf = vec![0xAA; MAX_PAYLOAD_SIZE];

            for _ in 0..100 {
                if recver.read(&mut buf).expect("Error calling read") != buf.len() {
                    panic!("Did not read everything");
                }
            }
        }).expect("Error spawning recv thread");

        send_handle.join();
        recv_handle.join();
    }
}

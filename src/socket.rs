use std::net::{UdpSocket, ToSocketAddrs, SocketAddr};
use std::io;
use std::time::Duration;
use std::fmt::Debug;
use std::marker::Sized;

pub trait Socket: Sized {
    fn send_to<A: ToSocketAddrs + Debug>(&self, buf: &[u8], addr: A) -> io::Result<usize>;

    fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)>;

    fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()>;

    fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()>;

    fn try_clone(&self) -> io::Result<Self>;
}

impl Socket for UdpSocket {
    fn send_to<A: ToSocketAddrs + Debug>(&self, buf: &[u8], addr: A) -> io::Result<usize> {
        return UdpSocket::send_to(self, buf, addr);
    }

    fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        return UdpSocket::recv_from(self, buf);
    }

    fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        return UdpSocket::set_read_timeout(self, dur);
    }

    fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        return UdpSocket::set_write_timeout(self, dur);
    }

    fn try_clone(&self) -> io::Result<Self> {
        return UdpSocket::try_clone(self);
    }
}

pub mod mocks {
    use std::net::{ToSocketAddrs, SocketAddr, IpAddr, Ipv4Addr};
    use std::io;
    use std::time::Duration;
    use socket::Socket;
    use std::fmt::Debug;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::cell::RefCell;
    use rand::{Rng, XorShiftRng, SeedableRng};

    struct PacketDroppingSocketInner {
        send_queue: Box<VecDeque<Vec<u8>>>,
        recv_queue: Box<VecDeque<Vec<u8>>>,
        read_timeout: Duration,
        rng: XorShiftRng
    }

    pub struct PacketDroppingSocket {
        inner: Arc<Mutex<PacketDroppingSocketInner>>
    }

    impl PacketDroppingSocket {
        pub fn new() -> Self {
            let inner = PacketDroppingSocketInner {
                send_queue: Box::new(VecDeque::<Vec<u8>>::new()),
                recv_queue: Box::new(VecDeque::<Vec<u8>>::new()),
                read_timeout: Duration::new(0, 0),
                rng: XorShiftRng::from_seed([0xAB; 16])
            };

            PacketDroppingSocket { inner: Arc::new(Mutex::new(inner)) }
        }

        pub fn duplex(&self) -> Self {
            let mut inner = self.inner.lock().unwrap();

            let new_inner = PacketDroppingSocketInner {
                send_queue: inner.recv_queue.clone(),
                recv_queue: inner.send_queue.clone(),
                read_timeout: inner.read_timeout.clone(),
                rng: inner.rng.clone()
            };

            PacketDroppingSocket { inner: Arc::new(Mutex::new(new_inner)) }
        }
    }

    impl Socket for PacketDroppingSocket {
        fn send_to<A: ToSocketAddrs + Debug>(&self, buf: &[u8], addr: A) -> io::Result<usize> {
            let mut inner = self.inner.lock().unwrap();

            // flip a coin to see if the packet makes it into the socket queue
            if inner.rng.gen_bool(1.0) {
                debug!("Called send_to; adding packet");

                inner.send_queue.push_back(buf.to_vec());
            } else {
                debug!("Called send_to; packet dropped");
            }

            return Ok(buf.len());
        }

        fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
            let mut inner = self.inner.lock().unwrap();

            let mut packet = inner.recv_queue.pop_front();

            if packet.is_none() {
                debug!("No packets, going to sleep for {:?}", inner.read_timeout);
                thread::sleep(inner.read_timeout);
                packet = inner.recv_queue.pop_front();
            }

            let mut packet_len = 0;

            if inner.rng.gen_bool(1.0) {
                if packet.is_some() {
                    debug!("Called recv_from; packet read");
                    let packet = packet.unwrap();
                    buf[..packet.len()].copy_from_slice(packet.as_slice());
                    packet_len = packet.len();
                } else {
                    debug!("Called recv_from; no packets");
                }
            } else {
                debug!("Called recv_from; packet dropped");
            }

            return Ok( (packet_len, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)) )
        }

        fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
            debug!("Called set_read_timeout: {:?}", dur);
            let mut inner = self.inner.lock().unwrap();

            inner.read_timeout = dur.unwrap();

            return Ok( () );
        }

        fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
            debug!("Called set_write_timeout: {:?}", dur);
            return Ok( () );
        }

        fn try_clone(&self) -> io::Result<Self> {
            debug!("Called try_clone");
            let inner = self.inner.lock().unwrap();

            let new_inner = PacketDroppingSocketInner {
                send_queue: inner.send_queue.clone(),
                recv_queue: inner.recv_queue.clone(),
                read_timeout: inner.read_timeout.clone(),
                rng: inner.rng.clone()
            };

            return Ok( PacketDroppingSocket { inner: Arc::new(Mutex::new(new_inner)) } );
        }
    }
}
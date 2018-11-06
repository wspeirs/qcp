use std::net::{UdpSocket, ToSocketAddrs, SocketAddr};
use std::io;
use std::time::Duration;
use std::fmt::Debug;
use std::marker::Sized;

pub trait Socket {
    fn bind<A: ToSocketAddrs + Debug, T: Socket + Send + Sync>(addr: A) -> io::Result<Self> where Self: Sized;

    fn send_to<A: ToSocketAddrs>(&self, buf: &[u8], addr: A) -> io::Result<usize>;

    fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)>;

    fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()>;

    fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()>;

    fn try_clone<T: Socket + Send + Sync>(&self) -> io::Result<T>;
}

impl Socket for UdpSocket {
    fn bind<A: ToSocketAddrs + Debug, T: Socket + Send + Sync>(addr: A) -> io::Result<Self> {
        return UdpSocket::bind(addr);
    }

    fn send_to<A: ToSocketAddrs>(&self, buf: &[u8], addr: A) -> io::Result<usize> {
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

    fn try_clone<T: Socket + Send + Sync>(&self) -> io::Result<T> {
        return UdpSocket::try_clone(self);
    }
}

mod test {
    use std::net::{ToSocketAddrs, SocketAddr, IpAddr, Ipv4Addr};
    use std::io;
    use std::time::Duration;
    use socket::Socket;
    use std::fmt::Debug;

    pub struct MockSocket { }

    impl Socket for MockSocket {
        fn bind<A: ToSocketAddrs + Debug, T: Socket>(addr: A) -> io::Result<Self> {
            debug!("Called bind: {:?}", addr);

            return Ok( MockSocket { } );
        }

        fn send_to<A: ToSocketAddrs>(&self, buf: &[u8], addr: A) -> io::Result<usize> {
            debug!("Called send_to: {:?}", addr);

            return Ok(5);
        }

        fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
            debug!("Called recv_from");

            return Ok( (5, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)) )
        }

        fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
            debug!("Called set_read_timeout: {:?}", dur);
            return Ok( () );
        }

        fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
            debug!("Called set_write_timeout: {:?}", dur);
            return Ok( () );
        }

        fn try_clone<T: Socket + Send + Sync>(&self) -> io::Result<T> {
            debug!("Called try clone");

            return Ok( MockSocket{} );
        }
    }
}
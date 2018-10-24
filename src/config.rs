use clap::{Arg, ArgGroup, App};

//use std::io::{Error as IOError, ErrorKind};
use std::fs::File;
use std::path::PathBuf;
use std::net::{SocketAddr};
use std::error::Error;
use std::default::Default;


pub struct Configuration {
    sender: bool,
    addr: SocketAddr,
    window_size: usize,
    file: Option<PathBuf>,
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            sender: false,
            addr: "127.0.0.1:1234".parse().unwrap(),
            window_size: 1024,
            file: Some(PathBuf::from("/tmp/test"))
        }
    }
}


impl Configuration {
    pub fn new() -> Result<Configuration, Box<Error>> {
        let matches = App::new("ets")
            .version("1.0")
            .author("William Speirs <bill.speirs@gmail.com>")
            .about("Quickly copy files from one machine to another")
            .group(ArgGroup::with_name("direction").args(&["send", "recv"]).required(true))
            .arg(Arg::with_name("send")
                .long("send")
                .help("Send files"))
            .arg(Arg::with_name("recv")
                .long("recv")
                .help("Receive files"))
            .arg(Arg::with_name("host")
                .long("host")
                .takes_value(true)
                .value_name("HOST")
                .default_value("0.0.0.0")
                .required_unless("recv")
                .help("Host to connect to when sending, or bind to when receiving"))
            .arg(Arg::with_name("port")
                .long("port")
                .takes_value(true)
                .value_name("PORT")
                .default_value("1234")
                .help("Port to connect to when sending, or listen on when receiving"))
            .arg(Arg::with_name("window-size")
                .short("w")
                .long("window-size")
                .takes_value(true)
                .default_value("1024")
                .help("The size of the sliding window"))
            .arg(Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"))
            .arg(Arg::with_name("FILE")
                .required(true)
                .help("The file to transfer")
                .index(1))
            .get_matches();

        // get the args
        let sender = matches.is_present("send");
        let file = matches.value_of("FILE");
        let host = matches.value_of("host").expect("Expected default host value");
        let port = matches.value_of("port").expect("Expected default port value");
        let addr = SocketAddr::new(host.parse()?, port.parse()?);
        let window_size = matches.value_of("window-size").expect("Expected default window-size").parse::<usize>()?;

        debug!("ADDR: {:?}", addr);

        if sender {
            info!("Sending file {} to {}", file.unwrap(), addr);
            return Ok(Configuration {
                sender,
                addr,
                window_size,
                file: Some(PathBuf::from(file.unwrap())),
            });
        } else {
            info!("Receiving file, listening on {}", addr);
            return Ok(Configuration {
                sender,
                addr,
                window_size,
                file: Some(PathBuf::from(file.unwrap()))
            });
        }

    }

    pub fn sender(&self) -> bool {
        self.sender
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn window_size(&self) -> usize {
        self.window_size
    }

    pub fn file(&self) -> &PathBuf {
        self.file.as_ref().unwrap()
    }

}

use clap::{Arg, App};

use std::io::{Error as IOError, ErrorKind};
use std::fs::File;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

pub struct Configuration {
    sender: bool,
    file: Option<PathBuf>,
}


impl Configuration {
    pub fn new() -> Result<Configuration, IOError> {
        let matches = App::new("ets")
            .version("1.0")
            .author("William Speirs <bill.speirs@gmail.com>")
            .about("Quickly copy files from one machine to another")
            .arg(Arg::with_name("send")
                .long("send")
                .help("Send files"))
            .arg(Arg::with_name("recv")
                .long("recv")
                .help("Receive files"))
            .arg(Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"))
            .arg(Arg::with_name("FILE")
                .help("The file to transfer")
                .index(1))
            .get_matches();

        let sender = matches.is_present("send");

        if sender && matches.is_present("recv") {
            return Err(IOError::new(ErrorKind::InvalidInput, "Specified both send and receive flags"));
        }

        let file = matches.value_of("FILE");

        if sender && file.is_none() {
            error!("Error attempting to send file, but did not specify one");
            return Err(IOError::new(ErrorKind::InvalidInput, "Attempting to send file, but no file specified"));
        }

        if sender {
            info!("Sending file: {}", file.unwrap());
            return Ok(Configuration { sender, file: Some(PathBuf::from(file.unwrap())) });
        } else {
            info!("Receiving file");
            return Ok(Configuration { sender, file: None });
        }

    }

    pub fn sender(&self) -> bool {
        self.sender
    }

    pub fn file(&self) -> &PathBuf {
        self.file.as_ref().unwrap()
    }

}

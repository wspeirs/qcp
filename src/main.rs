#[macro_use]
extern crate log;
extern crate simplelog;
extern crate clap;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate rmp_serde;

extern crate flatbuffers;

use std::io::Error as IOError;
use std::io::Write;
use std::process::exit;
use std::collections::HashMap;
use std::fs::File;

use simplelog::{TermLogger, LevelFilter, Config};

mod config;
mod transport;
mod tcp_transport;
mod sender;
mod receiver;
mod bbr_transport;
mod message_generated;
mod sliding_window;

use config::Configuration;
use sender::Sender;
use receiver::Receiver;
use bbr_transport::BBRTransport;


fn main() -> Result<(), IOError> {
    TermLogger::init(LevelFilter::Debug, Config::default()).unwrap();

    let config = Configuration::new()?;

    if config.sender() {
        Sender::new(config);
    } else {
        Receiver::new(config);
    }

    Ok( () )
}

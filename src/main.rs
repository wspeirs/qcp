#[macro_use] extern crate clap;
extern crate flatbuffers;
#[macro_use] extern crate log;
extern crate simplelog;

use std::io::Error as IOError;
use std::process::exit;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::error::Error;
use std::net::UdpSocket;

use simplelog::{TermLogger, LevelFilter, Config};

mod config;
mod transport;
mod tcp_transport;
mod bbr_transport;
mod message_generated;
mod sliding_window;
mod socket;

use config::Configuration;
use transport::Transport;

use bbr_transport::{Sender, Receiver, MAX_PAYLOAD_SIZE};

fn main() -> Result<(), Box<Error>> {
    TermLogger::init(LevelFilter::Debug, Config::default()).unwrap();

    let config = Configuration::new()?;

    if config.sender() {
        let mut sender = Sender::<UdpSocket>::connect(&config)?;
        let mut file = OpenOptions::new().read(true).create(false).open(config.file())?;

        let mut buf = vec![0; MAX_PAYLOAD_SIZE];

        loop {
            let amt = file.read(&mut buf)?;

            if amt == 0 {
                break;
            }

            sender.write_all(&buf[0..amt]);
        }
    } else {
        let mut recver = Receiver::<UdpSocket>::listen(&config)?;
        let mut file = OpenOptions::new().write(true).create(true).open(config.file())?;

        let mut buf = vec![0; MAX_PAYLOAD_SIZE];

        loop {
            let amt = recver.read(&mut buf)?;

            if amt == 0 {
                break;
            }

            file.write_all(&buf[0..amt]);
        }
    }

    Ok( () )
}

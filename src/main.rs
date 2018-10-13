#[macro_use]
extern crate log;
extern crate simplelog;
extern crate clap;
extern crate serde;
#[macro_use] extern crate serde_derive;

use std::io::Error as IOError;
use std::io::Write;
use std::process::exit;
use std::collections::HashMap;
use std::fs::File;

use simplelog::{TermLogger, LevelFilter, Config};

mod config;

use config::Configuration;


fn main() -> Result<(), IOError> {
    TermLogger::init(LevelFilter::Debug, Config::default()).unwrap();

    let config = Configuration::new();

    Ok( () )
}

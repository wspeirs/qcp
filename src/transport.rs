use std::io::{Error as IOError};

pub trait Transport {
    /// Read up to buf.len() bytes from the underlying transport
    fn read(&mut self, buf: &mut[u8]) -> Result<usize, IOError>;

    /// Write all buf.len() bytes to the underlying transport
    fn write_all(&mut self, buf: &[u8]) -> Result<(), IOError>;
}
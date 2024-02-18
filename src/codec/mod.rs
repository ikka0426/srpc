
use serde::{ Serialize, Deserialize, de::DeserializeOwned };

// use std::time::Duration;
use std::net::TcpStream;
use std::io::{ Read, Write };
use std::sync::Mutex;

use super::error::Error;

pub const MAGIC_NUMBER: i32 = 0x37373737;

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
  pub magic_number: i32,
}

impl Message {
  pub fn new(magic_number: i32) -> Self {
    Message { magic_number }
  }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
  pub service: String,
  pub method: String,
  pub seq: usize,
  // pub time: Duration,
  pub error: Option<Error>,
}

impl Header {
  pub fn new(
    service: String,
    method: String,
    seq: usize,
    // time: Duration,
    error: Option<Error>
  ) -> Self {
    Header { 
      service: service, 
      method: method, 
      seq, 
      // time,
      error 
    }
  }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Body<T: Serialize> {
  pub contents: T,
}

impl<T:Serialize> Body<T> {
  pub fn new(contents: T) -> Self {
    Body { contents }
  }
}

pub struct Codec {
  stream: Mutex<Option<TcpStream>>,
}

impl Codec {
  pub fn new() -> Self {
    Codec { stream: Mutex::new(None) }
  }

  pub fn bind(&self, stream: TcpStream) {
    let mut stream_locked = self.stream.lock().unwrap();
    stream.set_nonblocking(true).unwrap();
    *stream_locked = Some(stream);
  }

  pub fn encode<T: Serialize>(&self, value: &T) {
    let mut stream_locked = self.stream.lock().unwrap();
    let serialized = serde_json::to_string(value).unwrap();
    let bytes = serialized.as_bytes();
    let len: [u8; 8] = bytes.len().to_ne_bytes();
    
    stream_locked.as_mut().unwrap().write(&[&len, bytes].concat()).unwrap();
  }

  pub fn decode<T: DeserializeOwned>(&self) -> Result<T, Error> {
    let mut buf = [0; 8];
    loop {
      let mut stream_locked = self.stream.lock().unwrap();
      match stream_locked.as_mut().unwrap().read_exact(&mut buf) {
        Ok(_) => {
          let len: usize = usize::from_ne_bytes(buf);
          let mut message = vec![0; len];
          // std::thread::sleep(std::time::Duration::new(1, 0));
          match stream_locked.as_mut().unwrap().read_exact(&mut message) {
            Ok(_) => {
              let serialized = String::from_utf8_lossy(&message);
              // println!("---------\n{}\n---------\n", serialized);
              return Ok(serde_json::from_str::<T>(&serialized).unwrap())
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
              continue;
            }
            Err(e) => {
              return Err(Error::SystemIOError())
            }
          }
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
          continue;
        }
        Err(e) => {
          return Err(Error::SystemIOError())
        }
      }
    }
  }
}

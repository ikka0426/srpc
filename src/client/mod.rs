
use serde::de::DeserializeOwned;
use serde::{ Serialize, Deserialize };
use serde_json::Value;

use std::net::TcpStream;
use std::io::{ Read, Write };
use std::sync::{ Arc, Mutex };
use std::sync::mpsc::{ Sender, Receiver, channel };
use std::collections::HashMap;
use std::thread;

use super::codec::*;
use super::server::remote_call::Error;

struct Call {
  seq: usize,
  service: String,
  method: String,
  body: Value,
  error: Option<Error>,
  sender: Option<Sender<Value>>,
}

impl Call {
  fn new(service: String, method: String, body: Value) -> Self {
    Call { seq: 0, service, method, body, error: None, sender: None }
  }
}

pub struct Client {
  seq: Mutex<usize>,
  codec: Codec,
  pending: Mutex<HashMap<usize, Call>>,
  closing: Mutex<bool>,
  shutdown: Mutex<bool>,
}

impl Client {
  pub fn new() -> Self {
    Client { 
      seq: 1.into(), 
      codec: Codec::new(),
      pending: Mutex::new(HashMap::new()),
      closing: false.into(),
      shutdown: false.into(),
    }
  }

  pub fn dial(&self, addr: &str) {
    let stream = TcpStream::connect(addr).unwrap();
    self.codec.bind(stream);

    let message = Message::new(MAGIC_NUMBER);
    self.codec.encode(&message);
  }

  fn register_call(&self, mut call: Call) -> Result<usize, Error> {
    if self.is_available() {
      let mut seq_locked = self.seq.lock().unwrap();
      let mut pending_locked = self.pending.lock().unwrap();
      call.seq = *seq_locked;
      pending_locked.insert(call.seq, call);
      *seq_locked += 1;
      Ok(*seq_locked - 1)
    } else {
      Err(Error::ClientNotAvailableError)
    }
  }

  fn remove_call(&self, seq: usize) -> Option<Call> {
    let mut pending_locked = self.pending.lock().unwrap();
    pending_locked.remove(&seq)
  }

  fn terminate_calls(&self, e: Error) {
    let mut shutdown_locked = self.shutdown.lock().unwrap();
    let mut pending_locked = self.pending.lock().unwrap();
    *shutdown_locked = true;
    for call in &mut *pending_locked {
      call.1.error = Some(e.clone());
    }
  }

  pub fn call<T, U>(&self, service: &str, method: &str, args: U) -> Result<T, Error>
  where 
    T: DeserializeOwned,
    U: Serialize {
    let value = self.call_async(service, method, args).recv().unwrap();
    Ok(serde_json::from_value::<T>(value).unwrap())
  }
    
  fn send(&self, call: Call) {
    
    let body = call.body.clone();
    let header = Header::new(
      call.service.clone(), 
      call.method.clone(), 
      self.register_call(call).unwrap(), 
      None
    );
    
    self.codec.encode(&(header, body));
  }
  
  fn call_async<T: Serialize>(&self, service: &str, method: &str, args: T) -> Receiver<Value> {
    let body = serde_json::to_value(Body { contents: args }).unwrap();
    let mut call = Call::new(service.to_string(), method.to_string(), body);
    let (tx, rx) = channel::<Value>();
    call.sender = Some(tx);
    self.send(call);
    rx
  }

  pub fn recv(&self) {
    let error;
    loop {
      let (header, body): (Header, Body<Value>) = self.codec.decode().unwrap();
      // println!("client got new message from server: \n{:#?}", header);
      match self.remove_call(header.seq) {
        Some(call) => {
          if let Some(e) = header.error {
            error = e;
            println!("{}", error);
            break;
          } else {
            call.sender.unwrap().send(body.contents).unwrap();
          }
        }
        None => {
          error = Error::OtherError;
          println!("{}", error);
          break;
        }
      }
    }
    self.terminate_calls(error);
  } 

  fn is_available(&self) -> bool {
    let closing_locked = self.closing.lock().unwrap();
    let shutdown_locked = self.shutdown.lock().unwrap();
    !*closing_locked && !*shutdown_locked
  }

}

pub mod remote_call;
pub mod thread_pool;

use serde_json::Value;

use std::net::{ TcpListener, TcpStream };
use std::collections::HashMap;
use std::sync::{ Arc, Mutex };

use thread_pool::ThreadPool;
use remote_call::RemoteCall;
use super::codec::*;

type Service = Box<dyn RemoteCall + Send + 'static>;

pub struct Server {
  thread_pool: ThreadPool,
  services: Arc<Mutex<HashMap<String, Service>>>,
}

impl Server {
  pub fn new() -> Self {
    let thread_pool = ThreadPool::new(10);
    return Server { 
      thread_pool, 
      services: Arc::new(Mutex::new(HashMap::new()))
    }
  }

  pub fn register(&mut self, name: String, service: Service) {
    self.services.lock().unwrap().insert(name, service);
  }

  pub fn run(&self, addr: &str) {
    let listener = TcpListener::bind(addr).unwrap();

    for stream in listener.incoming() {
      match stream {
        Ok(stream) => {
          let services = Arc::clone(&self.services);
          self.thread_pool.execute(|| {
            Self::connect(stream, services);
          });
        }
        Err(e) => {
          println!("error at file {} line: {}, {}", file!(), line!(), e);
        }
      }
    }
  }

  fn connect(stream: TcpStream, services: Arc<Mutex<HashMap<String, Service>>>) {
    let codec = Codec::new();
    codec.bind(stream);
    let message: Message = codec.decode().unwrap();
    if message.magic_number != MAGIC_NUMBER {
      eprintln!("** Server Error: magic number mismatched.");
      return;
    }
    Self::serve(codec, services);
  }
  
  fn serve(codec: Codec, services: Arc<Mutex<HashMap<String, Service>>>) {
    loop {
      let (header, body): (Header, Body<Value>) = codec.decode().unwrap();
      // println!("server got new call from client \n{:#?}", header);
      let locked = services.lock().unwrap();
      let service = locked.get(&header.service).unwrap();
      let value = service.call(&header.method, body.contents).unwrap();
      let body = Body { contents: value };
      codec.encode(&(header, body));
    }
  }

}
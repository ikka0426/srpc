
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
    let addr: Vec<&str> = addr.split("@").collect();
    let (protocol, addr) = (addr[0], addr[1]);
    let listener = TcpListener::bind(addr).unwrap();
    
    for stream in listener.incoming() {
      match stream {
        Ok(stream) => {
          let protocol_copy = protocol.to_string();
          let services = Arc::clone(&self.services);
          self.thread_pool.execute(move || {
            Self::connect(stream, services, protocol_copy);
          });
        }
        Err(e) => {
          println!("error at file {} line: {}, {}", file!(), line!(), e);
        }
      }
    }
  }

  fn connect(stream: TcpStream, services: Arc<Mutex<HashMap<String, Service>>>, protocol: String) {
    let codec = Codec::new();
    codec.bind(stream);
    // let message: Message = codec.decode().unwrap();
    // if message.magic_number != MAGIC_NUMBER {
    //   eprintln!("** Server Error: magic number mismatched.");
    //   return;
    // }
    match &protocol[..] {
      "http" => Self::serve_http(codec, services),
      _ => Self::serve_tcp(codec, services)
    }
  }
  
  fn serve_http(codec: Codec, services: Arc<Mutex<HashMap<String, Service>>>) {
    loop {
      let x: String = codec.read_plain().unwrap();
      let request = x.split(" ").collect::<Vec<&str>>()[0];
      match request {
        "GET" => {
          let status_line = "HTTP/1.1 200 OK";
          let debug_html = "
            <html>
              <head>
                <title>SRPC Debug Page</title>
              </head>
              <body>
                <h2>SRPC Services</h2>
              </body>
            </html>
          ";
          let len = debug_html.len();
          let response = format!("{status_line}\r\nContent-Length: {len}\r\n\r\n{debug_html}");
          codec.write_plain(response).unwrap();
        }
        "CONNECT" => {

        }
        _ => {

        }
      }
    }
  }

  fn serve_tcp(codec: Codec, services: Arc<Mutex<HashMap<String, Service>>>) {
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
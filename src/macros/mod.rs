
#[macro_export]
macro_rules! register {
  ($server: ident, $service: ident) => {
    $server.register(stringify!($service).to_string(), Box::new($service {  }));
  };
}
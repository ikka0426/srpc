
use serde_json::Value;
use serde::{ Serialize, Deserialize };

use std::fmt::Display;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Error {
  NoSuchMethodError,
  ClientNotAvailableError,
  ArgumentsNotMatchError(String),
  SystemIOError(),
  OtherError
}

impl Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "SRPC inner error occurred: {:?}", self)
  }
}

pub trait RemoteCall: Send {
  fn call(&self, method: &str, args: Value) -> Result<Value, Error>;
}
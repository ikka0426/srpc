
use serde_json::Value;

use crate::error::Error;

pub trait RemoteCall: Send {
  fn call(&self, method: &str, args: Value) -> Result<Value, Error>;
}
use std::sync::{
    Arc,
    Mutex,
};

use blockchain::Configuration;
use jsonrpc_http_server::jsonrpc_core::*;

pub fn get_chain_length(_state: Arc<Mutex<Configuration>>) -> impl RpcMethodSimple {
    // WIP
    async move |params: Params| -> Result<Value> {
        match params {
            Params::Map(_) => Ok(Value::String("test".to_string())),
            _ => Err(Error::invalid_request()),
        }
    }
}

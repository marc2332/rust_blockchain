use blockchain::Configuration;
use std::sync::{
    Arc,
    Mutex,
};

fn main() {
    let mut node = node::Node::new();

    let config = Arc::new(Mutex::new(Configuration::new()));

    node.run(config);
}

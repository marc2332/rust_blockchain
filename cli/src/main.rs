use blockchain::Configuration;
use futures::lock::Mutex;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let mut node = node::Node::new();

    let config = Arc::new(Mutex::new(Configuration::new()));

    node.run(config).await;
}

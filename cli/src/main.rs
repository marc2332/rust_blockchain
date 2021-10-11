use blockchain::Configuration;

#[tokio::main]
async fn main() {
    let config = Configuration::new();

    let mut node = node::Node::new(config).await;

    node.run().await;
}

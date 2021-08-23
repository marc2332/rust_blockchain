use blockchain::Configuration;

#[tokio::main]
async fn main() {
    let mut node = node::Node::new();

    let config = Configuration::new();

    node.run(config).await;
}

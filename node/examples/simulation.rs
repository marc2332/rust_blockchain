use std::sync::{Arc};

use blockchain::{Configuration, Wallet};
use futures::{future::join_all, lock::Mutex};
use node::Node;

fn create_nodes() -> Vec<(Node, Configuration)> {

    (0..15).map(|i | {
        let config = Configuration::from_params(
            &format!("db_{}", i),
            3030+i,
            "0.0.0.0",
            Wallet::default()
        );

        let node = Node::new();

        (node, config)
    }).collect()
}


/*
 * The idea of this simulation is to run a few nodes that share the same blockchain and verify transactions
 *
 * To-Do:
 * - Create the blockchain 1 time and make all nodes sync it.
 */
#[tokio::main]
async fn main(){
    let nodes = create_nodes();

    let mut nodes_runtimes = Vec::new();

    for (node, config) in nodes {
       
        nodes_runtimes.push(tokio::spawn(async move {
            let mut node = node.clone();
            node.run(Arc::new(Mutex::new(config.clone()))).await;
        }));
    }

    join_all(nodes_runtimes).await;
}
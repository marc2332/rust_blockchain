#![feature(async_closure)]
use blockchain::Block;
use futures_util::{
    SinkExt,
    StreamExt,
};
use jsonrpc_core::{
    serde_json::json,
    IoHandler,
};
use jsonrpc_derive::rpc;
use std::sync::{
    mpsc::{
        channel,
        Sender,
    },
    Arc,
    Mutex,
};
use warp::{
    ws::Message,
    Filter,
};

enum SubscriberMessage {
    NewBlock { block: Block },
}

struct MetricsState {
    /// Subscribed clients
    pub senders: Vec<Sender<SubscriberMessage>>,
}

impl MetricsState {
    pub fn new() -> Self {
        Self {
            senders: Vec::new(),
        }
    }
}

type RPCResult<T> = jsonrpc_core::Result<T>;

#[rpc]
pub trait RpcMethods {
    #[rpc(name = "new_block")]
    fn new_block(&self, block: Block) -> RPCResult<()>;
}

struct RpcManager {
    state: Arc<Mutex<MetricsState>>,
}

impl RpcMethods for RpcManager {
    /// Announce that a new block has been added to all subscribed clients
    fn new_block(&self, block: Block) -> RPCResult<()> {
        for sender in &self.state.lock().unwrap().senders {
            sender
                .send(SubscriberMessage::NewBlock {
                    block: block.clone(),
                })
                .unwrap();
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    println!("Running Metrics Server");

    let metrics_state = Arc::new(Mutex::new(MetricsState::new()));

    let rpc_state = metrics_state.clone();

    tokio::task::spawn_blocking(move || {
        let mut ws_io = IoHandler::default();
        let ws_manager = RpcManager { state: rpc_state };
        ws_io.extend_with(ws_manager.to_delegate());

        let server = jsonrpc_ws_server::ServerBuilder::new(ws_io)
            .start(&format!("{}:{}", "127.0.0.1", 1234).parse().unwrap())
            .expect("Unable to start RPC server");

        server.wait().unwrap();
    });

    tokio::task::spawn_blocking(async move || {
        let metrics_state = metrics_state.clone();

        let routes = warp::path("echo")
            .and(warp::ws())
            .map(move |ws: warp::ws::Ws| {
                let metrics_state = metrics_state.clone();
                ws.on_upgrade(async move |websocket| {
                    let (sender, _recv) = websocket.split();
                    let (tx, rx) = channel::<SubscriberMessage>();

                    metrics_state.lock().unwrap().senders.push(tx);

                    use tokio::sync::Mutex;

                    let sender = Arc::new(Mutex::new(sender));
                    let rx = Arc::new(Mutex::new(rx));

                    std::thread::spawn(move || {
                        let runtime = tokio::runtime::Runtime::new().unwrap();
                        runtime.block_on(async {
                            loop {
                                let rx = rx.lock().await;
                                match rx.recv().unwrap() {
                                    SubscriberMessage::NewBlock { block } => {
                                        let msg = json!({
                                            "hash": block.hash.unite()
                                        });

                                        let mut sender = sender.lock().await;
                                        sender.send(Message::text(msg.to_string())).await.unwrap();
                                    }
                                }
                            }
                        });
                    });
                })
            });

        warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
    })
    .await
    .unwrap()
    .await;
}

use std::{
    sync::{
        mpsc::{
            channel,
            Sender,
        },
        Arc,
    },
    thread,
};

use futures_util::Future;
use jsonrpc_client_transports::{
    transports::ws,
    RpcChannel,
    RpcError,
    RpcResult,
    TypedClient,
};
use tokio::sync::Mutex;

use crate::Block;

/*
 * List of metrics messages
 */
#[derive(Clone)]
pub enum MetricMessage {
    NewBlock { block: Block },
    MempoolSize(u32),
}

#[derive(Clone)]
pub struct Metrics {
    pub connections: Vec<Sender<MetricMessage>>,
}

impl Metrics {
    pub fn new(sending_endpoints: Vec<String>) -> Self {
        Self {
            connections: sending_endpoints
                .iter()
                .map(|endpoint| Self::create_endpoint_handler(endpoint))
                .collect::<Vec<Sender<MetricMessage>>>(),
        }
    }

    /*
     * Send a MetricMesage to all active connection handlers
     */
    pub fn send_message(&self, message: MetricMessage) {
        for conn in &self.connections {
            conn.send(message.clone()).unwrap();
        }
    }

    /*
     * Announce a new block to all active connection handlers
     */
    pub fn new_block(&self, block: Block) {
        self.send_message(MetricMessage::NewBlock { block });
    }

    /*
     * Create a WebSocket connection handler for an specific endpoint
     */
    fn create_endpoint_handler(endpoint: &str) -> Sender<MetricMessage> {
        let endpoint = endpoint.to_string();
        let (tx, rx) = channel();
        let rx = Arc::new(Mutex::new(rx));
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let client = MetricsClient::new(&endpoint).await.unwrap();
                let client = Arc::new(Mutex::new(client));
                loop {
                    let rx = rx.lock().await;
                    let client = client.clone();
                    if let MetricMessage::NewBlock { block } = rx.recv().unwrap() {
                        tokio::spawn(async move {
                            client.lock().await.new_block(block.clone()).await.unwrap();
                        });
                    }
                }
            })
        });

        tx
    }
}

/*
 * Metrics JSON RPC Server client
 * Only used internally
 */
#[derive(Clone)]
pub struct MetricsClient(TypedClient);

impl From<RpcChannel> for MetricsClient {
    fn from(channel: RpcChannel) -> Self {
        MetricsClient(channel.into())
    }
}

impl MetricsClient {
    pub async fn new(uri: &str) -> Result<Self, RpcError> {
        ws::try_connect(uri).unwrap().await
    }
}

impl MetricsClient {
    /*
     * Communicate a new block was added
     */
    pub fn new_block(&self, block: Block) -> impl Future<Output = RpcResult<()>> {
        self.0.call_method("new_block", "()", (block,))
    }
}

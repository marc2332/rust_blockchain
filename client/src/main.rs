#[jsonrpc_client::api]
trait Math {
    async fn get_chain_length(&self) -> String;
}

#[jsonrpc_client::implement(Math)]
struct Client {
    inner: reqwest::Client,
    base_url: reqwest::Url,
}

impl Client {
    fn new(base_url: String) -> Result<Self, ()> {
        Ok(Self {
            inner: reqwest::Client::new(),
            base_url: base_url.parse().unwrap(),
        })
    }
}

#[tokio::main]
async fn main() {
    let client = Client::new("http://localhost:3030".to_owned()).unwrap();
    println!("{}", client.get_chain_length().await.unwrap());
}

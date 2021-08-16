use consensus::{GoalBuilder};

#[tokio::main]
async fn main(){

    let mut goal = GoalBuilder::new()
        .zeros(0)
        .data("_".to_string())
        .build();

    let result = goal.start().await;

    println!("nonce is {:?}", result);

}
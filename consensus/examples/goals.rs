use consensus::{GoalBuilder, Player};

#[tokio::main]
async fn main(){
    let player_a = Player::new(0);

    let mut goal = GoalBuilder::new()
        .zeros(5)
        .player(player_a)
        .data("_".to_string())
        .build();

    let result = goal.start().await;

    println!("winner is {:?}", result);

}
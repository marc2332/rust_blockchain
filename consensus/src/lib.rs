#![feature(async_closure)]
use std::{sync::{Arc, Mutex}};
use crypto::{
    digest::Digest,
    sha3::{
        Sha3,
        Sha3Mode,
    },
};

#[derive(Clone, Debug)]
pub struct Player {
    pub id: usize
}

impl Player {
    pub fn new(id: usize) -> Self {
        Self {
            id
        }
    }
}

pub struct GoalBuilder {
    zeros: Option<usize>,
    player: Option<Player>,
    data: Option<String>
}

impl GoalBuilder {
    pub fn new() -> Self {
        Self {
            zeros: None,
            player: None,
            data: None
        }
    }

    pub fn zeros(&mut self, n: usize) -> &mut Self {
        self.zeros = Some(n);
        self
    }

    pub fn player(&mut self, player: Player) -> &mut Self {
        self.player = Some(player);
        self
    }

    pub fn data(&mut self, data: String) -> &mut Self {
        self.data = Some(data);
        self
    }

    pub fn build(&self) -> Goal {
        Goal {
            zeros: self.zeros.unwrap(),
            player: self.player.as_ref().unwrap().clone(),
            data: self.data.as_ref().unwrap().clone()
        }
    }
}

pub struct Goal {
    zeros: usize,
    player: Player,
    data: String
}


async fn play(player: Player, winner: Arc<Mutex<GoalResult>>, data: String, zeros: usize){

    let mut data = data.clone();
    let mut times = 0;

    let goal = "0".repeat(zeros);
        
    loop {
        let mut hasher = Sha3::new(Sha3Mode::Keccak256);

        hasher.input_str(&data);

        let res = hasher.result_str();

        if res.starts_with(&goal) {
            winner.lock().unwrap().player = Some(player.clone());
            winner.lock().unwrap().times = times;
            break;
        } else {
            times += 1;
            data = res;
        }
    }

}

impl Goal {
    
    pub async fn start(&mut self) -> Player {

        let winner = Arc::new(Mutex::new(GoalResult::new()));

        play(self.player.clone(), winner.clone(), self.data.clone(),  self.zeros).await;

        winner.clone().lock().unwrap().player.as_ref().unwrap().clone()
    }
}


pub struct GoalResult {
    pub player: Option<Player>,
    pub times: usize
}

impl GoalResult {
    pub fn new() -> Self {
        Self {
            player: None,
            times: 0
        }
    }
}
use crypto::{
    digest::Digest,
    sha3::{
        Sha3,
        Sha3Mode,
    },
};

pub struct GoalBuilder {
    zeros: Option<usize>,
    data: Option<String>
}

impl GoalBuilder {
    pub fn new() -> Self {
        Self {
            zeros: None,
            data: None
        }
    }

    pub fn zeros(&mut self, n: usize) -> &mut Self {
        self.zeros = Some(n);
        self
    }

    pub fn data(&mut self, data: String) -> &mut Self {
        self.data = Some(data);
        self
    }

    pub fn build(&self) -> Goal {
        Goal {
            zeros: self.zeros.unwrap(),
            data: self.data.as_ref().unwrap().clone()
        }
    }
}

pub struct Goal {
    zeros: usize,
    data: String
}

impl Goal {
    
    pub async fn start(&mut self) -> u64 {

        let mut nonce = 0;

        let goal = "0".repeat(self.zeros);
            
        loop {
            let mut hasher = Sha3::new(Sha3Mode::Keccak256);

            hasher.input_str(&format!("{}{}",self.data,nonce));

            let res = hasher.result_str();
            
            if res.starts_with(&goal) {
                break;
            } else {
                nonce += 1;
            }
        }

        nonce
    }
}

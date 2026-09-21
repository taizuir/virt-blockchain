pub mod sha256;
pub mod wallet;
use sha256::sha256_hex;
use wallet::Transaction;

pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub data: String,
    pub prev_hash: String,
    pub hash: String,
    pub nonce: u64,
}

impl Block {
    pub fn new(index: u64, data: String, prev_hash: String, difficulty: usize) -> Block {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut block = Block {
            index,
            timestamp,
            data,
            prev_hash,
            hash: String::new(),
            nonce: 0,
        };

        block.mine(difficulty);
        block
    }

    pub fn header_string(&self) -> String {
        format!(
            "{}{}{}{}{}",
            self.index, self.timestamp, self.data, self.prev_hash, self.nonce
        )
    }

    pub fn calculate_hash(&self) -> String {
        sha256_hex(self.header_string().as_bytes())
    }

    pub fn mine(&mut self, difficulty: usize) {
        let target = "0".repeat(difficulty);
        loop {
            self.hash = self.calculate_hash();
            if self.hash.starts_with(&target) {
                break;
            }
            self.nonce += 1;
        }
    }

    pub fn genesis(difficulty: usize) -> Block {
        Block::new(
            0,
            String::from("genesis block"),
            String::from("0"),
            difficulty,
        )
    }
}

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: usize,
    pub mempool: Vec<Transaction>,
}

impl Blockchain {
    pub fn new() -> Blockchain {
        Blockchain {
            chain: vec![Block::genesis(1)],
            difficulty: 1,
            mempool: vec![],
        }
    }

    pub fn with_difficulty(difficulty: usize) -> Blockchain {
        Blockchain {
            chain: vec![Block::genesis(difficulty)],
            difficulty,
            mempool: vec![],
        }
    }

    pub fn access_last(&self) -> &Block {
        self.chain.last().unwrap()
    }

    pub fn add_block(&mut self, data: String) {
        let prev_hash = self.access_last().hash.clone();
        let new_index = self.access_last().index + 1;
        let new_block = Block::new(new_index, data, prev_hash, self.difficulty);
        self.chain.push(new_block);
    }

    pub fn is_valid(&self) -> bool {
        for i in 1..self.chain.len() {
            let current = &self.chain[i];
            let previous = &self.chain[i - 1];

            if current.hash != current.calculate_hash() {
                return false; // intégrité du bloc cassée
            }
            if current.prev_hash != previous.hash {
                return false; // lien avec le bloc précédent cassé
            }
        }
        true
    }
    pub fn asked_transaction(&mut self, T: Transaction) -> bool {
        T.verify()
    }
}

pub mod sha256;
pub mod wallet;
use sha256::sha256_hex;
use std::mem::take;
use wallet::Transaction;
//un simple bloc sur le blockcahin il contient son numero sa date de création  une transacation le hash de la clé en cour le hashde la clé du block precedent
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub data: Vec<Transaction>,
    pub prev_hash: String,
    pub hash: String,
    pub nonce: u64,
}

impl Block {
    pub fn new(index: u64, data: Vec<Transaction>, prev_hash: String, difficulty: usize) -> Block {
        //recupère al date
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        //creéee un block
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
    // prend les info di bock pour le convertir en bytes et ensuite paermmetre de crypter
    pub fn header_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = vec![];
        bytes.extend_from_slice(&self.index.to_be_bytes());
        bytes.extend_from_slice(&self.timestamp.to_be_bytes());
        for fields in &self.data {
            bytes.extend_from_slice(&fields.payload());
        }
        bytes.extend_from_slice(&self.prev_hash.as_bytes());
        bytes.extend_from_slice(&self.hash.as_bytes());
        bytes.extend_from_slice(&self.nonce.to_be_bytes());
        bytes
    }
    // crypte la clé
    pub fn calculate_hash(&self) -> String {
        sha256_hex(&self.header_bytes())
    }
    // hash tant que on a pas un diifficulty dnomcre de zero a debut sert à
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
        Block::new(0, vec![], String::from("0"), difficulty)
    }
}
// la bockcjain contient les blocs une difculté choisi et une liste de transacton a ajouter
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
    //new rendu plus difficulté
    pub fn with_difficulty(difficulty: usize) -> Blockchain {
        Blockchain {
            chain: vec![Block::genesis(difficulty)],
            difficulty,
            mempool: vec![],
        }
    }
    // donne accès au dernier elemetn
    pub fn access_last(&self) -> &Block {
        self.chain.last().unwrap()
    }

    pub fn add_block(&mut self) {
        let clean_mempool = take(&mut self.mempool);
        let prev_hash = self.access_last().hash.clone();
        let new_index = self.access_last().index + 1;
        let new_block = Block::new(new_index, clean_mempool, prev_hash, self.difficulty);
        self.chain.push(new_block);
    }

    pub fn is_valid(&self) -> bool {
        for i in 1..self.chain.len() {
            let current = &self.chain[i];
            let previous = &self.chain[i - 1];
            for i in &current.data {
                if !i.verify() {
                    return false;
                }
            }
            if current.hash != current.calculate_hash() {
                return false; // intégrité du bloc cassée
            }
            if current.prev_hash != previous.hash {
                return false; // lien avec le bloc précédent cassé
            }
        }
        true
    }
    pub fn asked_transaction(&mut self, t: Transaction) -> () {
        if t.verify() {
            self.mempool.push(t)
        }
    }
}

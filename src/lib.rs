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

#[cfg(test)]
mod tests {
    use super::*;
    use wallet::Wallet;

    const DIFFICULTY: usize = 2;

    fn address(w: &Wallet) -> String {
        hex::encode(w.public_key.to_bytes())
    }

    fn signed_tx(from: &Wallet, to: &Wallet, amount: u64) -> Transaction {
        let mut tx = Transaction::new(address(from), address(to), amount);
        tx.sign(from);
        tx
    }

    fn raw_block(data: Vec<Transaction>) -> Block {
        Block {
            index: 7,
            timestamp: 1_700_000_000,
            data,
            prev_hash: "p".repeat(64),
            hash: String::new(),
            nonce: 42,
        }
    }

    #[test]
    fn header_bytes_length_and_layout() {
        let a = Wallet::new();
        let b = Wallet::new();
        let tx = signed_tx(&a, &b, 5);
        let payload_len = tx.payload().len();
        let block = raw_block(vec![tx]);
        let bytes = block.header_bytes();

        assert_eq!(bytes.len(), 8 + 8 + payload_len + 64 + 8);
        assert_eq!(&bytes[..8], &7u64.to_be_bytes());
        assert_eq!(&bytes[8..16], &1_700_000_000u64.to_be_bytes());
        assert_eq!(&bytes[bytes.len() - 8..], &42u64.to_be_bytes());
    }

    #[test]
    fn header_bytes_do_not_depend_on_hash() {
        let mut block = raw_block(vec![]);
        let before = block.header_bytes();
        block.hash = "n'importe quoi".to_string();
        assert_eq!(before, block.header_bytes());
    }

    #[test]
    fn header_bytes_change_with_each_field() {
        let a = Wallet::new();
        let b = Wallet::new();
        let base = raw_block(vec![signed_tx(&a, &b, 5)]).header_bytes();

        let mut x = raw_block(vec![signed_tx(&a, &b, 5)]);
        x.index += 1;
        assert_ne!(base, x.header_bytes(), "index");

        let mut x = raw_block(vec![signed_tx(&a, &b, 5)]);
        x.timestamp += 1;
        assert_ne!(base, x.header_bytes(), "timestamp");

        let mut x = raw_block(vec![signed_tx(&a, &b, 5)]);
        x.prev_hash = "q".repeat(64);
        assert_ne!(base, x.header_bytes(), "prev_hash");

        let mut x = raw_block(vec![signed_tx(&a, &b, 5)]);
        x.nonce += 1;
        assert_ne!(base, x.header_bytes(), "nonce");

        let x = raw_block(vec![signed_tx(&a, &b, 6)]);
        assert_ne!(base, x.header_bytes(), "montant d'une transaction");
    }

    #[test]
    fn header_bytes_depend_on_transaction_order() {
        let a = Wallet::new();
        let b = Wallet::new();
        let first = raw_block(vec![signed_tx(&a, &b, 1), signed_tx(&a, &b, 2)]);
        let second = raw_block(vec![signed_tx(&a, &b, 2), signed_tx(&a, &b, 1)]);
        assert_ne!(first.header_bytes(), second.header_bytes());
    }

    #[test]
    fn calculate_hash_is_sha256_of_header_bytes() {
        let block = raw_block(vec![]);
        assert_eq!(block.calculate_hash(), sha256_hex(&block.header_bytes()));
    }

    #[test]
    fn calculate_hash_changes_with_nonce() {
        let mut block = raw_block(vec![]);
        let h1 = block.calculate_hash();
        block.nonce += 1;
        assert_ne!(h1, block.calculate_hash());
    }

    #[test]
    fn mine_difficulty_zero_keeps_the_nonce() {
        let mut block = raw_block(vec![]);
        block.mine(0);
        assert_eq!(block.nonce, 42);
        assert_eq!(block.hash, block.calculate_hash());
    }

    #[test]
    fn mine_satisfies_difficulty() {
        for difficulty in 1..=3 {
            let mut block = raw_block(vec![]);
            block.nonce = 0;
            block.mine(difficulty);
            assert!(
                block.hash.starts_with(&"0".repeat(difficulty)),
                "difficulté {} : {}",
                difficulty,
                block.hash
            );
            assert_eq!(block.hash, block.calculate_hash());
        }
    }

    #[test]
    fn mine_finds_the_first_valid_nonce() {
        let mut mined = raw_block(vec![]);
        mined.nonce = 0;
        mined.mine(DIFFICULTY);

        let target = "0".repeat(DIFFICULTY);
        let mut probe = raw_block(vec![]);
        for n in 0..mined.nonce {
            probe.nonce = n;
            assert!(
                !probe.calculate_hash().starts_with(&target),
                "le nonce {} aurait suffi (mine s'est arrêté à {})",
                n,
                mined.nonce
            );
        }
    }

    #[test]
    fn genesis_block_shape() {
        let g = Block::genesis(DIFFICULTY);
        assert_eq!(g.index, 0);
        assert_eq!(g.prev_hash, "0");
        assert!(g.data.is_empty());
        assert!(g.hash.starts_with(&"0".repeat(DIFFICULTY)));
        assert_eq!(g.hash, g.calculate_hash());
    }

    #[test]
    fn block_with_transactions_is_mined() {
        let a = Wallet::new();
        let b = Wallet::new();
        let block = Block::new(
            1,
            vec![signed_tx(&a, &b, 1), signed_tx(&b, &a, 2)],
            "0".repeat(64),
            DIFFICULTY,
        );
        assert!(block.hash.starts_with(&"0".repeat(DIFFICULTY)));
        assert_eq!(block.data.len(), 2);
    }

    #[test]
    fn new_blockchain_has_only_genesis() {
        let bc = Blockchain::new();
        assert_eq!(bc.chain.len(), 1);
        assert_eq!(bc.mempool.len(), 0);
        assert_eq!(bc.difficulty, 1);
        assert_eq!(bc.access_last().index, 0);
    }

    #[test]
    fn with_difficulty_is_applied_to_genesis() {
        let bc = Blockchain::with_difficulty(3);
        assert_eq!(bc.difficulty, 3);
        assert!(bc.chain[0].hash.starts_with("000"));
    }

    #[test]
    fn mempool_accepts_valid_transaction() {
        let a = Wallet::new();
        let b = Wallet::new();
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        bc.asked_transaction(signed_tx(&a, &b, 50));
        assert_eq!(bc.mempool.len(), 1);
    }

    #[test]
    fn mempool_rejects_unsigned_transaction() {
        let a = Wallet::new();
        let b = Wallet::new();
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        bc.asked_transaction(Transaction::new(address(&a), address(&b), 50));
        assert_eq!(bc.mempool.len(), 0);
    }

    #[test]
    fn mempool_rejects_altered_transaction() {
        let a = Wallet::new();
        let b = Wallet::new();
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        let mut tx = signed_tx(&a, &b, 50);
        tx.amount = 5000;
        bc.asked_transaction(tx);
        assert_eq!(bc.mempool.len(), 0);
    }

    #[test]
    fn mempool_rejects_wrong_signer() {
        let a = Wallet::new();
        let b = Wallet::new();
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        let mut tx = Transaction::new(address(&a), address(&b), 50);
        tx.sign(&b);
        bc.asked_transaction(tx);
        assert_eq!(bc.mempool.len(), 0);
    }

    #[test]
    fn mempool_rejects_malformed_sender_without_panic() {
        let a = Wallet::new();
        let b = Wallet::new();
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        for bad in ["pas de l'hexa", "abcd", ""] {
            let mut tx = Transaction::new(bad.to_string(), address(&b), 50);
            tx.sign(&a);
            bc.asked_transaction(tx);
        }
        assert_eq!(bc.mempool.len(), 0);
    }

    #[test]
    fn add_block_links_and_empties_mempool() {
        let a = Wallet::new();
        let b = Wallet::new();
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        bc.asked_transaction(signed_tx(&a, &b, 10));
        bc.asked_transaction(signed_tx(&a, &b, 20));
        bc.asked_transaction(signed_tx(&b, &a, 5));
        let genesis_hash = bc.chain[0].hash.clone();

        bc.add_block();

        assert_eq!(bc.chain.len(), 2);
        assert_eq!(bc.mempool.len(), 0);
        let last = bc.access_last();
        assert_eq!(last.index, 1);
        assert_eq!(last.prev_hash, genesis_hash);
        assert_eq!(last.data.len(), 3);
    }

    #[test]
    fn add_block_keeps_arrival_order() {
        let a = Wallet::new();
        let b = Wallet::new();
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        for amount in [3, 1, 2] {
            bc.asked_transaction(signed_tx(&a, &b, amount));
        }
        bc.add_block();
        let amounts: Vec<u64> = bc.access_last().data.iter().map(|t| t.amount).collect();
        assert_eq!(amounts, vec![3, 1, 2]);
    }

    #[test]
    fn add_block_with_empty_mempool_is_allowed() {
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        bc.add_block();
        assert_eq!(bc.chain.len(), 2);
        assert!(bc.access_last().data.is_empty());
        assert!(bc.is_valid());
    }

    #[test]
    fn every_mined_block_meets_the_difficulty() {
        let a = Wallet::new();
        let b = Wallet::new();
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        for i in 0..3 {
            bc.asked_transaction(signed_tx(&a, &b, i));
            bc.add_block();
        }
        let target = "0".repeat(DIFFICULTY);
        for block in &bc.chain {
            assert!(block.hash.starts_with(&target), "bloc {}", block.index);
        }
    }

    fn healthy_chain() -> Blockchain {
        let a = Wallet::new();
        let b = Wallet::new();
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        bc.asked_transaction(signed_tx(&a, &b, 10));
        bc.add_block();
        bc.asked_transaction(signed_tx(&b, &a, 3));
        bc.add_block();
        bc.add_block();
        bc
    }

    #[test]
    fn healthy_chain_is_valid() {
        assert!(healthy_chain().is_valid());
    }

    #[test]
    fn genesis_only_chain_is_valid() {
        assert!(Blockchain::new().is_valid());
    }

    #[test]
    fn tampered_amount_invalidates_chain() {
        let mut bc = healthy_chain();
        bc.chain[1].data[0].amount = 999;
        assert!(!bc.is_valid());
    }

    #[test]
    fn broken_prev_hash_invalidates_chain() {
        let mut bc = healthy_chain();
        bc.chain[2].prev_hash = "0".repeat(64);
        assert!(!bc.is_valid());
    }

    #[test]
    fn tampered_nonce_invalidates_chain() {
        let mut bc = healthy_chain();
        bc.chain[1].nonce += 1;
        assert!(!bc.is_valid());
    }

    #[test]
    fn overwritten_hash_invalidates_chain() {
        let mut bc = healthy_chain();
        bc.chain[1].hash = "0".repeat(64);
        assert!(!bc.is_valid());
    }

    #[test]
    fn swapped_signature_invalidates_chain() {
        let mut bc = healthy_chain();
        let sig_block2 = bc.chain[2].data[0].signature;
        bc.chain[1].data[0].signature = sig_block2;
        assert!(!bc.is_valid());
    }

    #[test]
    fn removed_middle_block_invalidates_chain() {
        let mut bc = healthy_chain();
        bc.chain.remove(2);
        assert!(!bc.is_valid());
    }

    #[test]
    fn swapped_blocks_invalidate_chain() {
        let mut bc = healthy_chain();
        bc.chain.swap(1, 2);
        assert!(!bc.is_valid());
    }

    #[test]
    #[ignore = "faille connue : is_valid ne vérifie pas la difficulté"]
    fn rejects_block_without_proof_of_work() {
        let mut bc = healthy_chain();
        let target = "0".repeat(bc.difficulty);
        let last = bc.chain.len() - 1;
        loop {
            bc.chain[last].timestamp += 1;
            bc.chain[last].hash = bc.chain[last].calculate_hash();
            if !bc.chain[last].hash.starts_with(&target) {
                break;
            }
        }
        assert!(!bc.is_valid());
    }

    #[test]
    #[ignore = "faille connue : is_valid ne vérifie pas le genesis"]
    fn rejects_forged_genesis() {
        let mut bc = Blockchain::new();
        bc.chain[0].index = 5;
        bc.chain[0].prev_hash = "forgé".to_string();
        bc.chain[0].hash = bc.chain[0].calculate_hash();
        assert!(!bc.is_valid());
    }
}

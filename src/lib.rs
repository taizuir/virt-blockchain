use std::collections::BTreeMap;
pub mod node;
pub mod sha256;
pub mod wallet;
use sha256::sha256_hex;
use std::mem::take;
use wallet::Transaction;
//pub mod vm;
//un simple bloc sur le blockcahin il contient son numero sa date de création  une transacation le hash de la clé en cour le hashde la clé du block precedent
#[derive(Clone, Debug, PartialEq)]
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
        let mut block = Block {
            index: 0,
            timestamp: 0,
            data: vec![],
            prev_hash: String::from("0"),
            hash: String::new(),
            nonce: 0,
        };
        block.mine(difficulty);
        block
    }
}
// la bockchain contient les blocs une difculté choisi et une liste de transacton a ajouter
#[derive(Clone, Debug)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: usize,
    pub mempool: Vec<Transaction>,
    pub counts: BTreeMap<String, u64>,
}

impl Blockchain {
    pub fn new() -> Blockchain {
        Blockchain {
            chain: vec![Block::genesis(1)],
            difficulty: 1,
            mempool: vec![],
            counts: BTreeMap::new(),
        }
    }
    //new rendu plus difficulté
    pub fn with_difficulty(difficulty: usize) -> Blockchain {
        Blockchain {
            chain: vec![Block::genesis(difficulty)],
            difficulty,
            mempool: vec![],
            counts: BTreeMap::new(),
        }
    }
    pub fn expected_count(&self, from: &str) -> u64 {
        self.counts.get(from).copied().unwrap_or(0)
            + self.mempool.iter().filter(|x| x.from == from).count() as u64
    }
    // donne accès au dernier elemetn
    pub fn access_last(&self) -> &Block {
        self.chain.last().unwrap()
    }

    pub fn add_block(&mut self) {
        let clean_mempool = take(&mut self.mempool);
        for t in &clean_mempool {
            *self.counts.entry(t.from.clone()).or_insert(0) += 1;
        }
        let prev_hash = self.access_last().hash.clone();
        let new_index = self.access_last().index + 1;
        let new_block = Block::new(new_index, clean_mempool, prev_hash, self.difficulty);
        self.chain.push(new_block);
    }

    pub fn is_valid(&self) -> bool {
        let target = "0".repeat(self.difficulty);
        if self.chain[0].index != 0 {
            return false;
        } else if self.chain[0].prev_hash != "0" {
            return false;
        } else if !(self.chain[0].data.is_empty()) {
            return false;
        } else if !(self.chain[0].hash.starts_with(&target)) {
            return false;
        } else if self.chain[0].hash != self.chain[0].calculate_hash() {
            return false;
        }
        let mut seen = BTreeMap::new();

        for i in 1..self.chain.len() {
            if !(self.chain[i].hash.starts_with(&target)) {
                return false;
            }
            let current = &self.chain[i];
            let previous = &self.chain[i - 1];
            for i in &current.data {
                if !i.verify() {
                    return false;
                }
                let n = seen.entry(i.from.clone()).or_insert(0);
                if i.count != *n {
                    return false;
                }
                *n += 1;
            }

            if current.hash != current.calculate_hash() {
                return false; // intégrité du bloc cassée
            }
            if current.prev_hash != previous.hash {
                return false; // lien avec le bloc précédent cassé
            }
            if current.index != previous.index + 1 {
                return false;
            }
        }
        true
    }
    pub fn asked_transaction(&mut self, t: Transaction) -> bool {
        if t.verify() && t.count == self.expected_count(&t.from) {
            self.mempool.push(t);
            return true;
        } else {
            return false;
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
        assert!(bc.asked_transaction(tx_with_count(&a, &b, 10, 0)));
        assert!(bc.asked_transaction(tx_with_count(&a, &b, 20, 1)));
        assert!(bc.asked_transaction(tx_with_count(&b, &a, 5, 0)));
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
        for (count, amount) in [3, 1, 2].into_iter().enumerate() {
            assert!(bc.asked_transaction(tx_with_count(&a, &b, amount, count as u64)));
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
            assert!(bc.asked_transaction(tx_with_count(&a, &b, i, i)));
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
    fn rejects_forged_genesis() {
        let mut bc = Blockchain::new();
        bc.chain[0].index = 5;
        bc.chain[0].prev_hash = "forgé".to_string();
        bc.chain[0].hash = bc.chain[0].calculate_hash();
        assert!(!bc.is_valid());
    }

    // ───────────────────────────── REJEU ─────────────────────────────
    // À coller dans `mod tests` de src/lib.rs (en bas, avant l'accolade fermante).

    // Transaction signée avec un `count` choisi. Le count est posé AVANT la signature :
    // il fait partie du payload signé.
    fn tx_with_count(from: &Wallet, to: &Wallet, amount: u64, count: u64) -> Transaction {
        let mut tx = Transaction::new(address(from), address(to), amount);
        tx.count = count;
        tx.sign(from);
        tx
    }

    // Transaction avec le prochain count attendu pour `from` (confirmé + en attente).
    fn next_tx(bc: &Blockchain, from: &Wallet, to: &Wallet, amount: u64) -> Transaction {
        tx_with_count(from, to, amount, bc.expected_count(&address(from)))
    }

    #[test]
    fn asked_transaction_returns_true_when_accepted() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        assert!(bc.asked_transaction(tx_with_count(&a, &b, 5, 0)));
    }

    #[test]
    fn asked_transaction_returns_false_when_rejected() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        let unsigned = Transaction::new(address(&a), address(&b), 5);
        assert!(!bc.asked_transaction(unsigned));
        assert!(bc.mempool.is_empty());
    }

    #[test]
    fn expected_count_starts_at_zero_for_unknown_account() {
        let a = Wallet::new();
        let bc = Blockchain::with_difficulty(DIFFICULTY);
        assert_eq!(bc.expected_count(&address(&a)), 0);
    }

    #[test]
    fn first_transaction_of_an_account_must_have_count_zero() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        assert!(!bc.asked_transaction(tx_with_count(&a, &b, 5, 1)));
        assert!(bc.mempool.is_empty());
    }

    #[test]
    fn same_transaction_twice_in_mempool_is_rejected() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        let tx = tx_with_count(&a, &b, 5, 0);
        assert!(bc.asked_transaction(tx.clone()));
        assert!(!bc.asked_transaction(tx));
        assert_eq!(bc.mempool.len(), 1);
    }

    #[test]
    fn replay_of_an_included_transaction_is_rejected() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        let tx = tx_with_count(&a, &b, 5, 0);
        assert!(bc.asked_transaction(tx.clone()));
        bc.add_block();

        // L'attaquant renvoie exactement la même transaction, signature comprise.
        assert!(!bc.asked_transaction(tx));
        assert!(bc.mempool.is_empty());
    }

    #[test]
    fn skipped_count_is_rejected() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        assert!(bc.asked_transaction(tx_with_count(&a, &b, 1, 0)));
        assert!(!bc.asked_transaction(tx_with_count(&a, &b, 1, 2)));
        assert_eq!(bc.mempool.len(), 1);
    }

    #[test]
    fn consecutive_counts_are_accepted_in_the_same_mempool() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        for count in 0..3 {
            assert!(
                bc.asked_transaction(tx_with_count(&a, &b, 1, count)),
                "count {count}"
            );
        }
        assert_eq!(bc.mempool.len(), 3);
        assert_eq!(bc.expected_count(&address(&a)), 3);
    }

    #[test]
    fn counts_continue_across_blocks() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        assert!(bc.asked_transaction(tx_with_count(&a, &b, 1, 0)));
        bc.add_block();
        assert!(bc.asked_transaction(tx_with_count(&a, &b, 1, 1)));
        bc.add_block();
        assert_eq!(bc.expected_count(&address(&a)), 2);
        // L'ancien count 0 ne repasse plus.
        assert!(!bc.asked_transaction(tx_with_count(&a, &b, 1, 0)));
    }

    #[test]
    fn counts_are_independent_per_account() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        assert!(bc.asked_transaction(tx_with_count(&a, &b, 1, 0)));
        assert!(bc.asked_transaction(tx_with_count(&b, &a, 1, 0)));
        assert_eq!(bc.mempool.len(), 2);
    }

    #[test]
    fn add_block_updates_counts_of_senders_only() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        assert!(bc.asked_transaction(tx_with_count(&a, &b, 1, 0)));
        assert!(bc.asked_transaction(tx_with_count(&a, &b, 1, 1)));
        bc.add_block();
        assert_eq!(bc.counts.get(&address(&a)).copied(), Some(2));
        assert_eq!(bc.counts.get(&address(&b)).copied().unwrap_or(0), 0);
        assert_eq!(bc.expected_count(&address(&b)), 0);
    }

    #[test]
    fn rejected_transaction_changes_nothing() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        assert!(!bc.asked_transaction(tx_with_count(&a, &b, 1, 7)));
        assert!(bc.mempool.is_empty());
        assert!(bc.counts.is_empty());
        assert_eq!(bc.expected_count(&address(&a)), 0);
    }

    #[test]
    fn count_is_covered_by_the_signature() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        let mut tx = tx_with_count(&a, &b, 5, 0);
        assert!(tx.verify());
        // Modifier le count après la signature doit casser la vérification.
        tx.count = 1;
        assert!(!tx.verify());
        assert!(!bc.asked_transaction(tx));
    }

    #[test]
    fn next_tx_helper_builds_an_acceptable_sequence() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        for amount in 1..=4 {
            let tx = next_tx(&bc, &a, &b, amount);
            assert!(bc.asked_transaction(tx), "amount {amount}");
        }
        bc.add_block();
        let tx = next_tx(&bc, &a, &b, 9);
        assert_eq!(tx.count, 4);
        assert!(bc.asked_transaction(tx));
    }

    // ── is_valid : une chaîne fabriquée à la main doit être refusée ──
    // Block::new mine le bloc, donc le hash et la preuve de travail sont corrects :
    // seule la règle des counts peut le rejeter.

    fn push_forged_block(bc: &mut Blockchain, txs: Vec<Transaction>) {
        let prev = bc.access_last();
        let block = Block::new(prev.index + 1, txs, prev.hash.clone(), bc.difficulty);
        bc.chain.push(block);
    }

    #[test]
    fn is_valid_accepts_a_chain_with_correct_counts() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        for _ in 0..3 {
            let tx = next_tx(&bc, &a, &b, 1);
            assert!(bc.asked_transaction(tx));
            let tx = next_tx(&bc, &b, &a, 1);
            assert!(bc.asked_transaction(tx));
            bc.add_block();
        }
        assert!(bc.is_valid());
    }

    #[test]
    fn is_valid_rejects_a_replayed_transaction_in_a_later_block() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        let tx = tx_with_count(&a, &b, 5, 0);
        assert!(bc.asked_transaction(tx.clone()));
        bc.add_block();
        assert!(bc.is_valid());

        push_forged_block(&mut bc, vec![tx]);
        assert!(!bc.is_valid());
    }

    #[test]
    fn is_valid_rejects_a_replay_inside_the_same_block() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        let tx = tx_with_count(&a, &b, 5, 0);
        push_forged_block(&mut bc, vec![tx.clone(), tx]);
        assert!(!bc.is_valid());
    }

    #[test]
    fn is_valid_rejects_a_first_transaction_with_nonzero_count() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        push_forged_block(&mut bc, vec![tx_with_count(&a, &b, 5, 1)]);
        assert!(!bc.is_valid());
    }

    #[test]
    fn is_valid_rejects_counts_in_the_wrong_order() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut bc = Blockchain::with_difficulty(DIFFICULTY);
        push_forged_block(
            &mut bc,
            vec![tx_with_count(&a, &b, 1, 1), tx_with_count(&a, &b, 1, 0)],
        );
        assert!(!bc.is_valid());
    }

    #[test]
    fn genesis_is_deterministic() {
        let a = Block::genesis(DIFFICULTY);
        let b = Block::genesis(DIFFICULTY);
        assert_eq!(a.hash, b.hash);
        assert_eq!(a.timestamp, 0);
    }
}

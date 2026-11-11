use crate::Blockchain;
use crate::wallet::Transaction;

struct Node {
    id: u32,
    addr: String,
    chain: Blockchain,
    peers: Vec<String>,
}
impl Node {
    pub fn new(id: u32, addr: String, difficulty: usize) -> Node {
        let chain = Blockchain::with_difficulty(difficulty);
        let peers: Vec<String> = vec![];
        Node {
            id,
            addr,
            chain,
            peers,
        }
    }
    pub fn blockchain_len(&self) -> usize {
        self.chain.chain.len()
    }
    pub fn submit_transaction(&mut self, t: Transaction) -> bool {
        self.chain.asked_transaction(t)
    }
    pub fn mine(&mut self) -> () {
        self.chain.add_block();
    }
    pub fn is_chain_valid(&self) -> bool {
        self.chain.is_valid()
    }
    pub fn add_peer(&mut self, addr: String) -> bool {
        if addr == self.addr || self.peers.contains(&addr) {
            return false;
        }
        self.peers.push(addr);
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Wallet;

    const DIFFICULTY: usize = 2;

    fn address(w: &Wallet) -> String {
        hex::encode(w.public_key.to_bytes())
    }

    // Transaction signée avec un `count` choisi (posé AVANT la signature).
    fn signed_tx(from: &Wallet, to: &Wallet, amount: u64, count: u64) -> Transaction {
        let mut tx = Transaction::new(address(from), address(to), amount);
        tx.count = count;
        tx.sign(from);
        tx
    }

    fn test_node(id: u32, port: u16) -> Node {
        Node::new(id, format!("127.0.0.1:{port}"), DIFFICULTY)
    }

    // 1
    #[test]
    fn new_node_has_valid_chain_of_one_block() {
        let node = test_node(1, 7001);
        assert_eq!(node.blockchain_len(), 1);
        assert!(node.is_chain_valid());
    }

    // 2
    #[test]
    fn new_node_keeps_its_parameters_and_starts_empty() {
        let node = test_node(7, 7007);
        assert_eq!(node.id, 7);
        assert_eq!(node.addr, "127.0.0.1:7007");
        assert!(node.peers.is_empty());
        assert!(node.chain.mempool.is_empty());
        assert_eq!(node.chain.difficulty, DIFFICULTY);
    }

    // 3
    #[test]
    fn two_nodes_have_the_same_genesis_hash() {
        let a = test_node(1, 7001);
        let b = test_node(2, 7002);
        assert_eq!(a.chain.chain[0].hash, b.chain.chain[0].hash);
    }

    // 4
    #[test]
    fn submit_valid_transaction_returns_true_and_fills_mempool() {
        let (alice, bob) = (Wallet::new(), Wallet::new());
        let mut node = test_node(1, 7001);
        assert!(node.submit_transaction(signed_tx(&alice, &bob, 5, 0)));
        assert_eq!(node.chain.mempool.len(), 1);
    }

    // 5
    #[test]
    fn submit_invalid_transaction_returns_false() {
        let (alice, bob) = (Wallet::new(), Wallet::new());
        let mut node = test_node(1, 7001);

        // non signée
        let unsigned = Transaction::new(address(&alice), address(&bob), 5);
        assert!(!node.submit_transaction(unsigned));

        // mauvais count (un compte neuf attend 0)
        assert!(!node.submit_transaction(signed_tx(&alice, &bob, 5, 3)));

        // rejeu : la même transaction deux fois
        let tx = signed_tx(&alice, &bob, 5, 0);
        assert!(node.submit_transaction(tx.clone()));
        assert!(!node.submit_transaction(tx));

        assert_eq!(node.chain.mempool.len(), 1);
    }

    // 6
    #[test]
    fn mine_adds_a_block_and_empties_mempool() {
        let (alice, bob) = (Wallet::new(), Wallet::new());
        let mut node = test_node(1, 7001);
        assert!(node.submit_transaction(signed_tx(&alice, &bob, 5, 0)));
        node.mine();
        assert_eq!(node.blockchain_len(), 2);
        assert!(node.chain.mempool.is_empty());
        assert_eq!(node.chain.access_last().data.len(), 1);
    }

    // 7
    #[test]
    fn mined_chain_stays_valid() {
        let (alice, bob) = (Wallet::new(), Wallet::new());
        let mut node = test_node(1, 7001);
        for count in 0..3 {
            assert!(node.submit_transaction(signed_tx(&alice, &bob, 1, count)));
            node.mine();
        }
        assert_eq!(node.blockchain_len(), 4);
        assert!(node.is_chain_valid());
    }

    // 8
    #[test]
    fn nodes_are_independent() {
        let (alice, bob) = (Wallet::new(), Wallet::new());
        let mut a = test_node(1, 7001);
        let mut b = test_node(2, 7002);
        assert!(a.submit_transaction(signed_tx(&alice, &bob, 5, 0)));
        a.mine();

        assert_eq!(a.blockchain_len(), 2);
        assert_eq!(b.blockchain_len(), 1);
        assert!(b.chain.mempool.is_empty());

        // B ne connaît pas la transaction d'Alice : son compteur est resté à 0.
        assert!(b.submit_transaction(signed_tx(&alice, &bob, 5, 0)));
    }

    // 9
    #[test]
    fn add_peer_accepts_a_new_address() {
        let mut node = test_node(1, 7001);
        assert!(node.add_peer("127.0.0.1:7002".to_string()));
        assert_eq!(node.peers, vec!["127.0.0.1:7002".to_string()]);
    }

    // 10
    #[test]
    fn add_peer_ignores_duplicate() {
        let mut node = test_node(1, 7001);
        assert!(node.add_peer("127.0.0.1:7002".to_string()));
        assert!(!node.add_peer("127.0.0.1:7002".to_string()));
        assert_eq!(node.peers.len(), 1);
    }

    // 11
    #[test]
    fn add_peer_ignores_own_address() {
        let mut node = test_node(1, 7001);
        assert!(!node.add_peer("127.0.0.1:7001".to_string()));
        assert!(node.peers.is_empty());
    }

    // bonus : plusieurs voisins, dans l'ordre d'ajout
    #[test]
    fn add_peer_keeps_insertion_order() {
        let mut node = test_node(1, 7001);
        for port in [7003, 7002, 7004] {
            assert!(node.add_peer(format!("127.0.0.1:{port}")));
        }
        assert_eq!(
            node.peers,
            vec![
                "127.0.0.1:7003".to_string(),
                "127.0.0.1:7002".to_string(),
                "127.0.0.1:7004".to_string(),
            ]
        );
    }
}

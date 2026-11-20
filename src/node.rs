use crate::Blockchain;
use crate::message::Message::{self, Chain, GetChain, NewBlock, NewTransaction};
use crate::wallet::Transaction;
use ed25519_dalek::VerifyingKey;
use ed25519_dalek::ed25519::signature::Result;
use std::mem::take;

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
    pub fn handle_message(&mut self, msg: Message, from: &str) -> Vec<(String, Message)> {
        let others: Vec<String> = self
            .peers
            .iter()
            .filter(|p| p.as_str() != from)
            .cloned()
            .collect();
        match msg {
            Message::NewTransaction(t) => {
                if !self.submit_transaction(t.clone()) {
                    return vec![];
                }
                others
                    .into_iter()
                    .map(|p| (p, Message::NewTransaction(t.clone())))
                    .collect()
            }
            Message::NewBlock(b) => {
                let behind = b.index > self.chain.access_last().index + 1;
                self.chain.chain.push(b.clone());
                if !self.chain.is_valid() {
                    self.chain.chain.pop();
                    return if behind {
                        vec![(from.to_string(), Message::GetChain)]
                    } else {
                        vec![]
                    };
                }
                for t in &b.data {
                    *self.chain.counts.entry(t.from.clone()).or_insert(0) += 1;
                }
                for t in take(&mut self.chain.mempool) {
                    if !b.data.contains(&t) {
                        self.chain.asked_transaction(t);
                    }
                }
                others
                    .into_iter()
                    .map(|p| (p, Message::NewBlock(b.clone())))
                    .collect()
            }
            Message::GetChain => vec![(from.to_string(), Message::Chain(self.chain.chain.clone()))],
            Message::Chain(blocks) => {
                if blocks.len() <= self.chain.chain.len() || blocks[0] != self.chain.chain[0] {
                    return vec![];
                }
                let mut candidate = self.chain.clone();
                candidate.chain = blocks;
                if !candidate.is_valid() {
                    return vec![];
                }
                candidate.counts.clear();
                for block in &candidate.chain {
                    for t in &block.data {
                        *candidate.counts.entry(t.from.clone()).or_insert(0) += 1;
                    }
                }
                let pending = take(&mut candidate.mempool);
                self.chain = candidate;
                for t in pending {
                    self.chain.asked_transaction(t);
                }
                vec![]
            }
        }
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
    fn node_with_peers(id: u32, port: u16, peers: &[u16]) -> Node {
        let mut n = test_node(id, port);
        for p in peers {
            assert!(n.add_peer(format!("127.0.0.1:{p}")));
        }
        n
    }

    fn addr(port: u16) -> String {
        format!("127.0.0.1:{port}")
    }

    #[test]
    fn new_valid_transaction_is_forwarded_to_all_peers_but_sender() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut node = node_with_peers(1, 7001, &[7002, 7003, 7004]);
        let t = signed_tx(&a, &b, 5, 0);
        let out = node.handle_message(Message::NewTransaction(t.clone()), &addr(7002));
        assert_eq!(node.chain.mempool.len(), 1);
        let dests: Vec<String> = out.iter().map(|(d, _)| d.clone()).collect();
        assert_eq!(dests, vec![addr(7003), addr(7004)]);
        assert!(
            out.iter()
                .all(|(_, m)| *m == Message::NewTransaction(t.clone()))
        );
    }

    #[test]
    fn known_transaction_is_not_forwarded_again() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut node = node_with_peers(1, 7001, &[7002, 7003]);
        let t = signed_tx(&a, &b, 5, 0);
        assert!(
            !node
                .handle_message(Message::NewTransaction(t.clone()), &addr(7002))
                .is_empty()
        );
        assert!(
            node.handle_message(Message::NewTransaction(t), &addr(7003))
                .is_empty()
        );
        assert_eq!(node.chain.mempool.len(), 1);
    }

    #[test]
    fn invalid_transaction_is_not_forwarded() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut node = node_with_peers(1, 7001, &[7002, 7003]);
        let unsigned = Transaction::new(address(&a), address(&b), 5);
        assert!(
            node.handle_message(Message::NewTransaction(unsigned), &addr(7002))
                .is_empty()
        );
        assert!(
            node.handle_message(
                Message::NewTransaction(signed_tx(&a, &b, 5, 9)),
                &addr(7002)
            )
            .is_empty()
        );
        assert!(node.chain.mempool.is_empty());
    }

    #[test]
    fn forwarding_with_no_peers_returns_nothing() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut node = test_node(1, 7001);
        let out = node.handle_message(
            Message::NewTransaction(signed_tx(&a, &b, 5, 0)),
            &addr(7002),
        );
        assert!(out.is_empty());
        assert_eq!(node.chain.mempool.len(), 1);
    }

    #[test]
    fn get_chain_answers_with_the_full_chain_to_the_sender() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut node = node_with_peers(1, 7001, &[7002, 7003]);
        assert!(node.submit_transaction(signed_tx(&a, &b, 5, 0)));
        node.mine();
        let out = node.handle_message(Message::GetChain, &addr(7003));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, addr(7003));
        assert_eq!(out[0].1, Message::Chain(node.chain.chain.clone()));
        assert_eq!(node.blockchain_len(), 2);
    }

    #[test]
    fn valid_block_is_accepted_cleans_mempool_and_is_forwarded() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut miner = test_node(1, 7001);
        let mut node = node_with_peers(2, 7002, &[7001, 7003]);
        let t0 = signed_tx(&a, &b, 5, 0);
        assert!(miner.submit_transaction(t0.clone()));
        miner.mine();
        assert!(node.submit_transaction(t0.clone()));
        let block = miner.chain.access_last().clone();

        let out = node.handle_message(Message::NewBlock(block.clone()), &addr(7001));
        assert_eq!(node.blockchain_len(), 2);
        assert!(node.chain.mempool.is_empty());
        assert!(node.is_chain_valid());
        assert_eq!(node.chain.expected_count(&address(&a)), 1);
        assert_eq!(out, vec![(addr(7003), Message::NewBlock(block))]);
    }

    #[test]
    fn same_block_twice_is_not_forwarded_again() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut miner = test_node(1, 7001);
        let mut node = node_with_peers(2, 7002, &[7001, 7003]);
        assert!(miner.submit_transaction(signed_tx(&a, &b, 5, 0)));
        miner.mine();
        let block = miner.chain.access_last().clone();
        assert!(
            !node
                .handle_message(Message::NewBlock(block.clone()), &addr(7001))
                .is_empty()
        );
        assert!(
            node.handle_message(Message::NewBlock(block), &addr(7001))
                .is_empty()
        );
        assert_eq!(node.blockchain_len(), 2);
    }

    #[test]
    fn invalid_block_is_rejected_and_chain_is_untouched() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut miner = test_node(1, 7001);
        let mut node = node_with_peers(2, 7002, &[7001, 7003]);
        assert!(miner.submit_transaction(signed_tx(&a, &b, 5, 0)));
        miner.mine();
        let good = miner.chain.access_last().clone();
        let before = node.chain.chain.clone();

        let mut tampered = good.clone();
        tampered.data[0].amount = 999;
        assert!(
            node.handle_message(Message::NewBlock(tampered), &addr(7001))
                .is_empty()
        );

        let mut wrong_link = good.clone();
        wrong_link.prev_hash = "ff".to_string();
        assert!(
            node.handle_message(Message::NewBlock(wrong_link), &addr(7001))
                .is_empty()
        );

        let mut fake_pow = good;
        fake_pow.nonce += 1;
        assert!(
            node.handle_message(Message::NewBlock(fake_pow), &addr(7001))
                .is_empty()
        );

        assert_eq!(node.chain.chain, before);
        assert!(node.chain.counts.is_empty());
    }

    #[test]
    fn block_far_ahead_triggers_a_get_chain_to_the_sender() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut miner = test_node(1, 7001);
        for c in 0..3 {
            assert!(miner.submit_transaction(signed_tx(&a, &b, 1, c)));
            miner.mine();
        }
        let mut node = node_with_peers(2, 7002, &[7001]);
        let tip = miner.chain.access_last().clone();
        let out = node.handle_message(Message::NewBlock(tip), &addr(7001));
        assert_eq!(out, vec![(addr(7001), Message::GetChain)]);
        assert_eq!(node.blockchain_len(), 1);
    }

    #[test]
    fn longer_valid_chain_is_adopted() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut leader = test_node(1, 7001);
        for c in 0..3 {
            assert!(leader.submit_transaction(signed_tx(&a, &b, 1, c)));
            leader.mine();
        }
        let mut node = node_with_peers(2, 7002, &[7001]);
        let stale = signed_tx(&a, &b, 1, 0);
        assert!(node.submit_transaction(stale));
        let out = node.handle_message(Message::Chain(leader.chain.chain.clone()), &addr(7001));
        assert!(out.is_empty());
        assert_eq!(node.blockchain_len(), 4);
        assert!(node.is_chain_valid());
        assert_eq!(node.chain.expected_count(&address(&a)), 3);
        assert!(node.chain.mempool.is_empty());
    }

    #[test]
    fn shorter_equal_or_invalid_chains_are_ignored() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut node = test_node(1, 7001);
        for c in 0..2 {
            assert!(node.submit_transaction(signed_tx(&a, &b, 1, c)));
            node.mine();
        }
        let before = node.chain.chain.clone();

        let mut shorter = test_node(2, 7002);
        assert!(shorter.submit_transaction(signed_tx(&a, &b, 1, 0)));
        shorter.mine();
        node.handle_message(Message::Chain(shorter.chain.chain.clone()), &addr(7002));
        node.handle_message(Message::Chain(before.clone()), &addr(7002));
        assert_eq!(node.chain.chain, before);

        let mut longer = test_node(3, 7003);
        for c in 0..4 {
            assert!(longer.submit_transaction(signed_tx(&a, &b, 1, c)));
            longer.mine();
        }
        let mut forged = longer.chain.chain.clone();
        forged[2].data[0].amount = 777;
        node.handle_message(Message::Chain(forged), &addr(7003));
        assert_eq!(node.chain.chain, before);
        assert_eq!(node.chain.expected_count(&address(&a)), 2);
    }

    #[test]
    fn chain_with_a_different_genesis_is_ignored() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut node = test_node(1, 7001);
        let mut other = Node::new(2, addr(7002), DIFFICULTY);
        other.chain.chain[0].nonce += 1;
        for c in 0..3 {
            assert!(other.submit_transaction(signed_tx(&a, &b, 1, c)));
            other.mine();
        }
        node.handle_message(Message::Chain(other.chain.chain.clone()), &addr(7002));
        assert_eq!(node.blockchain_len(), 1);
    }

    #[test]
    fn two_nodes_converge_through_messages() {
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut x = node_with_peers(1, 7001, &[7002]);
        let mut y = node_with_peers(2, 7002, &[7001]);
        let t = signed_tx(&a, &b, 5, 0);
        let out = x.handle_message(Message::NewTransaction(t), &addr(9999));
        for (dest, m) in out {
            assert_eq!(dest, addr(7002));
            y.handle_message(m, &addr(7001));
        }
        x.mine();
        let block = x.chain.access_last().clone();
        y.handle_message(Message::NewBlock(block), &addr(7001));
        assert_eq!(x.chain.chain, y.chain.chain);
        assert!(y.is_chain_valid());
    }
}

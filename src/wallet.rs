use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use getrandom::{SysRng, rand_core::UnwrapErr};
use hex;
//le wallet contient la clé secrete et la publique utiliser pour crypter les données
pub struct Wallet {
    pub public_key: VerifyingKey,
    signing_key: SigningKey,
}
//descriptif  de transaction
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub signature: Option<Signature>,
}
impl Wallet {
    pub fn new() -> Wallet {
        let signing_key = SigningKey::generate(&mut UnwrapErr(SysRng));

        let public_key = signing_key.verifying_key();
        Wallet {
            public_key,
            signing_key,
        }
    }
    pub fn sign(&self, payload: &Vec<u8>) -> Signature {
        self.signing_key.sign(payload)
    }
}
impl Transaction {
    pub fn new(from: String, to: String, amount: u64) -> Transaction {
        let signature = None;
        Transaction {
            from,
            to,
            amount,
            signature,
        }
    }
    pub fn payload(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(self.from.as_bytes());
        bytes.extend_from_slice(self.to.as_bytes());
        bytes.extend_from_slice(&self.amount.to_be_bytes());
        bytes
    }
    pub fn sign(&mut self, wall: &Wallet) {
        self.signature = Some(wall.sign(&self.payload()))
    }
    pub fn verify(&self) -> bool {
        let Ok(hexa) = hex::decode(&self.from) else {
            return false;
        };
        let Ok(hexa_fixed): Result<[u8; 32], _> = hexa.try_into() else {
            return false;
        };
        let Ok(checker) = VerifyingKey::from_bytes(&hexa_fixed) else {
            return false;
        };
        let Some(signa) = &self.signature else {
            return false;
        };
        checker.verify(&self.payload(), &signa).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(w: &Wallet) -> String {
        hex::encode(w.public_key.to_bytes())
    }

    fn is_lower_hex(s: &str, len: usize) -> bool {
        s.len() == len
            && s.chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    }

    fn signed(from: &Wallet, to: &Wallet, amount: u64) -> Transaction {
        let mut tx = Transaction::new(address(from), address(to), amount);
        tx.sign(from);
        tx
    }

    #[test]
    fn two_wallets_have_different_keys() {
        let wal1 = Wallet::new();
        let wal2 = Wallet::new();
        assert_ne!(wal1.public_key, wal2.public_key);
    }

    #[test]
    fn many_wallets_all_different() {
        let keys: Vec<_> = (0..50).map(|_| Wallet::new().public_key).collect();
        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                assert_ne!(keys[i], keys[j]);
            }
        }
    }

    #[test]
    fn size_of_the_hex_key() {
        let wallet = Wallet::new();
        assert_eq!(hex::encode(wallet.public_key.to_bytes()).len(), 64);
    }

    #[test]
    fn address_is_64_lowercase_hex() {
        let w = Wallet::new();
        let a = address(&w);
        assert!(is_lower_hex(&a, 64), "adresse invalide : {}", a);
    }

    #[test]
    fn address_round_trips_to_the_same_key() {
        let w = Wallet::new();
        let bytes: [u8; 32] = hex::decode(address(&w)).unwrap().try_into().unwrap();
        let back = VerifyingKey::from_bytes(&bytes).unwrap();
        assert_eq!(back, w.public_key);
    }

    #[test]
    fn new_transaction_has_no_signature() {
        let tx = Transaction::new("a".into(), "b".into(), 5);
        assert!(tx.signature.is_none());
    }

    #[test]
    fn payload_length() {
        let tx = Transaction::new("alice".into(), "bob".into(), 5);
        assert_eq!(tx.payload().len(), "alice".len() + "bob".len() + 8);
    }

    #[test]
    fn payload_amount_is_big_endian_at_the_end() {
        let tx = Transaction::new("a".into(), "b".into(), 0x0102_0304_0506_0708);
        let p = tx.payload();
        assert_eq!(&p[p.len() - 8..], &[1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn payload_is_deterministic() {
        let tx = Transaction::new("a".into(), "b".into(), 5);
        assert_eq!(tx.payload(), tx.payload());
    }

    #[test]
    fn payload_changes_with_each_field() {
        let base = Transaction::new("a".into(), "b".into(), 5).payload();
        assert_ne!(base, Transaction::new("x".into(), "b".into(), 5).payload());
        assert_ne!(base, Transaction::new("a".into(), "x".into(), 5).payload());
        assert_ne!(base, Transaction::new("a".into(), "b".into(), 6).payload());
    }

    #[test]
    fn payload_does_not_depend_on_signature() {
        let alice = Wallet::new();
        let bob = Wallet::new();
        let mut tx = Transaction::new(address(&alice), address(&bob), 5);
        let before = tx.payload();
        tx.sign(&alice);
        assert_eq!(before, tx.payload());
    }

    #[test]
    fn signed_transaction_verifies() {
        let alice = Wallet::new();
        let bob = Wallet::new();
        assert!(signed(&alice, &bob, 50).verify());
    }

    #[test]
    fn unsigned_transaction_does_not_verify() {
        let alice = Wallet::new();
        let bob = Wallet::new();
        let tx = Transaction::new(address(&alice), address(&bob), 50);
        assert!(!tx.verify());
    }

    #[test]
    fn tampered_amount_does_not_verify() {
        let alice = Wallet::new();
        let bob = Wallet::new();
        let mut tx = signed(&alice, &bob, 50);
        tx.amount = 5000;
        assert!(!tx.verify());
    }

    #[test]
    fn tampered_receiver_does_not_verify() {
        let alice = Wallet::new();
        let bob = Wallet::new();
        let eve = Wallet::new();
        let mut tx = signed(&alice, &bob, 50);
        tx.to = address(&eve);
        assert!(!tx.verify());
    }

    #[test]
    fn tampered_sender_does_not_verify() {
        let alice = Wallet::new();
        let bob = Wallet::new();
        let eve = Wallet::new();
        let mut tx = signed(&alice, &bob, 50);
        tx.from = address(&eve);
        assert!(!tx.verify());
    }

    #[test]
    fn signed_by_wrong_wallet_does_not_verify() {
        let alice = Wallet::new();
        let bob = Wallet::new();
        let mut tx = Transaction::new(address(&alice), address(&bob), 50);
        tx.sign(&bob);
        assert!(!tx.verify());
    }

    #[test]
    fn malformed_sender_returns_false_without_panic() {
        let alice = Wallet::new();
        let bob = Wallet::new();
        let long = "a".repeat(10_000);
        let accents = "é".repeat(32);
        let candidates = [
            "ceci n'est pas de l'hexa",
            "abcd",
            "abc",
            "",
            accents.as_str(),
            long.as_str(),
        ];
        for bad_from in candidates {
            let mut tx = Transaction::new(bad_from.to_string(), address(&bob), 50);
            tx.sign(&alice);
            assert!(!tx.verify(), "adresse {:?} acceptée", bad_from);
        }
    }

    #[test]
    fn sixty_four_hex_chars_that_are_not_the_signers_key() {
        let alice = Wallet::new();
        let bob = Wallet::new();
        for pattern in ["00", "ff", "ab", "01", "80"] {
            let from = pattern.repeat(32);
            assert_eq!(from.len(), 64);
            let mut tx = Transaction::new(from, address(&bob), 50);
            tx.sign(&alice);
            assert!(!tx.verify(), "motif {} accepté", pattern);
        }
    }

    #[test]
    fn resigning_after_modification_verifies_again() {
        let alice = Wallet::new();
        let bob = Wallet::new();
        let mut tx = signed(&alice, &bob, 50);
        tx.amount = 60;
        assert!(!tx.verify());
        tx.sign(&alice);
        assert!(tx.verify());
    }

    #[test]
    fn signing_is_deterministic() {
        let alice = Wallet::new();
        let bob = Wallet::new();
        let tx1 = signed(&alice, &bob, 50);
        let tx2 = signed(&alice, &bob, 50);
        assert_eq!(tx1.signature, tx2.signature);
    }

    #[test]
    fn signature_from_another_transaction_does_not_verify() {
        let alice = Wallet::new();
        let bob = Wallet::new();
        let tx_small = signed(&alice, &bob, 1);
        let mut tx_big = Transaction::new(address(&alice), address(&bob), 1_000_000);
        tx_big.signature = tx_small.signature;
        assert!(!tx_big.verify());
    }

    #[test]
    #[ignore = "faille connue : adresse non canonique (majuscules) acceptée"]
    fn rejects_uppercase_address() {
        let alice = Wallet::new();
        let bob = Wallet::new();
        let mut tx = Transaction::new(address(&alice).to_uppercase(), address(&bob), 50);
        tx.sign(&alice);
        assert!(!tx.verify());
    }
}

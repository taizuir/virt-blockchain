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
mod test {
    use crate::wallet;
    use 

    use super::*;
    #[test]
    fn two_wallet_two_ashes() {
        let wal1 = Wallet::new();
        let wal2 = Wallet::new();
        assert_ne!(wal1.public_key, wal2.public_key)
    }
    #[test]
    fn size_of_the_hex_key() {
        let wallet = Wallet::new();
        assert_eq!(hex::encode(wallet.public_key.to_bytes()).len(), 64);
        assert!(hex::encode(wallet.public_key.to_bytes()).is_ascii_lowercase());
    }
    #[test]
    fn transaction_signature_field_is_correctly_initialised(){
        let tr =Transaction::new("","",0) ;
        assert_eq!(tr.signature,None);
    }
    #[test]
    fn payload_testing(){

    }
}

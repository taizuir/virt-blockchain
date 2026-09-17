use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
pub struct Wallet {
    pub public_key: Signature,
    signing_key: SigningKey,
}
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub signature: Option<Signature>,
}
impl Wallet {
    pub fn new() -> Wallet {
        let signing_key = SigningKey::generate(&mut OsRng);

        let signature = signing_key.sign(message);
        Wallet {
            signature,
            signing_key,
        }
    }
}

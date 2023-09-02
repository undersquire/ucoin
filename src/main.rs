use std::{
    collections::{HashMap, HashSet},
    time::UNIX_EPOCH,
};

use base64ct::{Base64, Encoding};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Transaction {
    parents: Vec<String>,
    sender: String,
    timestamp: u64,
    amount: u64,
    receiver: String,
}

impl Transaction {
    fn new(
        parents: &[SignedTransaction],
        pk: &oqs::sig::PublicKey,
        amount: u64,
        receiver: &oqs::sig::PublicKey,
    ) -> Self {
        let parents = parents.iter().map(|t| t.hash()).collect();
        let sender = Base64::encode_string(pk.as_ref());
        let receiver = Base64::encode_string(receiver.as_ref());

        Self {
            parents,
            sender,
            timestamp: std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            amount,
            receiver,
        }
    }

    fn sign(self, sig: &oqs::sig::Sig, sk: &oqs::sig::SecretKey) -> SignedTransaction {
        let serialized = serde_json::to_string(&self).unwrap();

        let signature = sig.sign(serialized.as_bytes(), sk).unwrap();
        let signature = Base64::encode_string(signature.as_ref());

        SignedTransaction {
            transaction: self,
            signature,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct SignedTransaction {
    //#[serde(flatten)]
    transaction: Transaction,
    signature: String,
}

impl SignedTransaction {
    fn hash(&self) -> String {
        let serialized = serde_json::to_string(&self).unwrap();
        let hash = blake3::hash(serialized.as_bytes());

        Base64::encode_string(hash.as_bytes())
    }

    fn verify(&self, sig: &oqs::sig::Sig) -> bool {
        let serialized = serde_json::to_string(&self.transaction).unwrap();

        let pk = Base64::decode_vec(&self.transaction.sender).unwrap();
        let pk = sig.public_key_from_bytes(&pk).unwrap();

        let signature = Base64::decode_vec(&self.signature).unwrap();
        let signature = sig.signature_from_bytes(&signature).unwrap();

        sig.verify(serialized.as_bytes(), &signature, &pk).is_ok()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Wallet {
    balance: u64,
    history: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorldState {
    wallets: HashMap<String, Wallet>,
}

fn main() {
    oqs::init();

    let sig = oqs::sig::Sig::new(oqs::sig::Algorithm::Falcon1024).unwrap();

    let sender = sig.keypair().unwrap();
    let receiver = sig.keypair().unwrap();

    let start = std::time::Instant::now();
    let transaction = Transaction::new(&[], &sender.0, 1000, &receiver.0).sign(&sig, &sender.1);
    let sign_duration = start.elapsed();

    let start = std::time::Instant::now();
    let verified = transaction.verify(&sig);
    let verify_duration = start.elapsed();

    let serialized = serde_json::to_string_pretty(&transaction).unwrap();

    println!("{serialized}");
    println!(
        "VERIFIED: {}, {:?} sign, {:?} verify, NIST Level {}",
        verified,
        sign_duration,
        verify_duration,
        sig.claimed_nist_level()
    );

    let start = std::time::Instant::now();
    let serialized = serde_json::to_string(&transaction).unwrap();
    let ser_duration = start.elapsed();

    let start = std::time::Instant::now();
    let _deserialized: SignedTransaction = serde_json::from_str(&serialized).unwrap();
    let de_duration = start.elapsed();

    println!(
        "JSON: {} bytes, {:?} ser, {:?} de",
        serialized.as_bytes().len(),
        ser_duration,
        de_duration
    );
}

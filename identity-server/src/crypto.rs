use rand::Rng;
use sha2::{Digest, Sha256};

const KEY_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

pub struct GeneratedKey {
    pub full_key: String,
    pub prefix: String,
    pub hash: String,
}

pub fn generate_key() -> GeneratedKey {
    let mut rng = rand::rng();

    // Generate 8-char prefix
    let prefix_chars: String = (0..8)
        .map(|_| KEY_CHARS[rng.random_range(0..KEY_CHARS.len())] as char)
        .collect();

    // Generate 32-char random part
    let random_part: String = (0..32)
        .map(|_| KEY_CHARS[rng.random_range(0..KEY_CHARS.len())] as char)
        .collect();

    let full_key = format!("sk_{}_{}", prefix_chars, random_part);
    let prefix = format!("sk_{}", prefix_chars);
    let hash = hash_key(&full_key);

    GeneratedKey {
        full_key,
        prefix,
        hash,
    }
}

pub fn hash_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

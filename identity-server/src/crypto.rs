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

/// Generated activation code with its components
pub struct GeneratedActivationCode {
    pub full_code: String,
    pub prefix: String,
    pub hash: String,
}

/// Generates an activation code in the format: ac_XXXX-XXXX-XXXX
/// These are one-time use codes that can be exchanged for API keys
pub fn generate_activation_code() -> GeneratedActivationCode {
    let mut rng = rand::rng();

    // Generate 3 groups of 4 chars each
    let groups: Vec<String> = (0..3)
        .map(|_| {
            (0..4)
                .map(|_| KEY_CHARS[rng.random_range(0..KEY_CHARS.len())] as char)
                .collect()
        })
        .collect();

    let full_code = format!("ac_{}-{}-{}", groups[0], groups[1], groups[2]);
    let prefix = format!("ac_{}", groups[0]);
    let hash = hash_key(&full_code);

    GeneratedActivationCode {
        full_code,
        prefix,
        hash,
    }
}

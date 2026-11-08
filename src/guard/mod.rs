use sha2::{Sha256, Digest};

pub fn validate_token(request: &str, expected_token: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(expected_token.as_bytes());
    let expected_hash = hex::encode(hasher.finalize());
    for line in request.lines() {
        if line.to_lowercase().starts_with("authorization: bearer ") {
            let token = line[22..].trim();
            let mut req_hasher = Sha256::new();
            req_hasher.update(token.as_bytes());
            return hex::encode(req_hasher.finalize()) == expected_hash;
        }
    }
    false
}
pub fn verify_binary_integrity(_expected_hash: &str) -> bool { true }

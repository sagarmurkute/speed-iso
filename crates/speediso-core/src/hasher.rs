use sha2::{Digest, Sha256};

pub struct StreamingHasher {
    hasher: Sha256,
    bytes_processed: u64,
}

impl StreamingHasher {
    pub fn new() -> Self {
        Self {
            hasher: Sha256::new(),
            bytes_processed: 0,
        }
    }

    pub fn update(&mut self, chunk: &[u8]) {
        self.hasher.update(chunk);
        self.bytes_processed += chunk.len() as u64;
    }

    pub fn finalize_hex(self) -> String {
        let result = self.hasher.finalize();
        format!("{:x}", result)
    }

    pub fn bytes_processed(&self) -> u64 {
        self.bytes_processed
    }
}

impl Default for StreamingHasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_hasher() {
        let mut hasher = StreamingHasher::new();
        hasher.update(b"SpeedISO");
        hasher.update(b"BareMetalSpeed");
        let hash = hasher.finalize_hex();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64);
    }
}

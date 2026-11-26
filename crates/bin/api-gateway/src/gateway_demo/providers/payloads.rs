use ring::hmac;

use super::super::sealed::SealedBytes;

pub struct Payloads {
    encryption_key: SealedBytes,
    intent_key: hmac::Key,
    pub(in crate::gateway_demo) media: SealedBytes,
}

impl Payloads {
    pub fn new(root: &[u8; 32]) -> Self {
        let root_key = hmac::Key::new(hmac::HMAC_SHA256, root);
        let intent = hmac::sign(&root_key, b"tokencraft/dispatch-intent/hmac/v1");
        Self {
            encryption_key: SealedBytes::new(root, b"tokencraft/dispatch-payload/aes256/v1"),
            intent_key: hmac::Key::new(hmac::HMAC_SHA256, intent.as_ref()),
            media: SealedBytes::new(root, b"tokencraft/artifact-body/aes256/v1"),
        }
    }

    pub fn fingerprint(&self, bytes: &[u8]) -> String {
        hex::encode(hmac::sign(&self.intent_key, bytes).as_ref())
    }

    pub fn seal(&self, request_id: &str, field: &str, bytes: Vec<u8>) -> Option<Vec<u8>> {
        self.encryption_key
            .seal(&format!("{request_id}/{field}"), bytes)
    }

    pub fn open(&self, request_id: &str, field: &str, bytes: &[u8]) -> Option<Vec<u8>> {
        self.encryption_key
            .open(&format!("{request_id}/{field}"), bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::Payloads;

    #[test]
    fn encrypted_payload_is_bound_to_its_request_and_purpose() {
        let payloads = Payloads::new(&[42; 32]);
        let encrypted = payloads
            .seal("req_a", "input", b"example".to_vec())
            .unwrap();
        assert_eq!(
            payloads.open("req_a", "input", &encrypted).unwrap(),
            b"example"
        );
        assert!(payloads.open("req_b", "input", &encrypted).is_none());
        assert!(payloads.open("req_a", "result", &encrypted).is_none());
        assert!(Payloads::new(&[43; 32])
            .open("req_a", "input", &encrypted)
            .is_none());
    }
}

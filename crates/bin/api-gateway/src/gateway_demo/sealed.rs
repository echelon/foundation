use ring::{
    aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN},
    hmac,
    rand::{SecureRandom, SystemRandom},
};

pub(super) struct SealedBytes(LessSafeKey);

impl SealedBytes {
    pub fn new(root: &[u8; 32], domain: &[u8]) -> Self {
        let root_key = hmac::Key::new(hmac::HMAC_SHA256, root);
        let encryption = hmac::sign(&root_key, domain);
        Self(LessSafeKey::new(
            UnboundKey::new(&AES_256_GCM, encryption.as_ref())
                .expect("SHA-256 gives a 32-byte key"),
        ))
    }

    pub fn seal(&self, binding: &str, mut bytes: Vec<u8>) -> Option<Vec<u8>> {
        let mut nonce = [0u8; NONCE_LEN];
        SystemRandom::new().fill(&mut nonce).ok()?;
        self.0
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(binding.as_bytes()),
                &mut bytes,
            )
            .ok()?;
        let mut encrypted = nonce.to_vec();
        encrypted.extend_from_slice(&bytes);
        Some(encrypted)
    }

    pub fn open(&self, binding: &str, bytes: &[u8]) -> Option<Vec<u8>> {
        let nonce = Nonce::try_assume_unique_for_key(bytes.get(..NONCE_LEN)?).ok()?;
        let mut body = bytes.get(NONCE_LEN..)?.to_vec();
        let length = self
            .0
            .open_in_place(nonce, Aad::from(binding.as_bytes()), &mut body)
            .ok()?
            .len();
        body.truncate(length);
        Some(body)
    }
}

use argon2::Argon2;
use chacha20poly1305::{
    AeadCore, ChaCha20Poly1305, KeyInit,
    aead::{Aead, OsRng},
};
use rand::RngCore;

use crate::CloakError;

const MAGIC: &[u8; 4] = b"CLOK";
const VERSION: u8 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

/// Header size: magic(4) + version(1) + salt(16) + nonce(12) = 33
const HEADER_LEN: usize = MAGIC.len() + 1 + SALT_LEN + NONCE_LEN;

/// Derive a 256-bit key from a passphrase using Argon2id.
fn derive_key(passphrase: &[u8], salt: &[u8]) -> Result<[u8; KEY_LEN], CloakError> {
    let mut key = [0u8; KEY_LEN];
    Argon2::default()
        .hash_password_into(passphrase, salt, &mut key)
        .map_err(|e| CloakError::CorruptedData(format!("key derivation failed: {e}")))?;
    Ok(key)
}

/// Encrypt plaintext with a passphrase.
///
/// Wire format: `[CLOK][version:1][salt:16][nonce:12][ciphertext+tag]`
pub fn encrypt(plaintext: &[u8], passphrase: &str) -> Result<Vec<u8>, CloakError> {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);

    let key = derive_key(passphrase.as_bytes(), &salt)?;
    let cipher = ChaCha20Poly1305::new_from_slice(&key)
        .map_err(|e| CloakError::CorruptedData(format!("cipher init failed: {e}")))?;

    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| CloakError::CorruptedData(format!("encryption failed: {e}")))?;

    let mut output = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    output.extend_from_slice(MAGIC);
    output.push(VERSION);
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&ciphertext);

    Ok(output)
}

/// Decrypt ciphertext that was encrypted with [`encrypt`].
pub fn decrypt(data: &[u8], passphrase: &str) -> Result<Vec<u8>, CloakError> {
    if data.len() < HEADER_LEN {
        return Err(CloakError::CorruptedData(
            "data too short for header".into(),
        ));
    }

    if &data[..4] != MAGIC {
        return Err(CloakError::CorruptedData("invalid magic bytes".into()));
    }

    let version = data[4];
    if version != VERSION {
        return Err(CloakError::CorruptedData(format!(
            "unsupported version: {version}"
        )));
    }

    let salt = &data[5..5 + SALT_LEN];
    let nonce_bytes = &data[5 + SALT_LEN..HEADER_LEN];
    let ciphertext = &data[HEADER_LEN..];

    let key = derive_key(passphrase.as_bytes(), salt)?;
    let cipher = ChaCha20Poly1305::new_from_slice(&key)
        .map_err(|e| CloakError::CorruptedData(format!("cipher init failed: {e}")))?;

    let nonce = chacha20poly1305::Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CloakError::InvalidPassphrase)
}

/// Returns the overhead bytes added by encryption (header + AEAD tag).
pub const fn overhead() -> usize {
    HEADER_LEN + 16 // 16 = Poly1305 tag
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let plaintext = b"steganography is cool";
        let passphrase = "hunter2";

        let encrypted = encrypt(plaintext, passphrase).unwrap();
        let decrypted = decrypt(&encrypted, passphrase).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn roundtrip_empty() {
        let encrypted = encrypt(b"", "pass").unwrap();
        let decrypted = decrypt(&encrypted, "pass").unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn wrong_passphrase() {
        let encrypted = encrypt(b"secret", "correct").unwrap();
        let result = decrypt(&encrypted, "wrong");
        assert!(matches!(result, Err(CloakError::InvalidPassphrase)));
    }

    #[test]
    fn corrupted_magic() {
        let mut encrypted = encrypt(b"data", "pass").unwrap();
        encrypted[0] = b'X';
        let result = decrypt(&encrypted, "pass");
        assert!(matches!(result, Err(CloakError::CorruptedData(_))));
    }

    #[test]
    fn truncated_data() {
        let result = decrypt(&[0u8; 10], "pass");
        assert!(matches!(result, Err(CloakError::CorruptedData(_))));
    }

    #[test]
    fn corrupted_ciphertext() {
        let mut encrypted = encrypt(b"data", "pass").unwrap();
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0xFF;
        let result = decrypt(&encrypted, "pass");
        assert!(matches!(result, Err(CloakError::InvalidPassphrase)));
    }

    #[test]
    fn different_encryptions_differ() {
        let a = encrypt(b"same", "pass").unwrap();
        let b = encrypt(b"same", "pass").unwrap();
        assert_ne!(a, b); // different salt + nonce
    }

    #[test]
    fn overhead_is_correct() {
        let plaintext = b"test";
        let encrypted = encrypt(plaintext, "pass").unwrap();
        assert_eq!(encrypted.len(), plaintext.len() + overhead());
    }
}

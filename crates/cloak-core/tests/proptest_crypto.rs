use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn encrypt_decrypt_roundtrip(
        passphrase in "[a-zA-Z0-9]{1,100}",
        payload in prop::collection::vec(any::<u8>(), 0..1024),
    ) {
        let encrypted = cloak_core::crypto::encrypt(&payload, &passphrase).unwrap();
        let decrypted = cloak_core::crypto::decrypt(&encrypted, &passphrase).unwrap();
        prop_assert_eq!(decrypted, payload);
    }

    #[test]
    fn random_bytes_decrypt_never_panics(
        data in prop::collection::vec(any::<u8>(), 0..512),
    ) {
        // Should return Err, never panic
        let _ = cloak_core::crypto::decrypt(&data, "test-pass");
    }

    #[test]
    fn corrupted_ciphertext_errors(
        payload in prop::collection::vec(any::<u8>(), 1..256),
        corrupt_offset in 0usize..100,
    ) {
        let mut encrypted = cloak_core::crypto::encrypt(&payload, "pass").unwrap();
        if encrypted.is_empty() { return Ok(()); }
        let idx = corrupt_offset % encrypted.len();
        encrypted[idx] ^= 0xFF;
        let result = cloak_core::crypto::decrypt(&encrypted, "pass");
        prop_assert!(result.is_err());
    }

    #[test]
    fn truncated_ciphertext_errors(
        payload in prop::collection::vec(any::<u8>(), 1..256),
        truncate_to in 0usize..100,
    ) {
        let encrypted = cloak_core::crypto::encrypt(&payload, "pass").unwrap();
        // Only test actual truncation (shorter than full ciphertext)
        if truncate_to >= encrypted.len() { return Ok(()); }
        let truncated = &encrypted[..truncate_to];
        let result = cloak_core::crypto::decrypt(truncated, "pass");
        prop_assert!(result.is_err());
    }
}

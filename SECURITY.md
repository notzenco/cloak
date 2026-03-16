# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in cloak, please report it responsibly.

**Do not open a public issue.**

Email: **64717583+notzenco@users.noreply.github.com**

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact

You should receive a response within 72 hours. We will work with you to understand and address the issue before any public disclosure.

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.4.x   | Yes       |
| 0.3.x   | No        |
| 0.2.x   | No        |
| 0.1.x   | No        |

## Security Architecture

- **Mandatory encryption** — All embedded payloads are encrypted with ChaCha20-Poly1305 (AEAD). There is no unencrypted embedding path.
- **Key derivation** — Argon2id with 16-byte random salts. Separate salts for encryption, pixel permutation, and length masking prevent cross-purpose key reuse.
- **Authenticated encryption** — Poly1305 tags detect tampering and wrong passphrases.
- **No plaintext leaks** — The 4-byte payload length header is XOR-masked with a passphrase-derived key. Payload is padded with random bytes to 64-byte block boundaries.
- **Versioned wire format** — Encrypted data carries a version byte. Unknown versions are rejected with `UnsupportedVersion`, enabling forward-compatible format evolution without silent data corruption.
- **Error handling** — All cryptographic and embedding operations return `Result` types. No `panic!`, `unwrap()`, or `expect()` in library code paths.
- **Fuzz testing** — Six cargo-fuzz harnesses cover all untrusted input paths (extract, embed, analyze, decrypt, format detection, wire format parsing).

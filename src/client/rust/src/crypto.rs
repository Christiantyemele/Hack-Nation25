//! Cryptography module for the MCP client
//!
//! This module handles encryption and decryption of data using libsodium.
//! It implements the XChaCha20-Poly1305 and X25519 algorithms for secure
//! communication with the LogNarrator cloud.

use anyhow::{Context, Result};
use sodium_oxide::crypto::box_;
use sodium_oxide::crypto::secretbox;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Initialize the sodium library
pub fn init() -> Result<()> {
    if !sodium_oxide::init() {
        anyhow::bail!("Failed to initialize sodium library");
    }
    Ok(())
}

/// Keypair for asymmetric encryption
#[derive(Debug, Clone)]
pub struct KeyPair {
    pub public_key: box_::PublicKey,
    pub secret_key: box_::SecretKey,
}

/// Load a keypair from files
pub fn load_keypair<P: AsRef<Path>>(private_key_path: P) -> Result<KeyPair> {
    // Read the private key file
    let mut file = File::open(&private_key_path)
        .context("Failed to open private key file")?;

    let mut key_data = Vec::new();
    file.read_to_end(&mut key_data)
        .context("Failed to read private key file")?;

    // Parse the secret key
    let secret_key = box_::SecretKey::from_slice(&key_data)
        .context("Invalid private key format")?;

    // Derive the public key from the secret key
    let public_key = box_::PublicKey::from_slice(&box_::keypair_from_secretkey(&secret_key).0)
        .context("Failed to derive public key")?;

    Ok(KeyPair { public_key, secret_key })
}

/// Encrypt data with the recipient's public key
pub fn encrypt(data: &[u8], recipient_pk: &box_::PublicKey, sender_sk: &box_::SecretKey) -> Result<Vec<u8>> {
    // Generate a random nonce
    let nonce = box_::gen_nonce();

    // Encrypt the data
    let ciphertext = box_::seal(data, &nonce, recipient_pk, sender_sk);

    // Combine nonce and ciphertext
    let mut result = Vec::with_capacity(nonce.as_ref().len() + ciphertext.len());
    result.extend_from_slice(nonce.as_ref());
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// Decrypt data with the recipient's secret key
pub fn decrypt(data: &[u8], sender_pk: &box_::PublicKey, recipient_sk: &box_::SecretKey) -> Result<Vec<u8>> {
    // Split nonce and ciphertext
    if data.len() < box_::NONCEBYTES {
        anyhow::bail!("Data too short to contain nonce");
    }

    let nonce = box_::Nonce::from_slice(&data[..box_::NONCEBYTES])
        .context("Invalid nonce")?;

    let ciphertext = &data[box_::NONCEBYTES..];

    // Decrypt the data
    let plaintext = box_::open(ciphertext, &nonce, sender_pk, recipient_sk)
        .map_err(|_| anyhow::anyhow!("Decryption failed"))?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() -> Result<()> {
        // Initialize sodium
        init()?;

        // Generate keypairs
        let sender = box_::gen_keypair();
        let recipient = box_::gen_keypair();

        // Test data
        let data = b"This is a test message";

        // Encrypt
        let encrypted = encrypt(data, &recipient.0, &sender.1)?;

        // Decrypt
        let decrypted = decrypt(&encrypted, &sender.0, &recipient.1)?;

        assert_eq!(decrypted, data);

        Ok(())
    }
}

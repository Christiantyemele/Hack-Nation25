//! Cryptography module for the MCP client
//!
//! This module handles encryption and decryption of data using libsodium.
//! It implements the XChaCha20-Poly1305 and X25519 algorithms for secure
//! communication with the LogNarrator cloud.

use anyhow::{Context, Result};
use sodiumoxide::crypto::box_;
use sodiumoxide::crypto::secretbox;
use sodiumoxide::crypto::sign;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

/// Initialize the sodium library
pub fn init() -> Result<()> {
    sodiumoxide::init().map_err(|_| anyhow::anyhow!("Failed to initialize sodium library"))?;
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

    // Generate a new keypair and use the provided secret key
    // Note: In a real implementation, you'd need both keys stored separately
    let (public_key, _) = box_::gen_keypair();

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

/// Generate a new signing key pair
pub fn generate_keypair() -> (sign::PublicKey, sign::SecretKey) {
    sign::gen_keypair()
}

/// Sign data with a secret key
pub fn sign(data: &[u8], secret_key: &sign::SecretKey) -> Vec<u8> {
    sign::sign(data, secret_key)
}

/// Verify a signature
pub fn verify(signed_data: &[u8], public_key: &sign::PublicKey) -> Option<Vec<u8>> {
    sign::verify(signed_data, public_key).ok()
}

/// Read a secret key from a file
pub fn read_secret_key<P: AsRef<Path>>(path: P) -> Result<sign::SecretKey> {
    let key_data = fs::read(path)?;
    sign::SecretKey::from_slice(&key_data).ok_or_else(|| anyhow::anyhow!("Invalid secret key"))
}

/// Read a public key from a file
pub fn read_public_key<P: AsRef<Path>>(path: P) -> Result<sign::PublicKey> {
    let key_data = fs::read(path)?;
    sign::PublicKey::from_slice(&key_data).ok_or_else(|| anyhow::anyhow!("Invalid public key"))
}

/// Write a secret key to a file
pub fn write_secret_key<P: AsRef<Path>>(path: P, key: &sign::SecretKey) -> Result<()> {
    fs::write(path, key.as_ref())?;
    Ok(())
}

/// Write a public key to a file
pub fn write_public_key<P: AsRef<Path>>(path: P, key: &sign::PublicKey) -> Result<()> {
    fs::write(path, key.as_ref())?;
    Ok(())
}

/// Compute SHA-256 hash of data
pub fn hash_sha256(data: &str) -> String {
    use sodiumoxide::crypto::hash;
    let hash = hash::hash(data.as_bytes());
    hex::encode(hash)
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

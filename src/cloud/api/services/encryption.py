"""Encryption and decryption services for LogNarrator API."""

import base64
import json
import logging
from typing import Dict, Optional

import nacl.secret
import nacl.signing
import nacl.encoding
from sqlalchemy.ext.asyncio import AsyncSession

from api.config import get_settings
from api.models.encryption import EncryptedData

logger = logging.getLogger(__name__)


class EncryptionError(Exception):
    """Encryption/decryption error."""
    pass


class ClientKeyStore:
    """Simple in-memory client key store.
    
    In production, this would be backed by a database or key management service.
    """
    
    def __init__(self):
        self._keys: Dict[str, Dict[str, bytes]] = {}
    
    def add_client_key(self, client_id: str, public_key: bytes, private_key: Optional[bytes] = None):
        """Add a client's public key (and optionally private key for testing)."""
        self._keys[client_id] = {
            "public_key": public_key,
            "private_key": private_key
        }
    
    def get_client_public_key(self, client_id: str) -> Optional[bytes]:
        """Get a client's public key."""
        client_keys = self._keys.get(client_id)
        return client_keys.get("public_key") if client_keys else None
    
    def get_client_private_key(self, client_id: str) -> Optional[bytes]:
        """Get a client's private key (for testing only)."""
        client_keys = self._keys.get(client_id)
        return client_keys.get("private_key") if client_keys else None


# Global key store instance
_key_store = ClientKeyStore()


def get_key_store() -> ClientKeyStore:
    """Get the global key store instance."""
    return _key_store


async def decrypt_data(encrypted_data: EncryptedData, db: AsyncSession) -> str:
    """Decrypt encrypted log data from a client.
    
    Args:
        encrypted_data: The encrypted data payload from the client
        db: Database session (for future key lookup)
        
    Returns:
        Decrypted JSON string
        
    Raises:
        EncryptionError: If decryption fails
    """
    try:
        logger.debug(f"Decrypting data from client {encrypted_data.client_id}")
        
        # Get client's public key for verification
        key_store = get_key_store()
        client_public_key = key_store.get_client_public_key(encrypted_data.client_id)
        
        if not client_public_key:
            # For demo purposes, create a test key if none exists
            logger.warning(f"No key found for client {encrypted_data.client_id}, using demo key")
            demo_key = nacl.signing.SigningKey.generate()
            key_store.add_client_key(
                encrypted_data.client_id,
                demo_key.verify_key.encode(),
                demo_key.encode()
            )
            client_public_key = demo_key.verify_key.encode()
        
        # Decode the base64 encrypted data
        encrypted_bytes = base64.b64decode(encrypted_data.data)
        
        # Create verify key from client's public key
        verify_key = nacl.signing.VerifyKey(client_public_key)
        
        # Verify and decrypt the signed data
        # The Rust client uses nacl::sign which creates a signed message
        decrypted_bytes = verify_key.verify(encrypted_bytes)
        
        # Convert bytes to string
        decrypted_json = decrypted_bytes.decode('utf-8')
        
        # Validate that it's valid JSON
        json.loads(decrypted_json)  # This will raise if invalid
        
        logger.debug(f"Successfully decrypted {len(decrypted_bytes)} bytes from client {encrypted_data.client_id}")
        return decrypted_json
        
    except Exception as e:
        logger.error(f"Failed to decrypt data from client {encrypted_data.client_id}: {e}")
        raise EncryptionError(f"Decryption failed: {str(e)}")


async def encrypt_data(data: str, client_id: str) -> EncryptedData:
    """Encrypt data for a client (for testing purposes).
    
    Args:
        data: JSON string to encrypt
        client_id: Client identifier
        
    Returns:
        EncryptedData object
        
    Raises:
        EncryptionError: If encryption fails
    """
    try:
        logger.debug(f"Encrypting data for client {client_id}")
        
        # Get or create client key
        key_store = get_key_store()
        client_private_key = key_store.get_client_private_key(client_id)
        
        if not client_private_key:
            # Create a new key for testing
            signing_key = nacl.signing.SigningKey.generate()
            key_store.add_client_key(
                client_id,
                signing_key.verify_key.encode(),
                signing_key.encode()
            )
            client_private_key = signing_key.encode()
        
        # Create signing key
        signing_key = nacl.signing.SigningKey(client_private_key)
        
        # Sign the data
        signed_data = signing_key.sign(data.encode('utf-8'))
        
        # Encode as base64
        encrypted_b64 = base64.b64encode(signed_data).decode('utf-8')
        
        return EncryptedData(
            client_id=client_id,
            timestamp=int(datetime.utcnow().timestamp() * 1000),
            version=1,
            algorithm="nacl.signing",
            nonce="",  # Not used for signing
            data=encrypted_b64,
            compressed=False
        )
        
    except Exception as e:
        logger.error(f"Failed to encrypt data for client {client_id}: {e}")
        raise EncryptionError(f"Encryption failed: {str(e)}")


def load_client_keys_from_config():
    """Load client keys from configuration.
    
    This is a stub implementation. In production, you would load keys from:
    - A secure key management service (AWS KMS, HashiCorp Vault, etc.)
    - A database with encrypted key storage
    - Environment variables (for development only)
    """
    settings = get_settings()
    
    # For demo purposes, create some test keys
    key_store = get_key_store()
    
    # Create a test client key for the e2e test client
    test_signing_key = nacl.signing.SigningKey.generate()
    key_store.add_client_key(
        "e2e-test-client-001",
        test_signing_key.verify_key.encode(),
        test_signing_key.encode()
    )
    
    # Create a fixed key for the live test client to ensure consistency
    # Using a deterministic seed for testing purposes only
    import hashlib
    seed = hashlib.sha256(b"e2e-live-test-client-seed").digest()[:32]
    live_test_signing_key = nacl.signing.SigningKey(seed)
    key_store.add_client_key(
        "e2e-live-test-client",
        live_test_signing_key.verify_key.encode(),
        live_test_signing_key.encode()
    )
    
    logger.info("Loaded client keys from configuration")


# Import datetime here to avoid circular imports
from datetime import datetime
"""Encryption-related data models."""

from pydantic import BaseModel, Field


class EncryptedData(BaseModel):
    """Encrypted log data model."""
    client_id: str = Field(..., description="Client ID for key lookup")
    timestamp: int = Field(..., description="Timestamp of encryption")
    version: int = Field(..., description="Encryption format version")
    algorithm: str = Field(..., description="Encryption algorithm")
    nonce: str = Field(..., description="Nonce for encryption (base64)")
    data: str = Field(..., description="Encrypted data (base64)")
    compressed: bool = Field(False, description="Whether data is compressed")
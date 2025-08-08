"""Application configuration."""

import os
from functools import lru_cache
from typing import List, Optional

from pydantic import PostgresDsn, validator
from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    """Application settings."""

    # API settings
    app_name: str = "LogNarrator"
    api_prefix: str = "/api/v1"
    debug: bool = False
    cors_origins: List[str] = ["*"]

    # Authentication
    secret_key: str
    jwt_algorithm: str = "HS256"
    access_token_expire_minutes: int = 30

    # Database
    database_url: PostgresDsn
    database_pool_size: int = 20
    database_max_overflow: int = 10

    # Vector database
    vector_db_url: str
    vector_db_collection: str = "log_vectors"
    vector_dimension: int = 768  # Default for most SentenceTransformers models

    # Encryption
    encryption_key_path: Optional[str] = None

    # Processing
    batch_size: int = 100
    max_queue_size: int = 10000

    # Model settings
    embedding_model: str = "all-MiniLM-L6-v2"
    language_model: Optional[str] = None
    language_model_type: str = "mistral"

    # Telemetry settings
    prometheus_port: int = 8001
    enable_prometheus: bool = True
    enable_tracing: bool = True

    @validator("secret_key", pre=True)
    def validate_secret_key(cls, v):
        """Validate that secret key is set."""
        if not v or len(v) < 32:
            raise ValueError("SECRET_KEY must be at least 32 characters long")
        return v

    class Config:
        """Pydantic config."""
        env_file = [
            ".env",  # Default local .env file
            "/tmp/lognarrator-integration-test/config/cloud/.env",  # Integration test env file
        ]
        case_sensitive = False  # Allow case-insensitive environment variable matching
        extra = "ignore"  # Ignore extra environment variables that don't match fields


@lru_cache()
def get_settings() -> Settings:
    """Get application settings with caching."""
    return Settings()

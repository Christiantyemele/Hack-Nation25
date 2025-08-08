"""Database connection handling."""

import logging
from typing import AsyncGenerator

from sqlalchemy.ext.asyncio import AsyncSession, create_async_engine
from sqlalchemy.orm import sessionmaker
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy import text

from api.config import get_settings

logger = logging.getLogger(__name__)

# Create declarative base
Base = declarative_base()

# Create async engine
settings = get_settings()

# Convert PostgresDsn to AsyncPostgresDsn
# Replace postgresql:// with postgresql+asyncpg://
async_db_url = str(settings.database_url).replace("postgresql://", "postgresql+asyncpg://")

engine = create_async_engine(
    async_db_url,
    echo=settings.debug,
    pool_size=settings.database_pool_size,
    max_overflow=settings.database_max_overflow,
)

# Create session factory
async_session_factory = sessionmaker(
    engine,
    class_=AsyncSession,
    expire_on_commit=False,
    autocommit=False,
    autoflush=False,
)


async def init_db():
    """Initialize database connection and create tables."""
    logger.info("Initializing database connection")

    # Import models to ensure they are registered with Base
    from api.models.user import UserTable
    from api.models.log import LogEntryTable
    from api.services.encryption import load_client_keys_from_config
    
    # Create all tables
    async with engine.begin() as conn:
        logger.info("Creating database tables")
        await conn.run_sync(Base.metadata.create_all)
        logger.info("Database tables created successfully")
    
    # Load client keys for encryption
    load_client_keys_from_config()
    
    # Verify connection works
    async with engine.begin() as conn:
        result = await conn.execute(text("SELECT 1"))
        logger.debug(f"Database connectivity test: {result.scalar()}")

    logger.info("Database initialization complete")


async def close_db():
    """Close database connection."""
    logger.info("Closing database connection")
    await engine.dispose()
    logger.info("Database connection closed")


async def get_db() -> AsyncGenerator[AsyncSession, None]:
    """Get database session."""
    session = async_session_factory()
    try:
        yield session
    finally:
        await session.close()

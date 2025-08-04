"""Database connection handling."""

import logging
from typing import AsyncGenerator

from sqlalchemy.ext.asyncio import AsyncSession, create_async_engine
from sqlalchemy.orm import sessionmaker

from api.config import get_settings

logger = logging.getLogger(__name__)

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
    """Initialize database connection."""
    logger.info("Initializing database connection")

    # Create a connection to verify connectivity
    async with engine.begin() as conn:
        # Just verify connection works
        await conn.execute("SELECT 1")

    logger.info("Database connection established")


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

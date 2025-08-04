"""LogNarrator Cloud API main entry point."""

import logging
import os
from contextlib import asynccontextmanager

from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse

from api.config import get_settings
from api.db import init_db, close_db
from api.routers import auth, logs, vector_db, mcp
from api.utils.telemetry import setup_telemetry

# Configure logging
logging.basicConfig(
    level=os.environ.get("LOG_LEVEL", "INFO"),
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
)
logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Startup and shutdown events for the application."""
    # Startup
    logger.info("Initializing LogNarrator API")

    # Initialize database
    await init_db()

    # Setup telemetry
    setup_telemetry()

    logger.info("LogNarrator API started")
    yield

    # Shutdown
    logger.info("Shutting down LogNarrator API")
    await close_db()
    logger.info("LogNarrator API stopped")


# Create FastAPI application
app = FastAPI(
    title="LogNarrator API",
    description="Log analysis and narrative generation API",
    version="0.1.0",
    lifespan=lifespan,
)

# Add CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=get_settings().cors_origins,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


# Global exception handler
@app.exception_handler(Exception)
async def global_exception_handler(request: Request, exc: Exception):
    """Handle all unhandled exceptions."""
    logger.error(f"Unhandled exception: {exc}", exc_info=True)
    return JSONResponse(
        status_code=500,
        content={"detail": "Internal server error"},
    )


# Health check endpoint
@app.get("/health")
async def health_check():
    """Simple health check endpoint."""
    return {"status": "healthy"}


# Include routers
app.include_router(auth.router, prefix="/api/v1")
app.include_router(logs.router, prefix="/api/v1")
app.include_router(vector_db.router, prefix="/api/v1")
app.include_router(mcp.router, prefix="/api/v1")


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(
        "api.main:app",
        host="0.0.0.0",
        port=8000,
        reload=True,
    )

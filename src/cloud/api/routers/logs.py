"""Log ingestion and processing API endpoints."""

import json
import logging
from typing import Dict, List, Optional, Any

from fastapi import APIRouter, Depends, HTTPException, Request, status
from pydantic import BaseModel, Field
from sqlalchemy.ext.asyncio import AsyncSession

from api.db import get_db
from api.models.user import User
from api.routers.auth import get_current_active_user
from api.services import encryption, log_processor

logger = logging.getLogger(__name__)

router = APIRouter(tags=["logs"])


class LogRecord(BaseModel):
    """Log record model."""
    timestamp: int = Field(..., description="Timestamp in milliseconds")
    severity: str = Field(..., description="Log severity level")
    body: str = Field(..., description="Log message")
    attributes: Optional[Dict[str, str]] = Field(None, description="Log attributes")
    resource: Optional[Dict[str, str]] = Field(None, description="Resource attributes")
    trace_id: Optional[str] = Field(None, description="Trace ID")
    span_id: Optional[str] = Field(None, description="Span ID")
    severity_num: Optional[int] = Field(None, description="Numeric severity level")


class LogBatch(BaseModel):
    """Batch of log records."""
    records: List[LogRecord] = Field(..., description="List of log records")


class EncryptedData(BaseModel):
    """Encrypted log data model."""
    client_id: str = Field(..., description="Client ID for key lookup")
    timestamp: int = Field(..., description="Timestamp of encryption")
    version: int = Field(..., description="Encryption format version")
    algorithm: str = Field(..., description="Encryption algorithm")
    nonce: str = Field(..., description="Nonce for encryption (base64)")
    data: str = Field(..., description="Encrypted data (base64)")
    compressed: bool = Field(False, description="Whether data is compressed")


@router.post("/logs")
async def ingest_logs(request: Request, db: AsyncSession = Depends(get_db)):
    """Ingest logs from clients.

    This endpoint handles both encrypted and plaintext logs.
    """
    content_type = request.headers.get("Content-Type", "application/json")

    # Read the raw body
    body = await request.body()

    try:
        if content_type == "application/json+encrypted":
            # Parse as encrypted data
            encrypted_data = EncryptedData.parse_raw(body)

            # Decrypt the data
            decrypted_data = await encryption.decrypt_data(encrypted_data, db)

            # Parse the decrypted JSON
            log_batch = LogBatch.parse_raw(decrypted_data)
        else:
            # Parse as plaintext JSON
            log_batch = LogBatch.parse_raw(body)

        # Process the logs
        await log_processor.process_logs(log_batch, db)

        return {"status": "success", "processed": len(log_batch.records)}

    except Exception as e:
        logger.error(f"Error processing logs: {e}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"Invalid log data: {str(e)}",
        )


@router.get("/logs/search")
async def search_logs(
    query: str,
    start_time: Optional[int] = None,
    end_time: Optional[int] = None,
    limit: int = 100,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db),
):
    """Search logs by text query and time range."""
    try:
        # This is a stub - you'd implement actual log search here
        # For now, return a sample response
        return {
            "status": "success",
            "count": 1,
            "results": [
                {
                    "timestamp": 1625097600000,
                    "severity": "ERROR",
                    "body": "Connection refused to database",
                    "attributes": {"service": "api", "instance": "api-1"},
                    "resource": {"host": "server-01", "cluster": "prod"},
                }
            ],
        }

    except Exception as e:
        logger.error(f"Error searching logs: {e}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Error searching logs: {str(e)}",
        )

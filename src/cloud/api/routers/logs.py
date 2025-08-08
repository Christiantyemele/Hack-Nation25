"""Log ingestion and processing API endpoints."""

import json
import logging
from typing import Dict, List, Optional, Any

from fastapi import APIRouter, Depends, HTTPException, Request, status
from pydantic import BaseModel, Field
from sqlalchemy.ext.asyncio import AsyncSession

from api.db import get_db
from api.models.user import User
from api.models.encryption import EncryptedData
from api.models.logs import LogBatch, LogRecord
from api.models.log import LogSearchQuery, LogSearchResponse
from api.routers.auth import get_current_active_user
from api.services import encryption, log_processor

logger = logging.getLogger(__name__)

router = APIRouter(tags=["logs"])


@router.post("/logs")
async def ingest_logs(request: Request, db: AsyncSession = Depends(get_db)):
    """Ingest logs from clients.

    This endpoint handles both encrypted and plaintext logs.
    """
    content_type = request.headers.get("Content-Type", "application/json")

    # Read the raw body
    body = await request.body()

    try:
        client_id = "unknown"  # Default client ID
        
        if content_type == "application/json+encrypted":
            # Parse as encrypted data
            encrypted_data = EncryptedData.parse_raw(body)
            client_id = encrypted_data.client_id  # Extract client ID from encrypted data

            # Decrypt the data
            decrypted_data = await encryption.decrypt_data(encrypted_data, db)

            # Parse the decrypted JSON
            log_batch = LogBatch.parse_raw(decrypted_data)
        else:
            # Parse as plaintext JSON
            log_batch = LogBatch.parse_raw(body)

        # Process the logs with client ID
        await log_processor.process_logs(log_batch, db, client_id)

        return {"status": "success", "processed": len(log_batch.records)}

    except Exception as e:
        logger.error(f"Error processing logs: {e}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"Invalid log data: {str(e)}",
        )


@router.get("/logs/search", response_model=LogSearchResponse)
async def search_logs(
    query: Optional[str] = None,
    client_id: Optional[str] = None,
    severity: Optional[str] = None,
    start_time: Optional[int] = None,
    end_time: Optional[int] = None,
    trace_id: Optional[str] = None,
    limit: int = 100,
    offset: int = 0,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db),
):
    """Search logs by various criteria."""
    try:
        # Convert timestamps from milliseconds to datetime if provided
        start_datetime = None
        end_datetime = None
        
        if start_time:
            from datetime import datetime
            start_datetime = datetime.fromtimestamp(start_time / 1000.0)
        
        if end_time:
            from datetime import datetime
            end_datetime = datetime.fromtimestamp(end_time / 1000.0)
        
        # Create search query
        search_query = LogSearchQuery(
            query=query,
            client_id=client_id,
            severity=severity,
            start_time=start_datetime,
            end_time=end_datetime,
            trace_id=trace_id,
            limit=min(limit, 1000),  # Cap at 1000
            offset=max(offset, 0)    # Ensure non-negative
        )
        
        # Execute search using log processor
        search_result = await log_processor.search_logs(search_query, db)
        
        return search_result

    except Exception as e:
        logger.error(f"Error searching logs: {e}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Error searching logs: {str(e)}",
        )

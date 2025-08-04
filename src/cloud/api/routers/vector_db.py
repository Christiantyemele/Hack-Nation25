"""Vector database API endpoints."""

import logging
from typing import Dict, List, Optional, Any

from fastapi import APIRouter, Depends, HTTPException, status
from pydantic import BaseModel, Field
from sqlalchemy.ext.asyncio import AsyncSession

from api.db import get_db
from api.models.user import User
from api.routers.auth import get_current_active_user
from api.services import vector_store

logger = logging.getLogger(__name__)

router = APIRouter(tags=["vector-db"])


class VectorQuery(BaseModel):
    """Vector similarity search query."""
    text: str = Field(..., description="Text to search for similar logs")
    limit: int = Field(10, description="Maximum number of results to return")
    filters: Optional[Dict[str, Any]] = Field(None, description="Metadata filters")
    time_range: Optional[Dict[str, int]] = Field(None, description="Time range in milliseconds")


class VectorSearchResult(BaseModel):
    """Vector search result."""
    id: str = Field(..., description="Result ID")
    score: float = Field(..., description="Similarity score")
    payload: Dict[str, Any] = Field(..., description="Result payload")


class VectorSearchResponse(BaseModel):
    """Vector search response."""
    results: List[VectorSearchResult] = Field(..., description="Search results")
    count: int = Field(..., description="Total number of results")


@router.post("/vector/search", response_model=VectorSearchResponse)
async def search_vectors(
    query: VectorQuery,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db),
):
    """Search for similar logs using vector similarity."""
    try:
        results = await vector_store.search(
            query.text,
            limit=query.limit,
            filters=query.filters,
            time_range=query.time_range,
        )

        return VectorSearchResponse(
            results=[
                VectorSearchResult(
                    id=result["id"],
                    score=result["score"],
                    payload=result["payload"],
                )
                for result in results
            ],
            count=len(results),
        )

    except Exception as e:
        logger.error(f"Error searching vectors: {e}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Error searching vectors: {str(e)}",
        )


@router.post("/vector/context")
async def get_context(
    log_id: str,
    window_size: int = 10,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db),
):
    """Get temporal context around a specific log entry."""
    try:
        # This is a stub - you'd implement actual context retrieval here
        # For now, return sample data
        return {
            "status": "success",
            "context": {
                "before": [
                    {"timestamp": 1625097590000, "body": "Starting database connection"},
                    {"timestamp": 1625097595000, "body": "Database connection attempt 1 failed"},
                ],
                "target": {"timestamp": 1625097600000, "body": "Connection refused to database"},
                "after": [
                    {"timestamp": 1625097605000, "body": "Retrying database connection"},
                    {"timestamp": 1625097610000, "body": "Database connection successful"},
                ],
            },
        }

    except Exception as e:
        logger.error(f"Error getting context: {e}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Error getting context: {str(e)}",
        )

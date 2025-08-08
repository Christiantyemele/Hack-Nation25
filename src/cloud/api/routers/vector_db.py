"""Vector database API endpoints."""

import logging
from typing import Dict, List, Optional, Any

from fastapi import APIRouter, Depends, HTTPException, status
from pydantic import BaseModel, Field
from sqlalchemy.ext.asyncio import AsyncSession

from api.db import get_db
from api.models.user import User
from api.routers.auth import get_current_active_user

# Conditional import for vector store
try:
    from api.services import vector_store
    VECTOR_STORE_AVAILABLE = True
except ImportError:
    vector_store = None
    VECTOR_STORE_AVAILABLE = False

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
    if not VECTOR_STORE_AVAILABLE:
        raise HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail="Vector store service is not available. Please install required dependencies: pip install sentence-transformers qdrant-client",
        )
    
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
    if not VECTOR_STORE_AVAILABLE:
        raise HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail="Vector store service is not available. Please install required dependencies: pip install sentence-transformers qdrant-client",
        )
    
    try:
        context = await vector_store.get_temporal_context(log_id, window_size)
        
        return {
            "status": "success",
            "context": context,
        }

    except Exception as e:
        logger.error(f"Error getting context: {e}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Error getting context: {str(e)}",
        )

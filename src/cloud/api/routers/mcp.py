"""Multi-Command Protocol (MCP) API endpoints."""

import logging
from enum import Enum
from typing import Dict, List, Optional, Any

from fastapi import APIRouter, Depends, HTTPException, status
from pydantic import BaseModel, Field
from sqlalchemy.ext.asyncio import AsyncSession

from api.db import get_db
from api.models.user import User
from api.models.mcp import (
    PermissionLevel, ActionRecommendation, McpMessage, McpMessageRequest,
    ActionResult, McpResponse
)
from api.routers.auth import get_current_active_user
from api.services import mcp_service

logger = logging.getLogger(__name__)

router = APIRouter(tags=["mcp"])


@router.post("/mcp/message")
async def send_mcp_message(
    request: McpMessageRequest,
    current_user: User = Depends(get_current_active_user),
    db: AsyncSession = Depends(get_db),
):
    """Send an MCP message to a client."""
    try:
        message_id = await mcp_service.send_message(
            client_id=request.client_id,
            message=request.message,
            db=db,
        )

        return {"status": "success", "message_id": message_id}

    except Exception as e:
        logger.error(f"Error sending MCP message: {e}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Error sending MCP message: {str(e)}",
        )


@router.post("/mcp/response")
async def receive_mcp_response(
    response: McpResponse,
    db: AsyncSession = Depends(get_db),
):
    """Receive an MCP response from a client."""
    try:
        await mcp_service.process_response(response, db)
        return {"status": "success"}

    except Exception as e:
        logger.error(f"Error processing MCP response: {e}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"Error processing MCP response: {str(e)}",
        )


@router.get("/mcp/pending")
async def get_pending_messages(
    client_id: str,
    db: AsyncSession = Depends(get_db),
):
    """Get pending MCP messages for a client."""
    try:
        messages = await mcp_service.get_pending_messages(client_id, db)
        return {"status": "success", "messages": messages}

    except Exception as e:
        logger.error(f"Error getting pending messages: {e}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Error getting pending messages: {str(e)}",
        )

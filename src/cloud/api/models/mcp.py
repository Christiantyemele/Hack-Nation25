"""MCP (Multi-Command Protocol) related data models."""

from enum import Enum
from typing import Dict, List, Optional, Any
from pydantic import BaseModel, Field


class PermissionLevel(str, Enum):
    """Action permission level."""
    READ_ONLY = "ReadOnly"
    STANDARD = "Standard"
    ELEVATED = "Elevated"
    HIGH_RISK = "HighRisk"


class ActionRecommendation(BaseModel):
    """Action recommendation."""
    action_id: str = Field(..., description="Action identifier")
    description: str = Field(..., description="Human-readable description")
    parameters: Dict[str, str] = Field({}, description="Action parameters")
    permission_level: PermissionLevel = Field(..., description="Required permission level")


class McpMessage(BaseModel):
    """MCP message."""
    id: str = Field(..., description="Message identifier")
    timestamp: int = Field(..., description="Timestamp in milliseconds")
    severity: str = Field(..., description="Severity level")
    narrative: str = Field(..., description="Narrative explanation")
    actions: List[ActionRecommendation] = Field(..., description="Recommended actions")


class McpMessageRequest(BaseModel):
    """Request to send an MCP message to a client."""
    client_id: str = Field(..., description="Target client ID")
    message: McpMessage = Field(..., description="Message to send")


class ActionResult(BaseModel):
    """Action execution result."""
    action_id: str = Field(..., description="Action identifier")
    status: str = Field(..., description="Execution status")
    message: str = Field(..., description="Result message")
    data: Optional[Dict[str, Any]] = Field(None, description="Additional result data")


class McpResponse(BaseModel):
    """Response from an MCP client."""
    message_id: str = Field(..., description="Original message ID")
    timestamp: int = Field(..., description="Response timestamp")
    results: List[ActionResult] = Field(..., description="Action results")
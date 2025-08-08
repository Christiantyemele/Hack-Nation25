"""Log-related data models."""

from typing import Dict, List, Optional
from pydantic import BaseModel, Field


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
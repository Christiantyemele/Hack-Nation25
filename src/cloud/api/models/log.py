"""Log entry models for storing and processing logs."""

from datetime import datetime
from typing import Dict, Optional, Any

from pydantic import BaseModel, Field
from sqlalchemy import Column, DateTime, Integer, String, Text, JSON, Index
from sqlalchemy.ext.declarative import declarative_base

# Use the same Base instance across all models
try:
    from api.db import Base
except ImportError:
    # Fallback if there's a circular import
    Base = declarative_base()


class LogEntryTable(Base):
    """Log entry database table."""
    
    __tablename__ = "log_entries"
    
    id = Column(Integer, primary_key=True, index=True)
    timestamp = Column(DateTime, nullable=False, index=True)
    client_id = Column(String(100), nullable=False, index=True)
    severity = Column(String(20), nullable=False, index=True)
    body = Column(Text, nullable=False)
    attributes = Column(JSON, nullable=True)
    resource = Column(JSON, nullable=True)
    trace_id = Column(String(32), nullable=True, index=True)
    span_id = Column(String(16), nullable=True, index=True)
    severity_num = Column(Integer, nullable=True, index=True)
    created_at = Column(DateTime, default=datetime.utcnow, nullable=False)
    
    # Create composite indexes for common queries
    __table_args__ = (
        Index('idx_timestamp_severity', 'timestamp', 'severity'),
        Index('idx_client_timestamp', 'client_id', 'timestamp'),
        Index('idx_trace_span', 'trace_id', 'span_id'),
    )


class LogEntry(BaseModel):
    """Log entry Pydantic model for API responses."""
    
    id: Optional[int] = None
    timestamp: datetime
    client_id: str
    severity: str
    body: str
    attributes: Optional[Dict[str, Any]] = None
    resource: Optional[Dict[str, Any]] = None
    trace_id: Optional[str] = None
    span_id: Optional[str] = None
    severity_num: Optional[int] = None
    created_at: Optional[datetime] = None
    
    class Config:
        """Pydantic configuration."""
        from_attributes = True


class LogEntryCreate(BaseModel):
    """Log entry creation model."""
    
    timestamp: datetime
    client_id: str
    severity: str
    body: str
    attributes: Optional[Dict[str, Any]] = None
    resource: Optional[Dict[str, Any]] = None
    trace_id: Optional[str] = None
    span_id: Optional[str] = None
    severity_num: Optional[int] = None


class LogBatchCreate(BaseModel):
    """Batch of log entries for creation."""
    
    client_id: str
    entries: list[LogEntryCreate]


class LogSearchQuery(BaseModel):
    """Log search query parameters."""
    
    query: Optional[str] = None
    client_id: Optional[str] = None
    severity: Optional[str] = None
    start_time: Optional[datetime] = None
    end_time: Optional[datetime] = None
    trace_id: Optional[str] = None
    limit: int = Field(default=100, le=1000)
    offset: int = Field(default=0, ge=0)


class LogSearchResponse(BaseModel):
    """Log search response model."""
    
    total: int
    entries: list[LogEntry]
    query: LogSearchQuery
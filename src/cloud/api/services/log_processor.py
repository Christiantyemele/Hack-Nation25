"""Log processing services for LogNarrator API."""

import json
import logging
from datetime import datetime
from typing import List, Optional

from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select, and_, or_
from sqlalchemy.orm import selectinload

from api.models.log import LogEntry, LogEntryTable, LogEntryCreate, LogBatchCreate, LogSearchQuery, LogSearchResponse
from api.models.logs import LogBatch, LogRecord
try:
    from api.services.vector_store import process_logs_for_embedding
    VECTOR_STORE_AVAILABLE = True
except ImportError:
    VECTOR_STORE_AVAILABLE = False
    def process_logs_for_embedding(*args, **kwargs):
        """Stub function when vector store is not available."""
        return []

logger = logging.getLogger(__name__)


class LogProcessingError(Exception):
    """Log processing error."""
    pass


async def process_logs(log_batch: LogBatch, db: AsyncSession, client_id: str = "unknown") -> int:
    """Process a batch of logs and store them in the database.
    
    Args:
        log_batch: Batch of log records to process
        db: Database session
        
    Returns:
        Number of logs processed
        
    Raises:
        LogProcessingError: If processing fails
    """
    try:
        logger.debug(f"Processing batch of {len(log_batch.records)} logs")
        
        processed_count = 0
        log_entries = []
        
        for record in log_batch.records:
            try:
                # Convert LogRecord to LogEntryTable
                log_entry = await _convert_log_record_to_entry(record, client_id)
                log_entries.append(log_entry)
                processed_count += 1
                
            except Exception as e:
                logger.error(f"Failed to process log record: {e}")
                # Continue processing other records
                continue
        
        if log_entries:
            # Bulk insert the log entries
            db.add_all(log_entries)
            await db.commit()
            
            logger.info(f"Successfully processed and stored {processed_count} logs")
            
            # Process logs for vector embedding (Phase 3B) - optional
            if VECTOR_STORE_AVAILABLE:
                try:
                    # Convert log entries to dictionaries for embedding processing
                    log_dicts = []
                    for entry in log_entries:
                        log_dict = {
                            "timestamp": int(entry.timestamp.timestamp() * 1000),  # Convert to milliseconds
                            "severity": entry.severity,
                            "body": entry.body,
                            "client_id": entry.client_id,
                            "trace_id": entry.trace_id,
                            "span_id": entry.span_id,
                            "attributes": entry.attributes or {},
                            "resource": entry.resource or {}
                        }
                        log_dicts.append(log_dict)
                    
                    # Process for embedding asynchronously
                    vector_ids = await process_logs_for_embedding(log_dicts)
                    logger.info(f"Successfully created embeddings for {len(vector_ids)} logs")
                    
                except Exception as e:
                    # Don't fail the entire log processing if embedding fails
                    logger.error(f"Failed to process logs for embedding: {e}")
            else:
                logger.debug("Vector store not available, skipping embedding processing")
                
        else:
            logger.warning("No valid log entries to store")
        
        return processed_count
        
    except Exception as e:
        logger.error(f"Failed to process log batch: {e}")
        await db.rollback()
        raise LogProcessingError(f"Log processing failed: {str(e)}")


async def _convert_log_record_to_entry(record: LogRecord, client_id: str = "unknown") -> LogEntryTable:
    """Convert a LogRecord to a LogEntryTable for database storage.
    
    Args:
        record: The log record to convert
        client_id: Client identifier (extracted from context)
        
    Returns:
        LogEntryTable instance ready for database insertion
    """
    # Convert timestamp from milliseconds to datetime
    if isinstance(record.timestamp, int):
        timestamp = datetime.fromtimestamp(record.timestamp / 1000.0)
    else:
        timestamp = datetime.utcnow()
    
    # Create the database entry
    return LogEntryTable(
        timestamp=timestamp,
        client_id=client_id,
        severity=record.severity,
        body=record.body,
        attributes=record.attributes,
        resource=record.resource,
        trace_id=record.trace_id,
        span_id=record.span_id,
        severity_num=record.severity_num,
        created_at=datetime.utcnow()
    )


async def search_logs(query: LogSearchQuery, db: AsyncSession) -> LogSearchResponse:
    """Search logs based on query parameters.
    
    Args:
        query: Search query parameters
        db: Database session
        
    Returns:
        Search results with matching logs
    """
    try:
        logger.debug(f"Searching logs with query: {query}")
        
        # Build the base query
        stmt = select(LogEntryTable)
        conditions = []
        
        # Add filters based on query parameters
        if query.client_id:
            conditions.append(LogEntryTable.client_id == query.client_id)
        
        if query.severity:
            conditions.append(LogEntryTable.severity == query.severity)
        
        if query.start_time:
            conditions.append(LogEntryTable.timestamp >= query.start_time)
        
        if query.end_time:
            conditions.append(LogEntryTable.timestamp <= query.end_time)
        
        if query.trace_id:
            conditions.append(LogEntryTable.trace_id == query.trace_id)
        
        if query.query:
            # Simple text search in the body field
            # In production, you might want to use full-text search
            conditions.append(LogEntryTable.body.ilike(f"%{query.query}%"))
        
        # Apply all conditions
        if conditions:
            stmt = stmt.where(and_(*conditions))
        
        # Add ordering (most recent first)
        stmt = stmt.order_by(LogEntryTable.timestamp.desc())
        
        # Get total count for pagination
        count_stmt = select(LogEntryTable).where(and_(*conditions)) if conditions else select(LogEntryTable)
        total_result = await db.execute(count_stmt)
        total = len(total_result.fetchall())
        
        # Apply pagination
        stmt = stmt.offset(query.offset).limit(query.limit)
        
        # Execute the query
        result = await db.execute(stmt)
        log_entries = result.scalars().all()
        
        # Convert to Pydantic models
        entries = [
            LogEntry(
                id=entry.id,
                timestamp=entry.timestamp,
                client_id=entry.client_id,
                severity=entry.severity,
                body=entry.body,
                attributes=entry.attributes,
                resource=entry.resource,
                trace_id=entry.trace_id,
                span_id=entry.span_id,
                severity_num=entry.severity_num,
                created_at=entry.created_at
            )
            for entry in log_entries
        ]
        
        logger.debug(f"Found {len(entries)} logs matching query")
        
        return LogSearchResponse(
            total=total,
            entries=entries,
            query=query
        )
        
    except Exception as e:
        logger.error(f"Failed to search logs: {e}")
        raise LogProcessingError(f"Log search failed: {str(e)}")


async def get_log_by_id(log_id: int, db: AsyncSession) -> Optional[LogEntry]:
    """Get a specific log entry by ID.
    
    Args:
        log_id: The log entry ID
        db: Database session
        
    Returns:
        LogEntry if found, None otherwise
    """
    try:
        stmt = select(LogEntryTable).where(LogEntryTable.id == log_id)
        result = await db.execute(stmt)
        entry = result.scalar_one_or_none()
        
        if entry:
            return LogEntry(
                id=entry.id,
                timestamp=entry.timestamp,
                client_id=entry.client_id,
                severity=entry.severity,
                body=entry.body,
                attributes=entry.attributes,
                resource=entry.resource,
                trace_id=entry.trace_id,
                span_id=entry.span_id,
                severity_num=entry.severity_num,
                created_at=entry.created_at
            )
        
        return None
        
    except Exception as e:
        logger.error(f"Failed to get log by ID {log_id}: {e}")
        raise LogProcessingError(f"Failed to get log: {str(e)}")


async def get_logs_by_trace_id(trace_id: str, db: AsyncSession) -> List[LogEntry]:
    """Get all log entries for a specific trace ID.
    
    Args:
        trace_id: The trace ID to search for
        db: Database session
        
    Returns:
        List of LogEntry objects
    """
    try:
        stmt = select(LogEntryTable).where(
            LogEntryTable.trace_id == trace_id
        ).order_by(LogEntryTable.timestamp.asc())
        
        result = await db.execute(stmt)
        entries = result.scalars().all()
        
        return [
            LogEntry(
                id=entry.id,
                timestamp=entry.timestamp,
                client_id=entry.client_id,
                severity=entry.severity,
                body=entry.body,
                attributes=entry.attributes,
                resource=entry.resource,
                trace_id=entry.trace_id,
                span_id=entry.span_id,
                severity_num=entry.severity_num,
                created_at=entry.created_at
            )
            for entry in entries
        ]
        
    except Exception as e:
        logger.error(f"Failed to get logs by trace ID {trace_id}: {e}")
        raise LogProcessingError(f"Failed to get logs by trace: {str(e)}")


async def delete_old_logs(days_to_keep: int, db: AsyncSession) -> int:
    """Delete log entries older than the specified number of days.
    
    Args:
        days_to_keep: Number of days to keep logs
        db: Database session
        
    Returns:
        Number of logs deleted
    """
    try:
        cutoff_date = datetime.utcnow() - timedelta(days=days_to_keep)
        
        stmt = select(LogEntryTable).where(LogEntryTable.timestamp < cutoff_date)
        result = await db.execute(stmt)
        entries_to_delete = result.scalars().all()
        
        count = len(entries_to_delete)
        
        if count > 0:
            for entry in entries_to_delete:
                await db.delete(entry)
            
            await db.commit()
            logger.info(f"Deleted {count} old log entries")
        
        return count
        
    except Exception as e:
        logger.error(f"Failed to delete old logs: {e}")
        await db.rollback()
        raise LogProcessingError(f"Failed to delete old logs: {str(e)}")


# Import timedelta here to avoid circular imports
from datetime import timedelta
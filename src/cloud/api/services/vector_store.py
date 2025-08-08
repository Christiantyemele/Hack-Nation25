"""Vector database service for similarity search."""

import logging
import uuid
from typing import Dict, List, Optional, Any
from datetime import datetime

# Conditional imports for optional dependencies
try:
    from qdrant_client import QdrantClient
    from qdrant_client.http import models
    from qdrant_client.http.models import Distance, VectorParams, PointStruct
    QDRANT_AVAILABLE = True
except ImportError:
    QDRANT_AVAILABLE = False
    QdrantClient = None
    models = None
    Distance = None
    VectorParams = None
    PointStruct = None

try:
    from sentence_transformers import SentenceTransformer
    SENTENCE_TRANSFORMERS_AVAILABLE = True
except ImportError:
    SENTENCE_TRANSFORMERS_AVAILABLE = False
    SentenceTransformer = None

from api.config import get_settings

# Check if vector store is fully available
VECTOR_STORE_AVAILABLE = QDRANT_AVAILABLE and SENTENCE_TRANSFORMERS_AVAILABLE

logger = logging.getLogger(__name__)

# Global variables for model and client (initialized lazily)
_embedding_model: Optional[SentenceTransformer] = None
_qdrant_client: Optional[QdrantClient] = None


def get_embedding_model() -> SentenceTransformer:
    """Get or initialize the embedding model."""
    if not SENTENCE_TRANSFORMERS_AVAILABLE:
        raise ImportError("sentence_transformers is not available. Install it with: pip install sentence-transformers")
    
    global _embedding_model
    if _embedding_model is None:
        settings = get_settings()
        logger.info(f"Loading embedding model: {settings.embedding_model}")
        _embedding_model = SentenceTransformer(settings.embedding_model)
    return _embedding_model


def get_qdrant_client() -> QdrantClient:
    """Get or initialize the Qdrant client."""
    if not QDRANT_AVAILABLE:
        raise ImportError("qdrant_client is not available. Install it with: pip install qdrant-client")
    
    global _qdrant_client
    if _qdrant_client is None:
        settings = get_settings()
        logger.info(f"Connecting to Qdrant at: {settings.vector_db_url}")
        _qdrant_client = QdrantClient(url=settings.vector_db_url)
        
        # Ensure collection exists
        _ensure_collection_exists()
    
    return _qdrant_client


def _ensure_collection_exists():
    """Ensure the vector collection exists in Qdrant."""
    settings = get_settings()
    client = _qdrant_client
    
    try:
        # Check if collection exists
        collections = client.get_collections()
        collection_names = [col.name for col in collections.collections]
        
        if settings.vector_db_collection not in collection_names:
            logger.info(f"Creating collection: {settings.vector_db_collection}")
            client.create_collection(
                collection_name=settings.vector_db_collection,
                vectors_config=VectorParams(
                    size=settings.vector_dimension,
                    distance=Distance.COSINE
                )
            )
        else:
            logger.info(f"Collection {settings.vector_db_collection} already exists")
            
    except Exception as e:
        logger.error(f"Failed to ensure collection exists: {e}")
        raise


async def search(
    query: str,
    limit: int = 10,
    filters: Optional[Dict[str, Any]] = None,
    time_range: Optional[Dict[str, int]] = None,
) -> List[Dict[str, Any]]:
    """Search for similar logs using vector similarity.

    Args:
        query: Text to search for
        limit: Maximum number of results to return
        filters: Metadata filters to apply
        time_range: Time range in milliseconds

    Returns:
        List of search results with similarity scores
    """
    if not VECTOR_STORE_AVAILABLE:
        logger.warning("Vector store dependencies not available, returning empty results")
        return []
    
    try:
        settings = get_settings()
        model = get_embedding_model()
        client = get_qdrant_client()
        
        logger.debug(f"Vector search for: {query} with limit {limit}")
        
        # 1. Generate embedding for the query text
        query_embedding = model.encode([query])[0].tolist()
        
        # 2. Build filters if provided
        search_filter = None
        if filters or time_range:
            conditions = []
            
            if filters:
                for key, value in filters.items():
                    conditions.append(
                        models.FieldCondition(
                            key=key,
                            match=models.MatchValue(value=value)
                        )
                    )
            
            if time_range:
                if "start" in time_range and "end" in time_range:
                    conditions.append(
                        models.FieldCondition(
                            key="timestamp",
                            range=models.Range(
                                gte=time_range["start"],
                                lte=time_range["end"]
                            )
                        )
                    )
            
            if conditions:
                search_filter = models.Filter(must=conditions)
        
        # 3. Search the vector database
        search_results = client.search(
            collection_name=settings.vector_db_collection,
            query_vector=query_embedding,
            query_filter=search_filter,
            limit=limit,
            with_payload=True,
            with_vectors=False
        )
        
        # 4. Format results
        results = []
        for result in search_results:
            results.append({
                "id": str(result.id),
                "score": float(result.score),
                "payload": result.payload
            })
        
        logger.info(f"Found {len(results)} similar logs for query: {query}")
        return results
        
    except Exception as e:
        logger.error(f"Vector search failed: {e}")
        # Return empty results on error rather than failing completely
        return []


async def add_vectors(
    texts: List[str],
    metadata: List[Dict[str, Any]],
    ids: Optional[List[str]] = None,
) -> List[str]:
    """Add vectors to the vector database.

    Args:
        texts: Texts to convert to vectors and store
        metadata: Metadata for each text
        ids: Optional IDs for the vectors

    Returns:
        List of IDs for the stored vectors
    """
    if not VECTOR_STORE_AVAILABLE:
        logger.warning("Vector store dependencies not available, returning empty list")
        return []
    
    try:
        settings = get_settings()
        model = get_embedding_model()
        client = get_qdrant_client()
        
        if len(texts) != len(metadata):
            raise ValueError("Number of texts must match number of metadata entries")
        
        logger.debug(f"Adding {len(texts)} vectors to the vector database")
        
        # 1. Generate embeddings for the texts
        embeddings = model.encode(texts)
        
        # 2. Generate IDs if not provided
        if ids is None:
            ids = [str(uuid.uuid4()) for _ in range(len(texts))]
        elif len(ids) != len(texts):
            raise ValueError("Number of IDs must match number of texts")
        
        # 3. Create points for Qdrant
        points = []
        for i, (text, embedding, metadata_item, point_id) in enumerate(zip(texts, embeddings, metadata, ids)):
            # Add the original text to metadata for reference
            payload = dict(metadata_item)
            payload["text"] = text
            payload["created_at"] = datetime.utcnow().isoformat()
            
            points.append(
                PointStruct(
                    id=point_id,
                    vector=embedding.tolist(),
                    payload=payload
                )
            )
        
        # 4. Store vectors in the database in batches
        batch_size = settings.batch_size
        stored_ids = []
        
        for i in range(0, len(points), batch_size):
            batch = points[i:i + batch_size]
            
            client.upsert(
                collection_name=settings.vector_db_collection,
                points=batch
            )
            
            batch_ids = [point.id for point in batch]
            stored_ids.extend(batch_ids)
            
            logger.debug(f"Stored batch {i//batch_size + 1}: {len(batch)} vectors")
        
        logger.info(f"Successfully stored {len(stored_ids)} vectors in the database")
        return stored_ids
        
    except Exception as e:
        logger.error(f"Failed to add vectors: {e}")
        raise


async def get_temporal_context(
    log_id: str,
    window_size: int = 10,
) -> Dict[str, Any]:
    """Get temporal context around a specific log entry.

    Args:
        log_id: ID of the log entry
        window_size: Number of log entries before and after

    Returns:
        Context object with before, target, and after logs
    """
    try:
        settings = get_settings()
        client = get_qdrant_client()
        
        logger.debug(f"Getting temporal context for log {log_id} with window size {window_size}")
        
        # 1. Retrieve the target log entry
        target_points = client.retrieve(
            collection_name=settings.vector_db_collection,
            ids=[log_id],
            with_payload=True,
            with_vectors=False
        )
        
        if not target_points:
            logger.warning(f"Log entry {log_id} not found")
            return {
                "before": [],
                "target": None,
                "after": []
            }
        
        target_point = target_points[0]
        target_payload = target_point.payload
        
        # Extract timestamp from target log
        target_timestamp = target_payload.get("timestamp")
        if not target_timestamp:
            logger.warning(f"No timestamp found for log {log_id}")
            return {
                "before": [target_payload],
                "target": target_payload,
                "after": []
            }
        
        # 2. Retrieve logs before the target timestamp
        before_filter = models.Filter(
            must=[
                models.FieldCondition(
                    key="timestamp",
                    range=models.Range(lt=target_timestamp)
                )
            ]
        )
        
        before_results = client.search(
            collection_name=settings.vector_db_collection,
            query_vector=[0.0] * settings.vector_dimension,  # Dummy vector for filtering
            query_filter=before_filter,
            limit=window_size,
            with_payload=True,
            with_vectors=False
        )
        
        # Sort by timestamp descending and take the most recent ones
        before_logs = sorted(
            [result.payload for result in before_results],
            key=lambda x: x.get("timestamp", 0),
            reverse=True
        )[:window_size]
        
        # 3. Retrieve logs after the target timestamp
        after_filter = models.Filter(
            must=[
                models.FieldCondition(
                    key="timestamp",
                    range=models.Range(gt=target_timestamp)
                )
            ]
        )
        
        after_results = client.search(
            collection_name=settings.vector_db_collection,
            query_vector=[0.0] * settings.vector_dimension,  # Dummy vector for filtering
            query_filter=after_filter,
            limit=window_size,
            with_payload=True,
            with_vectors=False
        )
        
        # Sort by timestamp ascending and take the earliest ones
        after_logs = sorted(
            [result.payload for result in after_results],
            key=lambda x: x.get("timestamp", 0)
        )[:window_size]
        
        # 4. Return the context object
        context = {
            "before": before_logs,
            "target": target_payload,
            "after": after_logs
        }
        
        logger.info(f"Retrieved temporal context for {log_id}: {len(before_logs)} before, {len(after_logs)} after")
        return context
        
    except Exception as e:
        logger.error(f"Failed to get temporal context for {log_id}: {e}")
        return {
            "before": [],
            "target": None,
            "after": []
        }


async def process_logs_for_embedding(log_entries: List[Dict[str, Any]]) -> List[str]:
    """Process log entries and add them to the vector database.
    
    This function extracts meaningful text from log entries and stores their embeddings.
    
    Args:
        log_entries: List of log entry dictionaries
        
    Returns:
        List of vector IDs that were stored
    """
    if not VECTOR_STORE_AVAILABLE:
        logger.warning("Vector store dependencies not available, skipping embedding processing")
        return []
    
    try:
        if not log_entries:
            return []
        
        logger.info(f"Processing {len(log_entries)} log entries for embedding")
        
        # Extract text and metadata from log entries
        texts = []
        metadata = []
        
        for entry in log_entries:
            # Create a meaningful text representation of the log
            text_parts = []
            
            # Add severity if available
            if "severity" in entry:
                text_parts.append(f"[{entry['severity']}]")
            
            # Add main log body
            if "body" in entry:
                text_parts.append(entry["body"])
            
            # Add relevant attributes
            if "attributes" in entry and isinstance(entry["attributes"], dict):
                for key, value in entry["attributes"].items():
                    if key in ["service", "component", "error_type", "operation"]:
                        text_parts.append(f"{key}:{value}")
            
            text = " ".join(text_parts)
            texts.append(text)
            
            # Prepare metadata
            meta = {
                "timestamp": entry.get("timestamp"),
                "severity": entry.get("severity"),
                "client_id": entry.get("client_id"),
                "trace_id": entry.get("trace_id"),
                "span_id": entry.get("span_id"),
                "attributes": entry.get("attributes", {}),
                "resource": entry.get("resource", {})
            }
            metadata.append(meta)
        
        # Store vectors in the database
        vector_ids = await add_vectors(texts, metadata)
        
        logger.info(f"Successfully processed {len(vector_ids)} log entries for embedding")
        return vector_ids
        
    except Exception as e:
        logger.error(f"Failed to process logs for embedding: {e}")
        raise

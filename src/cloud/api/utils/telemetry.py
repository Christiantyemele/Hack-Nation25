"""Telemetry and monitoring utilities."""

import logging
import time
from typing import Dict, Optional

from prometheus_client import Counter, Histogram, Gauge, start_http_server
from opentelemetry import trace
from opentelemetry.exporter.jaeger.thrift import JaegerExporter
from opentelemetry.instrumentation.fastapi import FastAPIInstrumentor
from opentelemetry.instrumentation.sqlalchemy import SQLAlchemyInstrumentor
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor

from api.config import get_settings

logger = logging.getLogger(__name__)

# Prometheus metrics
REQUEST_COUNT = Counter(
    'lognarrator_requests_total',
    'Total number of HTTP requests',
    ['method', 'endpoint', 'status_code']
)

REQUEST_DURATION = Histogram(
    'lognarrator_request_duration_seconds',
    'HTTP request duration in seconds',
    ['method', 'endpoint']
)

LOG_INGESTION_COUNT = Counter(
    'lognarrator_logs_ingested_total',
    'Total number of logs ingested',
    ['client_id', 'status']
)

LOG_PROCESSING_DURATION = Histogram(
    'lognarrator_log_processing_duration_seconds',
    'Log processing duration in seconds',
    ['operation']
)

ACTIVE_CONNECTIONS = Gauge(
    'lognarrator_active_connections',
    'Number of active database connections'
)

ENCRYPTION_OPERATIONS = Counter(
    'lognarrator_encryption_operations_total',
    'Total number of encryption/decryption operations',
    ['operation', 'status']
)

# Global telemetry state
_telemetry_initialized = False
_tracer: Optional[trace.Tracer] = None


def setup_telemetry(app=None):
    """Set up telemetry for the application.

    Args:
        app: FastAPI application instance (for instrumentation)
    """
    global _telemetry_initialized, _tracer
    
    if _telemetry_initialized:
        logger.debug("Telemetry already initialized")
        return
    
    logger.info("Setting up telemetry")
    settings = get_settings()

    try:
        # Initialize Prometheus metrics
        if settings.enable_prometheus:
            _setup_prometheus_metrics(settings.prometheus_port)
        
        # Initialize OpenTelemetry tracing
        if settings.enable_tracing:
            _tracer = _setup_opentelemetry_tracing(app)
        
        # Instrument FastAPI if app is provided
        if app and settings.enable_tracing:
            FastAPIInstrumentor.instrument_app(app)
            SQLAlchemyInstrumentor().instrument()
        
        _telemetry_initialized = True
        logger.info("Telemetry setup complete")
        
    except Exception as e:
        logger.error(f"Failed to setup telemetry: {e}")
        # Don't fail the application if telemetry setup fails
        pass


def _setup_prometheus_metrics(port: int = 8001):
    """Initialize Prometheus metrics server."""
    try:
        # Start Prometheus metrics server on the specified port
        start_http_server(port)
        logger.info(f"Prometheus metrics server started on port {port}")
    except OSError as e:
        if e.errno == 98:  # Address already in use
            logger.warning(f"Prometheus metrics server port {port} is already in use. "
                         f"Metrics will not be available. Consider using a different port "
                         f"by setting PROMETHEUS_PORT environment variable.")
        else:
            logger.error(f"Failed to start Prometheus metrics server on port {port}: {e}")
    except Exception as e:
        logger.error(f"Failed to start Prometheus metrics server on port {port}: {e}")


def _setup_opentelemetry_tracing(app=None) -> trace.Tracer:
    """Initialize OpenTelemetry tracing."""
    try:
        # Set up the tracer provider
        trace.set_tracer_provider(TracerProvider())
        
        # Configure Jaeger exporter (optional - only if Jaeger is available)
        try:
            jaeger_exporter = JaegerExporter(
                agent_host_name="localhost",
                agent_port=6831,
            )
            span_processor = BatchSpanProcessor(jaeger_exporter)
            trace.get_tracer_provider().add_span_processor(span_processor)
            logger.info("Jaeger tracing exporter configured")
        except Exception as e:
            logger.debug(f"Jaeger exporter not available: {e}")
        
        # Get a tracer
        tracer = trace.get_tracer(__name__)
        logger.info("OpenTelemetry tracing initialized")
        
        return tracer
        
    except Exception as e:
        logger.error(f"Failed to setup OpenTelemetry tracing: {e}")
        return None


def get_tracer() -> Optional[trace.Tracer]:
    """Get the global tracer instance."""
    return _tracer


def record_request(method: str, endpoint: str, status_code: int, duration: float):
    """Record HTTP request metrics."""
    try:
        REQUEST_COUNT.labels(method=method, endpoint=endpoint, status_code=status_code).inc()
        REQUEST_DURATION.labels(method=method, endpoint=endpoint).observe(duration)
    except Exception as e:
        logger.debug(f"Failed to record request metrics: {e}")


def record_log_ingestion(client_id: str, count: int, status: str = "success"):
    """Record log ingestion metrics."""
    try:
        LOG_INGESTION_COUNT.labels(client_id=client_id, status=status).inc(count)
    except Exception as e:
        logger.debug(f"Failed to record log ingestion metrics: {e}")


def record_log_processing_duration(operation: str, duration: float):
    """Record log processing duration metrics."""
    try:
        LOG_PROCESSING_DURATION.labels(operation=operation).observe(duration)
    except Exception as e:
        logger.debug(f"Failed to record log processing duration: {e}")


def record_encryption_operation(operation: str, status: str = "success"):
    """Record encryption/decryption operation metrics."""
    try:
        ENCRYPTION_OPERATIONS.labels(operation=operation, status=status).inc()
    except Exception as e:
        logger.debug(f"Failed to record encryption metrics: {e}")


def update_active_connections(count: int):
    """Update active database connections gauge."""
    try:
        ACTIVE_CONNECTIONS.set(count)
    except Exception as e:
        logger.debug(f"Failed to update active connections: {e}")


class MetricsMiddleware:
    """FastAPI middleware for automatic metrics collection."""
    
    def __init__(self, app):
        self.app = app
    
    async def __call__(self, scope, receive, send):
        if scope["type"] != "http":
            await self.app(scope, receive, send)
            return
        
        start_time = time.time()
        method = scope["method"]
        path = scope["path"]
        
        # Wrap the send function to capture status code
        status_code = 500  # Default to error
        
        async def send_wrapper(message):
            nonlocal status_code
            if message["type"] == "http.response.start":
                status_code = message["status"]
            await send(message)
        
        try:
            await self.app(scope, receive, send_wrapper)
        finally:
            duration = time.time() - start_time
            record_request(method, path, status_code, duration)

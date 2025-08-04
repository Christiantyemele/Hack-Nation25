# LogNarrator Architecture Documentation

## System Architecture Overview

LogNarrator is designed with a hybrid architecture that balances security, privacy, and advanced AI capabilities. The system consists of client-side components that run within the user's infrastructure and cloud components that provide advanced analytics while maintaining data privacy.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                      Client Infrastructure                       │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                 LogNarrator Container                   │    │
│  │                                                         │    │
│  │   ┌─────────────┐    ┌───────────────┐    ┌─────────┐   │    │
│  │   │             │    │               │    │         │   │    │
│  │   │    Log      │    │  Encryption   │    │  Local  │   │    │
│  │   │  Collector  │───►│    Engine     │───►│  Cache  │   │    │
│  │   │             │    │               │    │         │   │    │
│  │   └─────────────┘    └───────────────┘    └────┬────┘   │    │
│  │          ▲                                     │        │    │
│  │          │                                     │        │    │
│  │   ┌──────┴──────┐                       ┌─────▼────┐   │    │
│  │   │             │                       │          │   │    │
│  │   │   Target    │                       │  Secure  │   │    │
│  │   │   Systems   │                       │  API     │───┼────┼────┐
│  │   │             │                       │  Client  │   │    │    │
│  │   └─────────────┘                       └─────┬────┘   │    │    │
│  │                                               │        │    │    │
│  │   ┌─────────────┐                       ┌─────▼────┐   │    │    │
│  │   │             │                       │          │   │    │    │
│  │   │  Response   │◄──────────────────────┤   MCP    │   │    │    │
│  │   │  Actions    │                       │  Client  │   │    │    │
│  │   │             │                       │          │   │    │    │
│  │   └─────────────┘                       └──────────┘   │    │    │
│  │                                                         │    │    │
│  └─────────────────────────────────────────────────────────┘    │    │
│                                                                 │    │
└─────────────────────────────────────────────────────────────────┘    │
                                                                       │
                              Encrypted                                │
                             Data Channel                              │
                                  │                                    │
                                  │                                    │
                                  ▼                                    │
┌─────────────────────────────────────────────────────────────────┐    │
│                      LogNarrator Cloud                          │    │
│                                                                 │    │
│   ┌────────────────┐      ┌─────────────────────────────────┐   │    │
│   │                │      │                                 │   │    │
│   │  API Gateway   │◄─────┤  Authentication & Authorization │◄──┘    │
│   │                │      │                                 │        │
│   └───────┬────────┘      └─────────────────────────────────┘        │
│           │                                                          │
│           ▼                                                          │
│   ┌────────────────┐      ┌─────────────────────────────────┐        │
│   │                │      │                                 │        │
│   │  Decryption    │      │  User & Key Management Service  │        │
│   │  Service       │◄─────┤                                 │        │
│   │                │      │                                 │        │
│   └───────┬────────┘      └─────────────────────────────────┘        │
│           │                                                          │
│           ▼                                                          │
│   ┌────────────────┐      ┌─────────────────────────────────┐        │
│   │                │      │                                 │        │
│   │  Log Analysis  │      │      Vector Database            │        │
│   │  Pipeline      │◄────►│                                 │        │
│   │                │      │                                 │        │
│   └───────┬────────┘      └─────────────────────────────────┘        │
│           │                                                          │
│           ▼                                                          │
│   ┌────────────────┐      ┌─────────────────────────────────┐        │
│   │                │      │                                 │        │
│   │  Narrative     │      │      Anomaly Knowledge Base     │        │
│   │  Engine        │◄────►│                                 │        │
│   │                │      │                                 │        │
│   └───────┬────────┘      └─────────────────────────────────┘        │
│           │                                                          │
│           ▼                                                          │
│   ┌────────────────┐      ┌─────────────────────────────────┐        │
│   │                │      │                                 │        │
│   │  Response      │      │      Action Repository          │        │
│   │  Formatter     │◄────►│                                 │        │
│   │                │      │                                 │        │
│   └───────┬────────┘      └─────────────────────────────────┘        │
│           │                                                          │
│           ▼                                                          │
│   ┌────────────────┐                                                 │
│   │                │                                                 │
│   │  Secure API    │────────────────────────────────────────────────┘
│   │  Response      │                                                 
│   │                │                 Encrypted                        
│   └────────────────┘                Response Channel                 
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Component Details

### Client-Side Components

#### 1. Log Collector
- **Purpose**: Gather logs from target systems
- **Implementation**: OpenTelemetry Collector
- **Features**:
  - Plugin architecture supporting multiple log sources
  - Filtering capabilities to reduce noise
  - Buffering for offline operation
  - Metadata enrichment for enhanced context

#### 2. Encryption Engine
- **Purpose**: Secure logs before transmission
- **Implementation**: libsodium-based encryption library
- **Features**:
  - Public-key encryption using client's private key
  - UUID association for identity verification
  - Compression to reduce bandwidth usage
  - Integrity validation via checksums

#### 3. Local Cache
- **Purpose**: Store logs temporarily in case of connectivity issues
- **Implementation**: SQLite database
- **Features**:
  - Configurable retention policies
  - Automatic synchronization when connectivity restored
  - Local pre-filtering to reduce transmission volume

#### 4. Secure API Client
- **Purpose**: Transmit encrypted logs to cloud service
- **Implementation**: Custom client with robust retry logic
- **Features**:
  - TLS 1.3 encryption for transport security
  - Certificate pinning to prevent MITM attacks
  - Bandwidth-aware transmission scheduling

#### 5. MCP Client
- **Purpose**: Execute response actions based on analysis
- **Implementation**: Rust-based Multi-Command Protocol client
- **Features**:
  - Permission-based action authorization
  - Pluggable action modules
  - Audit logging for all executed actions
  - Rollback capabilities for failed actions

### Cloud Components

#### 1. API Gateway
- **Purpose**: Secure entry point for encrypted logs
- **Implementation**: FastAPI with rate limiting and authentication
- **Features**:
  - DDoS protection
  - Client certificate validation
  - Request validation and sanitization

#### 2. Authentication & Authorization
- **Purpose**: Verify client identity and permissions
- **Implementation**: JWT-based auth system with role-based access control
- **Features**:
  - Fine-grained permission model
  - Multi-factor authentication for administrative access
  - Comprehensive audit logging

#### 3. User & Key Management
- **Purpose**: Store and manage user identities and encryption keys
- **Implementation**: PostgreSQL database with HashiCorp Vault
- **Features**:
  - Secure key storage and rotation
  - Key revocation capabilities
  - Multi-tenant isolation

#### 4. Decryption Service
- **Purpose**: Securely decrypt client logs
- **Implementation**: Isolated service with minimal permissions
- **Features**:
  - Memory-only processing (no persistent storage of decrypted logs)
  - Secure key handling
  - Anomaly detection for potential encryption attacks

#### 5. Log Analysis Pipeline
- **Purpose**: Process and analyze decrypted logs
- **Implementation**: Ray-based distributed processing system
- **Features**:
  - Scalable processing for high-volume logs
  - Plugin architecture for multiple analysis techniques
  - Real-time and batch processing capabilities

#### 6. Vector Database
- **Purpose**: Store semantic representations of log patterns
- **Implementation**: ChromaDB or Qdrant
- **Features**:
  - High-dimensional vector storage
  - Efficient similarity search
  - Time-aware context windowing
  - Multi-tenant isolation

#### 7. Narrative Engine
- **Purpose**: Generate contextual explanations of anomalies
- **Implementation**: Fine-tuned LLM (e.g., Mistral 7B)
- **Features**:
  - Context-aware interpretation
  - Temporal pattern recognition
  - Natural language explanations
  - Action recommendation generation

#### 8. Anomaly Knowledge Base
- **Purpose**: Store known patterns and resolutions
- **Implementation**: Graph database (Neo4j)
- **Features**:
  - Pattern-to-resolution mapping
  - Historical effectiveness tracking
  - Cross-customer pattern recognition (anonymized)

#### 9. Action Repository
- **Purpose**: Catalog of possible response actions
- **Implementation**: Structured database with versioning
- **Features**:
  - Parameterized action templates
  - Safety classification of actions
  - Prerequisites and dependency tracking

#### 10. Response Formatter
- **Purpose**: Package analysis results for MCP client consumption
- **Implementation**: Template-based formatter with schema validation
- **Features**:
  - MCP-compatible output format
  - Severity classification
  - Action prioritization

## Data Flow

### 1. Log Ingestion Flow

1. Target systems generate logs
2. Log Collector gathers and standardizes log format
3. Encryption Engine encrypts logs with client's private key
4. Local Cache stores logs temporarily if needed
5. Secure API Client transmits encrypted logs to cloud
6. API Gateway validates requests and routes to processing pipeline
7. Decryption Service uses client's public key to decrypt logs
8. Log Analysis Pipeline processes decrypted logs

### 2. Analysis Flow

1. Log Analysis Pipeline performs initial anomaly detection
2. Unusual patterns are vectorized and compared with Vector Database
3. Similar historical patterns are retrieved for context
4. Narrative Engine combines current anomalies with historical context
5. Anomaly Knowledge Base provides known solutions if available
6. Action Repository suggests appropriate response actions
7. Response Formatter creates structured MCP-compatible output
8. Secure API Response sends encrypted analysis back to client

### 3. Action Flow

1. MCP Client receives and validates analysis response
2. Response actions are evaluated against local permissions
3. Approved actions are executed on target systems
4. Results of actions are logged locally
5. Action effectiveness is optionally reported back to cloud for learning

## Security Considerations

### Data Privacy

- Logs are encrypted before leaving client infrastructure
- Public keys stored in cloud cannot decrypt logs without private key
- Decrypted logs exist only in memory during processing
- No raw logs are ever persistently stored in cloud

### Access Control

- Fine-grained permissions for MCP actions
- Role-based access to administration functions
- Audit logging for all administrative operations
- Secure credential storage with regular rotation

### Network Security

- TLS 1.3 for all communications
- Certificate pinning to prevent MITM attacks
- IP allowlisting for cloud API access
- Rate limiting to prevent abuse

### Compliance Considerations

- Data residency options for regulated industries
- Configurable retention policies
- Anonymization of cross-customer learning
- Audit trails for compliance reporting

## Deployment Models

### Fully Local Deployment

For maximum privacy, all components can be deployed within client infrastructure. This limits some AI capabilities but ensures no data leaves the premises.

### Hybrid Deployment (Default)

Local components handle collection and action execution, while encrypted data is processed in the cloud. This balances privacy with advanced analysis capabilities.

### Fully Managed Deployment

For clients with less restrictive requirements, a fully managed SaaS deployment model is available. This provides the most advanced features with minimal setup overhead.

## Scaling Considerations

### Client-Side Scaling

- Horizontal scaling of collectors for high-volume environments
- Resource-aware container configuration
- Local preprocessing to reduce cloud processing requirements

### Cloud-Side Scaling

- Kubernetes-based auto-scaling for analysis pipeline
- Multi-region deployment options for global customers
- Tenant isolation for enterprise customers

## Operational Considerations

### Monitoring

- Self-monitoring capabilities
- Health checks and alerting
- Performance metrics collection
- Usage analytics for capacity planning

### Updates

- Automatic updates for client container
- Versioned API for backward compatibility
- Phased rollout of new features
- Blue/green deployments for zero-downtime updates

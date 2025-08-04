# LogNarrator: Workflow Documentation

## Core Workflows

This document details the primary workflows within the LogNarrator system, from log collection to automated response actions. These workflows illustrate how the system components interact to deliver the key value propositions.

## 1. System Setup & Onboarding

```mermaid
sequenceDiagram
    actor Admin
    participant Portal as LogNarrator Portal
    participant KeyGen as Key Generation Service
    participant DB as User Database
    participant Container as LogNarrator Container

    Admin->>Portal: Register organization
    Portal->>KeyGen: Request key pair generation
    KeyGen->>Portal: Return public/private key pair
    Portal->>DB: Store user account with public key
    Portal->>Admin: Download container config with private key
    Admin->>Container: Deploy container with config
    Container->>Portal: Verify connection (heartbeat)
    Portal->>Admin: Confirm successful setup
```

### Key Steps

1. **Organization Registration**
   - Administrator creates account on LogNarrator portal
   - Provides basic organization information
   - Selects deployment model (fully local, hybrid, managed)

2. **Security Configuration**
   - System generates unique asymmetric key pair
   - Private key embedded in container configuration
   - Public key stored in LogNarrator cloud for decryption

3. **Container Deployment**
   - Administrator downloads preconfigured container
   - Deploys container to target environment
   - Container establishes secure connection to LogNarrator cloud

4. **Integration Configuration**
   - Administrator configures log sources
   - Sets up MCP action permissions
   - Defines alert routing and notification preferences

## 2. Log Collection & Processing

```mermaid
sequenceDiagram
    participant Source as Log Sources
    participant Collector as Log Collector
    participant Processor as Log Processor
    participant Encryption as Encryption Engine
    participant Cache as Local Cache
    participant API as Cloud API

    Source->>Collector: Generate logs
    Collector->>Processor: Standardize format
    Processor->>Processor: Filter & enrich
    Processor->>Encryption: Pass processed logs
    Encryption->>Encryption: Encrypt with private key
    Encryption->>Cache: Store temporarily
    Cache->>API: Transmit encrypted logs
    API->>Cache: Acknowledge receipt
    Cache->>Cache: Clear transmitted logs
```

### Key Steps

1. **Log Collection**
   - OpenTelemetry collector ingests logs from configured sources
   - Sources can include files, syslog, journald, API endpoints, etc.
   - Collection occurs in real-time with configurable buffering

2. **Initial Processing**
   - Logs standardized to common format
   - Enrichment with metadata (hostname, service name, etc.)
   - Preliminary filtering to reduce noise

3. **Encryption**
   - Logs encrypted using client's private key
   - Each log batch includes client UUID for identification
   - Encrypted data cannot be read without client's public key

4. **Transmission**
   - Encrypted logs sent to LogNarrator cloud API
   - Secure TLS connection with certificate validation
   - Transmission throttled based on network conditions
   - Failed transmissions automatically retried with exponential backoff

## 3. Cloud-Side Analysis

```mermaid
sequenceDiagram
    participant API as Cloud API
    participant Auth as Authentication Service
    participant KeySvc as Key Service
    participant Decrypt as Decryption Service
    participant Analysis as Analysis Pipeline
    participant VectorDB as Vector Database
    participant Narrative as Narrative Engine
    participant Response as Response Formatter

    API->>Auth: Validate request
    Auth->>API: Confirm authentication
    API->>KeySvc: Retrieve public key for UUID
    KeySvc->>API: Return public key
    API->>Decrypt: Forward encrypted logs + public key
    Decrypt->>Analysis: Send decrypted logs
    Analysis->>Analysis: Detect anomalies
    Analysis->>VectorDB: Query similar patterns
    VectorDB->>Analysis: Return historical context
    Analysis->>VectorDB: Store new vectors
    Analysis->>Narrative: Send anomaly + context
    Narrative->>Response: Generate contextual narrative
    Response->>API: Format MCP response
```

### Key Steps

1. **Authentication & Decryption**
   - Incoming request authenticated using JWT
   - Client UUID used to retrieve corresponding public key
   - Encrypted logs decrypted in secure, isolated environment
   - Decrypted logs never persistently stored

2. **Anomaly Detection**
   - Initial analysis using LogAI or similar technology
   - Pattern recognition across multi-dimensional log features
   - Statistical anomaly detection for unexpected behaviors
   - Threshold-based alerting for critical conditions

3. **Contextual Analysis**
   - Anomalous log sequences converted to vector embeddings
   - Similar historical patterns retrieved from vector database
   - Temporal context established across multiple time windows
   - Cross-service relationships identified

4. **Narrative Generation**
   - Language model (e.g., Mistral 7B) interprets anomaly in context
   - Generates human-readable explanation of the issue
   - Identifies potential root causes based on similar patterns
   - Suggests appropriate remediation actions

5. **Response Formatting**
   - Structures analysis results in MCP-compatible format
   - Includes severity classification
   - Provides structured action recommendations
   - Packages context information for troubleshooting

## 4. MCP Action Execution

```mermaid
sequenceDiagram
    participant API as Cloud API
    participant Client as MCP Client
    participant Auth as Permission Manager
    participant Actions as Action Repository
    participant Executor as Tool Executor
    participant Target as Target Systems
    participant Feedback as Feedback Collector

    API->>Client: Send MCP response
    Client->>Client: Validate message
    Client->>Auth: Check action permissions
    Auth->>Client: Return authorized actions
    Client->>Actions: Retrieve action templates
    Actions->>Client: Return parameterized actions
    Client->>Executor: Request action execution
    Executor->>Target: Execute actions
    Target->>Executor: Return results
    Executor->>Client: Report execution status
    Client->>Feedback: Collect effectiveness data
    Feedback->>API: Send action results (optional)
```

### Key Steps

1. **Response Processing**
   - MCP client receives structured response
   - Message validated for integrity and authenticity
   - Severity level determines processing priority

2. **Permission Verification**
   - Recommended actions checked against local permission policy
   - Role-based access control determines execution eligibility
   - High-risk actions may require explicit approval

3. **Action Preparation**
   - Action templates retrieved from local repository
   - Templates parameterized with context-specific values
   - Prerequisite checks performed to ensure safe execution

4. **Execution**
   - Authorized actions executed on target systems
   - Actions can include:
     - Service restarts
     - Configuration changes
     - Resource scaling
     - Traffic routing adjustments
     - Notification dispatching
   - Execution logged for audit purposes

5. **Feedback Collection**
   - Action results captured and evaluated
   - Effectiveness measured based on issue resolution
   - Feedback optionally sent to cloud for learning

## 5. Learning & Improvement

```mermaid
sequenceDiagram
    participant Client as MCP Client
    participant API as Cloud API
    participant Learning as Learning Service
    participant KB as Knowledge Base
    participant Model as Model Training

    Client->>API: Send action effectiveness data
    API->>Learning: Process feedback
    Learning->>KB: Update pattern-action mappings
    Learning->>KB: Store effectiveness metrics
    KB->>Model: Provide training examples
    Model->>Model: Fine-tune recommendation model
    Model->>API: Deploy updated model
```

### Key Steps

1. **Feedback Collection**
   - MCP client optionally reports action outcomes
   - Success/failure status captured
   - Resolution time measured
   - System state changes recorded

2. **Knowledge Base Update**
   - Pattern-to-action mappings strengthened or weakened
   - Effectiveness metrics updated
   - New patterns incorporated into knowledge base

3. **Model Improvement**
   - Recommendation models periodically retrained
   - Organization-specific patterns prioritized
   - Industry-wide patterns (anonymized) incorporated

## Specialized Workflows

### A. Alert Notification Workflow

```mermaid
sequenceDiagram
    participant Engine as Narrative Engine
    participant Formatter as Alert Formatter
    participant Router as Alert Router
    participant Channels as Notification Channels
    participant Escalation as Escalation Manager

    Engine->>Formatter: Generate alert narrative
    Formatter->>Router: Format by severity & type
    Router->>Router: Apply routing rules
    Router->>Channels: Send to appropriate channels
    Channels->>Escalation: Report delivery status
    Escalation->>Escalation: Track acknowledgment
    Escalation->>Router: Escalate if unacknowledged
```

### B. Compliance Reporting Workflow

```mermaid
sequenceDiagram
    participant Audit as Audit Logger
    participant Collector as Report Collector
    participant Generator as Report Generator
    participant Storage as Compliance Storage
    participant Export as Export Service

    Audit->>Audit: Log system events
    Collector->>Audit: Request logs for period
    Audit->>Collector: Return filtered logs
    Collector->>Generator: Provide log data
    Generator->>Generator: Create compliance report
    Generator->>Storage: Store report
    Storage->>Export: Export in required format
```

### C. Deployment Update Workflow

```mermaid
sequenceDiagram
    participant Registry as Container Registry
    participant Update as Update Service
    participant Container as Client Container
    participant Verify as Update Verifier

    Registry->>Update: Publish new version
    Update->>Container: Notify of available update
    Container->>Update: Request update package
    Update->>Container: Send signed update
    Container->>Verify: Verify package signature
    Container->>Container: Apply update
    Container->>Update: Report update status
```

## Error Handling Workflows

### Connection Failure Recovery

```mermaid
sequenceDiagram
    participant Collector as Log Collector
    participant Cache as Local Cache
    participant Monitor as Connection Monitor
    participant API as Cloud API

    Collector->>Cache: Store logs locally
    Monitor->>API: Attempt connection
    API->>Monitor: Connection failed
    Monitor->>Monitor: Implement backoff strategy
    Monitor->>API: Retry connection
    API->>Monitor: Connection restored
    Cache->>API: Send backlogged data
    API->>Cache: Acknowledge receipt
```

### Action Execution Failure

```mermaid
sequenceDiagram
    participant Client as MCP Client
    participant Executor as Tool Executor
    participant Target as Target System
    participant Rollback as Rollback Manager
    participant Notify as Notification Service

    Client->>Executor: Request action execution
    Executor->>Target: Execute action
    Target->>Executor: Report failure
    Executor->>Rollback: Initiate rollback
    Rollback->>Target: Restore previous state
    Rollback->>Executor: Report rollback status
    Executor->>Notify: Send failure notification
    Executor->>Client: Report execution failure
```

## User Interaction Workflows

### Interactive Troubleshooting

```mermaid
sequenceDiagram
    actor User
    participant Dashboard as User Dashboard
    participant Narrative as Narrative Engine
    participant Knowledge as Knowledge Base
    participant Actions as Action Repository

    User->>Dashboard: View incident details
    Dashboard->>Narrative: Request detailed explanation
    Narrative->>Dashboard: Provide contextual narrative
    User->>Dashboard: Request additional context
    Dashboard->>Knowledge: Query related patterns
    Knowledge->>Dashboard: Return historical context
    User->>Dashboard: Select remediation action
    Dashboard->>Actions: Trigger selected action
    Actions->>Dashboard: Report execution status
    Dashboard->>User: Display resolution status
```

### Custom Rule Creation

```mermaid
sequenceDiagram
    actor Admin
    participant UI as Admin Interface
    participant Validator as Rule Validator
    participant Repository as Rule Repository
    participant Engine as Analysis Engine

    Admin->>UI: Create custom detection rule
    UI->>Validator: Validate rule syntax
    Validator->>UI: Return validation result
    Admin->>UI: Confirm rule creation
    UI->>Repository: Store validated rule
    Repository->>Engine: Deploy rule to analyzer
    Engine->>Repository: Acknowledge deployment
    Repository->>UI: Confirm rule activation
    UI->>Admin: Display confirmation
```

## Integration Workflows

### Third-Party Tool Integration

```mermaid
sequenceDiagram
    participant Admin as Administrator
    participant Config as Configuration Service
    participant MCP as MCP Framework
    participant Adapter as Tool Adapter
    participant External as External Tool

    Admin->>Config: Register external tool
    Config->>Config: Generate API credentials
    Config->>MCP: Update action repository
    MCP->>Adapter: Configure connection
    Adapter->>External: Verify connectivity
    External->>Adapter: Confirm connection
    Adapter->>MCP: Register available actions
    MCP->>Config: Update action catalog
    Config->>Admin: Confirm successful integration
```

### Data Source Integration

```mermaid
sequenceDiagram
    participant Admin as Administrator
    participant Config as Configuration Service
    participant Collector as Log Collector
    participant Parser as Format Parser
    participant Source as Data Source

    Admin->>Config: Register new data source
    Config->>Collector: Update collection config
    Collector->>Parser: Configure parser
    Parser->>Source: Test connection
    Source->>Parser: Send sample data
    Parser->>Parser: Validate format
    Parser->>Collector: Confirm compatibility
    Collector->>Config: Update source status
    Config->>Admin: Display integration status
```

## Maintenance Workflows

### Routine Health Check

```mermaid
sequenceDiagram
    participant Scheduler as Health Check Scheduler
    participant Container as Client Container
    participant Components as Container Components
    participant Reporter as Health Reporter
    participant Cloud as Cloud Service

    Scheduler->>Container: Initiate health check
    Container->>Components: Check component status
    Components->>Container: Report component health
    Container->>Container: Check resource usage
    Container->>Reporter: Compile health report
    Reporter->>Cloud: Send health metrics
    Cloud->>Reporter: Acknowledge receipt
    Reporter->>Scheduler: Schedule next check
```

### System Backup

```mermaid
sequenceDiagram
    participant Scheduler as Backup Scheduler
    participant Config as Configuration Service
    participant Data as Local Data Store
    participant Backup as Backup Service
    participant Storage as Secure Storage

    Scheduler->>Config: Initiate backup
    Config->>Data: Export configuration
    Data->>Backup: Provide local data
    Backup->>Backup: Encrypt backup
    Backup->>Storage: Store encrypted backup
    Storage->>Backup: Confirm storage
    Backup->>Scheduler: Update backup status
    Scheduler->>Scheduler: Schedule next backup
```

## Workflow Integration

These workflows do not operate in isolation but form an integrated system where outputs from one workflow become inputs to another. The system maintains state across workflows to ensure consistency and continuity of operations.

### Key Integration Points

1. **Log Collection → Analysis**: Encrypted logs flow from collection to analysis
2. **Analysis → Action**: Analysis results drive automated actions
3. **Action → Learning**: Action outcomes inform future recommendations
4. **Learning → Analysis**: Improved models enhance future analysis

This closed-loop system enables continuous improvement over time, with each component benefiting from the outputs of others.

# LogNarrator: Contextual Log Analysis with Intelligent Response

![LogNarrator Logo](img_1.png)

## 🏆 Hack-Nation25 | Agentic AI & Data Engineering Track

A next-generation log analysis system that goes beyond traditional monitoring by combining privacy-first architecture with contextual intelligence and automated response capabilities.

## 📊 Project Overview

LogNarrator is built for the [Hack-Nation 25](https://hack-nation.ai/) competition in the `Agentic AI & Data Engineering` track. Unlike conventional log analysis solutions like Datadog or Grafana, LogNarrator focuses on extracting meaningful narratives from logs to detect anomalies in context and trigger appropriate automated responses.

## 🌟 Unique Selling Propositions

### 1. Narrative Intelligence
- **Beyond Pattern Matching**: Traditional tools detect anomalies based on static thresholds or simple patterns. LogNarrator builds contextual stories from log sequences.
- **Temporal Context Understanding**: Recognizes that today's logs make sense only in the context of yesterday's events.
- **Root Cause Storytelling**: Doesn't just alert on anomalies but explains them in a human-readable narrative that speeds troubleshooting.

### 2. Privacy-First Architecture
- **Local-First Processing**: Runs as a secure Docker container within the client's infrastructure.
- **End-to-End Encryption**: All logs are encrypted before leaving the client's environment.
- **Minimized Data Exposure**: Even during cloud processing, logs are only decrypted within secure analysis environments.

### 3. Intelligent Action Framework
- **MCP Integration**: Directly connects insights to the Multi-Command Protocol for automated response.
- **Remediation Recommendations**: Suggests specific actions based on historical successful resolutions.
- **Feedback Loop**: Learns from the effectiveness of triggered actions to improve future recommendations.

## 🏛️ System Architecture

### High-Level Architecture

```
┌─────────────────────────────┐       ┌──────────────────────────────────────┐
│                             │       │                                      │
│   Client Infrastructure      │       │         LogNarrator Cloud            │
│   ┌───────────────────────┐ │       │ ┌────────────┐     ┌──────────────┐ │
│   │ LogNarrator Container │ │       │ │            │     │              │ │
│   │ ┌───────────────────┐ │ │       │ │  Secure    │     │ Contextual   │ │
│   │ │  Log Collector    │ │ │       │ │  Analysis  │     │ Intelligence │ │
│   │ └───────────────────┘ │ │       │ │  Pipeline  │     │ Engine       │ │
│   │ ┌───────────────────┐ │ │ HTTPS │ │            │     │              │ │
│   │ │  Encryption       │◄┼─┼───────┼─┤            │     │              │ │
│   │ │  Engine           │ │ │ (E2E  │ │            │◄────┤              │ │
│   │ └───────────────────┘ │ │ Encr.)│ │            │     │              │ │
│   │ ┌───────────────────┐ │ │       │ │            │     │              │ │
│   │ │  MCP Action       │◄┼─┼───────┼─┤            │     │              │ │
│   │ │  Client           │ │ │       │ └────────────┘     └──────────────┘ │
│   │ └───────────────────┘ │ │       │ ┌────────────┐                      │
│   └───────────────────────┘ │       │ │            │     ┌──────────────┐ │
│                             │       │ │  User &     │     │              │ │
│   ┌───────────────────────┐ │       │ │  Key        ├────►│  Vector      │ │
│   │ Target Systems        │ │       │ │  Management │     │  Database    │ │
│   └───────────────────────┘ │       │ │            │     │              │ │
│                             │       │ └────────────┘     └──────────────┘ │
└─────────────────────────────┘       └──────────────────────────────────────┘
```

### Detailed Component Flow

1. **Log Collection & Encryption**
   - Local agent collects logs from specified endpoints
   - Logs are encrypted with client's private key and user UUID
   - Encrypted logs are sent securely to LogNarrator cloud

2. **Secure Cloud Processing**
   - User UUID is used to fetch the corresponding public key
   - Logs are decrypted in a secure, isolated environment
   - Raw logs never exposed outside the secure processing container

3. **Contextual Analysis Pipeline**
   - Initial anomaly detection using LogAI or similar technology
   - Embedding model creates vector representations of log sequences
   - Contextual patterns stored in vector database for "narrative memory"

4. **Narrative Intelligence**
   - Current log context compared with historical patterns
   - AI constructs "stories" explaining detected anomalies in context
   - Generates action recommendations based on pattern matching

5. **MCP Integration & Response**
   - Structured response sent to client MCP
   - Client MCP triggers appropriate tool calls (alerts, automation, remediation)
   - Results from actions feed back into the learning system

## 🔧 Technology Stack

### Client-Side Components

| Component | Technology | Purpose |
|-----------|------------|----------|
| Container Runtime | Docker | Isolated, portable deployment |
| Log Collection | OpenTelemetry Collector | Standard, extensible log ingestion |
| Encryption | libsodium | Public/private key cryptography |
| MCP Client | Custom Rust implementation | Action execution framework |
| Local Cache | SQLite | Offline operation capability |

### Cloud Components

| Component | Technology | Purpose |
|-----------|------------|----------|
| API Gateway | FastAPI (Python) | Secure endpoint for log ingestion |
| User Management | PostgreSQL | Store user accounts and public keys |
| Secret Management | HashiCorp Vault | Secure key storage and rotation |
| Analysis Pipeline | Ray (distributed Python) | Scalable log processing |
| Anomaly Detection | LogAI / Anomalib | Base pattern recognition |
| Embedding Model | SentenceTransformers | Convert logs to vector space |
| Vector Database | Chroma DB / Qdrant | Store contextual log patterns |
| Narrative Engine | LLM (e.g., Mistral 7B) | Generate explanatory narratives |

## 🔄 Workflow Examples

### Example 1: Database Connection Failure Detection

1. **Traditional Alert**: "Database connection failed at 14:35:22"
2. **LogNarrator Response**:
   ```
   Narrative: Database connection failures began 3 minutes after network latency spikes.
   Similar pattern occurred last Tuesday during cloud provider maintenance.
   Root cause analysis suggests cloud provider network issue, not database itself.
   Recommended Action: Switch to backup database cluster until provider issues resolved.
   ```
3. **MCP Action**: Automatically reroute connections to backup cluster

### Example 2: API Slowdown Analysis

1. **Traditional Alert**: "API response times exceeding threshold"
2. **LogNarrator Response**:
   ```
   Narrative: API slowdowns correlate with new deployment at 09:15.
   Pattern matches memory leak in authentication middleware.
   5 similar incidents in past 3 months all resolved by middleware restart.
   Recommended Action: Restart auth middleware service, deploy hotfix #27891
   ```
3. **MCP Action**: Trigger middleware restart, create ticket for permanent fix

## 🚀 Getting Started

### Prerequisites
- Docker & Docker Compose
- Access to log endpoints to monitor
- Network connectivity to LogNarrator cloud (if using hybrid mode)

### Setup Process
1. Create account to receive UUID and key pair
2. Deploy LogNarrator container to target environment
3. Configure log sources and MCP action permissions
4. Start container and verify connectivity

### Deployment Modes
- **Fully Local**: Complete privacy, limited AI capabilities
- **Hybrid**: Encrypted logs processed in cloud, actions executed locally
- **Managed**: Full cloud processing with VPN connectivity to client actions

## 👥 Team

- [Team Member 1] - Architecture & Backend
- [Team Member 2] - ML & Vector Analysis
- [Team Member 3] - Security & MCP Integration
- [Team Member 4] - Frontend & Visualization

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
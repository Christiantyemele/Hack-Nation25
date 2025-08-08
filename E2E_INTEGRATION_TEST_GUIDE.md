# LogNarrator End-to-End Integration Test Guide
## Phase 1B → Phase 2A Complete Pipeline

## 🎯 **Overview**

This guide provides a complete end-to-end test flow demonstrating the full LogNarrator pipeline from **Phase 1B log collection** through **Phase 2A cloud reception**. You'll see logs flow from collection → encryption → transmission → decryption → storage.

### **What This Test Demonstrates**
- ✅ **Phase 1B**: Secure log collection, encryption, and transmission
- ✅ **Phase 2A**: Cloud API Gateway reception, decryption, and storage
- ✅ **Integration**: Complete encrypted pipeline working together
- ✅ **Authentication**: JWT-based API security
- ✅ **Monitoring**: Telemetry and metrics collection

## 📋 **Prerequisites**

### **System Requirements**
- Linux system with systemd/journald
- Docker and Docker Compose
- Python 3.9+ and pip
- Rust toolchain (already built)
- PostgreSQL (via Docker)
- curl and jq for testing

### **Built Components**
- ✅ Phase 1B Rust client binaries (`log_collector`, `mcp_client`)
- ✅ Phase 2A Python API Gateway implementation

## 🚀 **Complete Test Flow**

### **Step 1: Environment Setup**

#### **1.1 Create Test Environment**
```bash
# Create test directory structure
mkdir -p /tmp/lognarrator-integration-test/{config,data,logs}
cd /tmp/lognarrator-integration-test

# Create subdirectories
mkdir -p config/{client,cloud,keys}
mkdir -p data/{client,cloud}
mkdir -p logs/{client,cloud}
```

#### **1.2 Generate Cryptographic Keys**
```bash
# Generate client keypair for encryption
cd /home/christian/RustroverProjects/Hack-Nation25/src/client/rust
./target/debug/log_collector --generate-keys /tmp/lognarrator-integration-test/config/keys/client_key

# Verify keys were created
ls -la /tmp/lognarrator-integration-test/config/keys/
# Expected: client_key.private, client_key.public
```

### **Step 2: Cloud API Gateway Setup**

#### **2.1 Start PostgreSQL Database**
```bash
# Start PostgreSQL container
docker run -d \
  --name lognarrator-postgres \
  -e POSTGRES_DB=lognarrator \
  -e POSTGRES_USER=lognarrator \
  -e POSTGRES_PASSWORD=password \
  -p 5432:5432 \
  postgres

# Wait for database to be ready
sleep 10
docker logs lognarrator-postgres | tail -5
```

#### **2.2 Configure Cloud API**
```bash
# Create cloud API configuration
cat > /tmp/lognarrator-integration-test/config/cloud/.env << 'EOF'
# LogNarrator Cloud API Configuration
SECRET_KEY=integration-test-secret-key-at-least-32-characters-long-for-jwt
JWT_ALGORITHM=HS256
ACCESS_TOKEN_EXPIRE_MINUTES=60

# Database
DATABASE_URL=postgresql://lognarrator:password@localhost:5432/lognarrator
DATABASE_POOL_SIZE=10
DATABASE_MAX_OVERFLOW=5

# Vector Database (stub)
VECTOR_DB_URL=http://localhost:6333
VECTOR_DB_COLLECTION=log_vectors
VECTOR_DIMENSION=768

# API Configuration
DEBUG=true
CORS_ORIGINS=["*"]

# Processing
BATCH_SIZE=50
MAX_QUEUE_SIZE=1000

# Models
EMBEDDING_MODEL=all-MiniLM-L6-v2
LANGUAGE_MODEL_TYPE=mistral
EOF
```

#### **2.3 Install Cloud API Dependencies**
```bash
cd /home/christian/RustroverProjects/Hack-Nation25/src/cloud

# Install Python dependencies
pip install -r requirements.txt

# Set environment
export PYTHONPATH=/home/christian/RustroverProjects/Hack-Nation25/src/cloud:$PYTHONPATH
```

#### **2.4 Start Cloud API Gateway**
```bash
# Start the API Gateway (in background)
cd /home/christian/RustroverProjects/Hack-Nation25/src/cloud
nohup uvicorn api.main:app \
  --host 0.0.0.0 \
  --port 8000 \
  --env-file /tmp/lognarrator-integration-test/config/cloud/.env \
  > /tmp/lognarrator-integration-test/logs/cloud/api.log 2>&1 &

# Wait for API to start
sleep 5

# Verify API is running
curl -s http://localhost:8000/health
# Expected: {"status":"healthy"}
```

### **Step 3: Client Configuration**

#### **3.1 Create Client Configuration**
```bash
cat > /tmp/lognarrator-integration-test/config/client/integration_collector.yaml << 'EOF'
# LogNarrator Integration Test Configuration
# Phase 1B → Phase 2A Pipeline

collector:
  sources:
    # Journald source for system logs
    - source_type: journald
      name: system-integration-test
      units:
        - "systemd-journald.service"
        - "dbus.service"
        - "NetworkManager.service"
    
    # OTLP receiver for external log testing
    - source_type: otlp
      name: integration-otlp
      port: 4318
      interface: "127.0.0.1"

  processors:
    # Add integration test metadata
    - processor_type: resource
      name: integration-metadata
      attributes:
        - action: upsert
          key: "hostname"
          value: "${HOSTNAME}"
        - action: upsert
          key: "environment"
          value: "integration-test"
        - action: upsert
          key: "service.name"
          value: "lognarrator-integration"
        - action: upsert
          key: "test.phase"
          value: "1b-to-2a-pipeline"
    
    # Filter for integration test logs
    - processor_type: filter
      name: integration-filter
      logs:
        include:
          match_type: regexp
          regexp:
            - ".*integration.*"
            - ".*test.*"
            - ".*Started.*"
            - ".*error.*"
            - ".*warning.*"
    
    # Batch for efficient transmission
    - processor_type: batch
      name: integration-batch
      timeout: 5
      send_batch_size: 10

  exporters:
    # Local cache for debugging
    - exporter_type: localcache
      name: integration-local-cache
      directory: "/tmp/lognarrator-integration-test/data/client"
      max_size_mb: 50
    
    # Cloud API Gateway (Phase 2A)
    - exporter_type: lognarrator
      name: integration-cloud-export
      endpoint: "http://localhost:8000/api/v1/logs"
      client_id: "integration-test-client"
      key_path: "/tmp/lognarrator-integration-test/config/keys/client_key.private"

# Encryption configuration
encryption:
  private_key_path: "/tmp/lognarrator-integration-test/config/keys/client_key.private"
  server_public_key_path: "/tmp/lognarrator-integration-test/config/keys/client_key.public"

# Database configuration
database:
  db_path: "/tmp/lognarrator-integration-test/data/client/collector.db"
EOF
```

### **Step 4: Authentication Setup**

#### **4.1 Create Test User and Get Token**
```bash
# Get authentication token (using default admin user)
TOKEN=$(curl -s -X POST "http://localhost:8000/api/v1/auth/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=admin&password=password" | jq -r '.access_token')

echo "Authentication Token: $TOKEN"

# Verify token works
curl -s -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8000/api/v1/auth/me" | jq
```

### **Step 5: Execute Integration Test**

#### **5.1 Start Log Collector (Phase 1B)**
```bash
# Start the log collector
cd /home/christian/RustroverProjects/Hack-Nation25/src/client/rust

# Run collector for 30 seconds to collect logs
timeout 30s ./target/debug/log_collector \
  --config /tmp/lognarrator-integration-test/config/client/integration_collector.yaml \
  --verbose \
  > /tmp/lognarrator-integration-test/logs/client/collector.log 2>&1 &

COLLECTOR_PID=$!
echo "Log Collector started with PID: $COLLECTOR_PID"
```

#### **5.2 Generate Test Logs**
```bash
# Create test log generator
cat > /tmp/lognarrator-integration-test/generate_integration_logs.sh << 'EOF'
#!/bin/bash
echo "🚀 Generating integration test logs..."

# Generate system logs via logger
logger -t "lognarrator-integration" "Started integration test session"
logger -t "lognarrator-integration" "Phase 1B to Phase 2A pipeline test"
logger -t "integration-test" "Testing encrypted log transmission"
logger -t "integration-test" -p user.warning "Test warning: Integration pipeline active"
logger -t "integration-test" -p user.err "Test error: Simulated error for testing"

# Send OTLP test logs
curl -s -X POST "http://127.0.0.1:4318/v1/logs" \
  -H "Content-Type: application/json" \
  -d '{
    "logs": [
      {
        "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%S.%3NZ)'",
        "level": "INFO",
        "message": "OTLP integration test log",
        "attributes": {
          "source": "integration-test",
          "test.type": "otlp-direct"
        }
      }
    ]
  }' || echo "OTLP endpoint not ready yet"

echo "✅ Integration test logs generated"
EOF

chmod +x /tmp/lognarrator-integration-test/generate_integration_logs.sh

# Generate test logs
/tmp/lognarrator-integration-test/generate_integration_logs.sh
```

#### **5.3 Wait and Monitor**
```bash
# Wait for logs to be processed
echo "⏳ Waiting for log processing..."
sleep 15

# Check collector logs
echo "📋 Collector Output (last 20 lines):"
tail -20 /tmp/lognarrator-integration-test/logs/client/collector.log

# Check API logs
echo "📋 API Gateway Output (last 20 lines):"
tail -20 /tmp/lognarrator-integration-test/logs/cloud/api.log
```

### **Step 6: Verification and Results**

#### **6.1 Verify Phase 1B (Client Side)**
```bash
echo "🔍 Phase 1B Verification:"

# Check local cache
echo "Local cached logs:"
ls -la /tmp/lognarrator-integration-test/data/client/

# Check for cache files (fix wildcard expansion issue)
CACHE_FILES=$(ls /tmp/lognarrator-integration-test/data/client/logs_*.jsonl 2>/dev/null)
if [ -n "$CACHE_FILES" ]; then
    echo "✅ Local cache files created"
    echo "Sample cached log:"
    head -1 $(ls /tmp/lognarrator-integration-test/data/client/logs_*.jsonl | head -1) | jq
else
    echo "❌ No local cache files found"
fi

# Check client database
echo "Client database logs:"
sqlite3 /tmp/lognarrator-integration-test/data/client/collector.db \
  "SELECT COUNT(*) as log_count FROM logs;" 2>/dev/null || echo "Database not accessible"
```

#### **6.2 Verify Phase 2A (Cloud Side)**
```bash
echo "🔍 Phase 2A Verification:"

# Check API health
echo "API Health:"
curl -s http://localhost:8000/health | jq

# Search for integration test logs
echo "Logs received by API Gateway:"
curl -s -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8000/api/v1/logs/search?query=integration&limit=5" | jq

# Check Prometheus metrics
echo "Prometheus Metrics (log ingestion):"
curl -s http://localhost:8002/metrics | grep -E "(lognarrator_logs_ingested|lognarrator_requests)" | head -5
```

#### **6.3 Verify End-to-End Pipeline**
```bash
echo "🔗 End-to-End Pipeline Verification:"

# Count logs at each stage
CLIENT_LOGS=$(ls /tmp/lognarrator-integration-test/data/client/logs_*.jsonl 2>/dev/null | wc -l)
API_RESPONSE=$(curl -s -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8000/api/v1/logs/search?query=integration&limit=100" | jq '.total // 0')

echo "📊 Pipeline Results:"
echo "  Client cache files: $CLIENT_LOGS"
echo "  API received logs: $API_RESPONSE"

if [ "$API_RESPONSE" -gt 0 ]; then
    echo "✅ End-to-end pipeline SUCCESS!"
    echo "   Logs successfully flowed from Phase 1B → Phase 2A"
else
    echo "❌ Pipeline issue detected"
    echo "   Check logs for errors"
fi
```

### **Step 7: Advanced Verification**

#### **7.1 Test Encryption/Decryption**
```bash
echo "🔐 Encryption/Decryption Verification:"

# Check for encryption operations in API logs
grep -i "encrypt\|decrypt\|sign" /tmp/lognarrator-integration-test/logs/cloud/api.log | tail -3

# Verify client is sending encrypted data
grep -i "encrypt\|sign\|cloud" /tmp/lognarrator-integration-test/logs/client/collector.log | tail -3
```

#### **7.2 Test Authentication**
```bash
echo "🔑 Authentication Verification:"

# Test without token (should fail)
echo "Testing without authentication:"
curl -s -w "HTTP Status: %{http_code}\n" \
  "http://localhost:8000/api/v1/logs/search?query=test" | tail -1

# Test with token (should succeed)
echo "Testing with authentication:"
curl -s -w "HTTP Status: %{http_code}\n" \
  -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8000/api/v1/logs/search?query=test&limit=1" | tail -1
```

#### **7.3 Performance Metrics**
```bash
echo "📈 Performance Metrics:"

# API Gateway metrics
echo "API Gateway Performance:"
curl -s http://localhost:8002/metrics | grep -E "(request_duration|logs_ingested)" | head -5

# Database connection status
echo "Database Status:"
docker exec lognarrator-postgres psql -U lognarrator -d lognarrator \
  -c "SELECT COUNT(*) as total_logs FROM log_entries;" 2>/dev/null || echo "Database query failed"
```

### **Step 8: Cleanup**

#### **8.1 Stop Services**
```bash
echo "🧹 Cleaning up test environment..."

# Stop log collector
if [ ! -z "$COLLECTOR_PID" ]; then
    kill $COLLECTOR_PID 2>/dev/null
fi

# Stop API Gateway
pkill -f "uvicorn api.main:app" 2>/dev/null

# Stop PostgreSQL
docker stop lognarrator-postgres 2>/dev/null
docker rm lognarrator-postgres 2>/dev/null

echo "✅ Cleanup complete"
```

#### **8.2 Preserve Test Results**
```bash
# Create results summary
cat > /tmp/lognarrator-integration-test/TEST_RESULTS.md << EOF
# LogNarrator Integration Test Results
**Test Date**: $(date)
**Duration**: 30 seconds log collection + verification

## Results Summary
- **Phase 1B**: Log collection, encryption, transmission
- **Phase 2A**: API reception, decryption, storage
- **Integration**: End-to-end pipeline verification

## Files Generated
- Client logs: /tmp/lognarrator-integration-test/data/client/
- API logs: /tmp/lognarrator-integration-test/logs/cloud/api.log
- Collector logs: /tmp/lognarrator-integration-test/logs/client/collector.log

## Test Status
$(if [ "$API_RESPONSE" -gt 0 ]; then echo "✅ SUCCESS - Pipeline working"; else echo "❌ FAILED - Check logs"; fi)
EOF

echo "📄 Test results saved to: /tmp/lognarrator-integration-test/TEST_RESULTS.md"
```

## 🎯 **Expected Results**

### **Successful Test Indicators**
- ✅ **Phase 1B**: Collector starts, processes logs, exports to both local and cloud
- ✅ **Phase 2A**: API receives requests, decrypts data, stores in database
- ✅ **Authentication**: JWT tokens work for API access
- ✅ **Encryption**: Logs are encrypted in transit, decrypted in API
- ✅ **Storage**: Logs appear in PostgreSQL database
- ✅ **Monitoring**: Prometheus metrics show activity

### **Success Criteria**
| Component | Expected Behavior | Verification Command |
|-----------|------------------|---------------------|
| **Log Collection** | Journald logs collected | Check collector.log for "Processing log" |
| **Encryption** | Logs encrypted before transmission | Check for "Successfully exported" messages |
| **API Reception** | HTTP 200 responses from API | Check api.log for successful requests |
| **Decryption** | Encrypted data processed | Check for "Successfully decrypted" in API logs |
| **Database Storage** | Logs stored in PostgreSQL | Query log_entries table |
| **Authentication** | JWT tokens validated | API returns user info with valid token |

## 🔧 **Troubleshooting**

### **Common Issues**

#### **Issue: API Gateway not starting**
```bash
# Check port availability
netstat -tlnp | grep :8000

# Check Python dependencies
pip list | grep -E "(fastapi|uvicorn|sqlalchemy)"

# Check environment variables
env | grep -E "(DATABASE_URL|SECRET_KEY)"
```

#### **Issue: Database connection failed**
```bash
# Check PostgreSQL container
docker ps | grep postgres
docker logs lognarrator-postgres

# Test database connection
docker exec lognarrator-postgres psql -U lognarrator -d lognarrator -c "SELECT 1;"
```

#### **Issue: No logs reaching API**
```bash
# Check collector is sending to correct endpoint
grep -i "endpoint\|export" /tmp/lognarrator-integration-test/config/client/integration_collector.yaml

# Check API is receiving requests
grep -i "POST /api/v1/logs" /tmp/lognarrator-integration-test/logs/cloud/api.log

# Test API endpoint manually
curl -X POST "http://localhost:8000/api/v1/logs" \
  -H "Content-Type: application/json" \
  -d '{"records":[{"timestamp":1234567890000,"severity":"INFO","body":"test"}]}'
```

#### **Issue: Encryption/Decryption errors**
```bash
# Check key files exist
ls -la /tmp/lognarrator-integration-test/config/keys/

# Check key format
file /tmp/lognarrator-integration-test/config/keys/client_key.*

# Check encryption service logs
grep -i "encrypt\|decrypt\|key" /tmp/lognarrator-integration-test/logs/cloud/api.log
```

## 🎉 **Success Confirmation**

When the test completes successfully, you should see:

1. **Collector Output**: "Successfully exported X logs to cloud-demo-export"
2. **API Logs**: "Successfully decrypted N bytes from client integration-test-client"
3. **Database Query**: Returns count > 0 for log_entries table
4. **API Search**: Returns integration test logs with proper metadata
5. **Metrics**: Prometheus shows log ingestion and request metrics

This confirms the **complete Phase 1B → Phase 2A pipeline is working** with:
- ✅ Secure log collection and encryption
- ✅ Reliable transmission to cloud API
- ✅ Proper authentication and authorization
- ✅ Successful decryption and storage
- ✅ Full observability and monitoring

## 📚 **Next Steps**

After successful integration testing:
1. **Production Deployment**: Use real certificates and secure key management
2. **Scale Testing**: Test with higher log volumes
3. **Phase 3 Integration**: Add vector analysis and MCP capabilities
4. **Monitoring Setup**: Deploy Grafana dashboards for production monitoring
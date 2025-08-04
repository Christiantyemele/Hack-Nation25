// Package exporter implements log exporting to the LogNarrator cloud
package exporter

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"time"

	"github.com/lognarrator/client/internal/config"
	"github.com/lognarrator/client/internal/encryption"
	"go.opentelemetry.io/collector/consumer"
	"go.opentelemetry.io/collector/pdata/plog"
	"go.uber.org/zap"
)

// CloudExporter exports logs to the LogNarrator cloud
type CloudExporter struct {
	cfg       config.ExporterConfig
	encryptor *encryption.Encryptor
	client    *http.Client
	logger    *zap.SugaredLogger
}

// NewCloudExporter creates a new cloud exporter
func NewCloudExporter(
	cfg config.ExporterConfig,
	encryptor *encryption.Encryptor,
	logger *zap.SugaredLogger,
) (*CloudExporter, error) {
	// Create HTTP client with appropriate timeouts
	client := &http.Client{
		Timeout: time.Duration(cfg.Timeout) * time.Second,
		// TODO: Configure TLS settings
	}

	return &CloudExporter{
		cfg:       cfg,
		encryptor: encryptor,
		client:    client,
		logger:    logger,
	}, nil
}

// ConsumeLogs implements the OpenTelemetry logs consumer interface
func (e *CloudExporter) ConsumeLogs(ctx context.Context, logs plog.Logs) error {
	e.logger.Debugf("Received %d log records for export", logs.LogRecordCount())

	// Convert logs to JSON
	logData, err := e.logsToJSON(logs)
	if err != nil {
		return fmt.Errorf("failed to convert logs to JSON: %w", err)
	}

	// Encrypt the logs if encryption is enabled
	var payload []byte
	var contentType string

	if e.encryptor != nil {
		encData, err := e.encryptor.Encrypt(logData)
		if err != nil {
			return fmt.Errorf("failed to encrypt logs: %w", err)
		}

		// Marshal the encrypted data structure
		payload, err = json.Marshal(encData)
		if err != nil {
			return fmt.Errorf("failed to marshal encrypted data: %w", err)
		}

		contentType = "application/json+encrypted"
	} else {
		// Use the raw JSON data
		payload = logData
		contentType = "application/json"
	}

	// Send to the cloud endpoint
	req, err := http.NewRequestWithContext(
		ctx,
		"POST",
		e.cfg.Endpoint,
		bytes.NewBuffer(payload),
	)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}

	req.Header.Set("Content-Type", contentType)
	req.Header.Set("User-Agent", "LogNarrator-Collector/0.1.0")

	// TODO: Add retry logic
	resp, err := e.client.Do(req)
	if err != nil {
		return fmt.Errorf("failed to send logs: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		return fmt.Errorf("server returned error: %s", resp.Status)
	}

	e.logger.Debugf("Successfully exported %d log records", logs.LogRecordCount())
	return nil
}

// Capabilities implements the OpenTelemetry consumer capabilities interface
func (e *CloudExporter) Capabilities() consumer.Capabilities {
	return consumer.Capabilities{MutatesData: false}
}

// Start implements the component.Component interface
func (e *CloudExporter) Start(ctx context.Context, host interface{}) error {
	e.logger.Info("Starting LogNarrator cloud exporter")
	return nil
}

// Shutdown implements the component.Component interface
func (e *CloudExporter) Shutdown(ctx context.Context) error {
	e.logger.Info("Shutting down LogNarrator cloud exporter")
	return nil
}

// logsToJSON converts OpenTelemetry logs to JSON format
func (e *CloudExporter) logsToJSON(logs plog.Logs) ([]byte, error) {
	// TODO: Implement proper log format conversion
	// For now, create a simple JSON structure

	type LogRecord struct {
		Timestamp   int64             `json:"timestamp"`
		Severity    string            `json:"severity"`
		Body        string            `json:"body"`
		Attributes  map[string]string `json:"attributes,omitempty"`
		Resource    map[string]string `json:"resource,omitempty"`
		TraceID     string            `json:"traceId,omitempty"`
		SpanID      string            `json:"spanId,omitempty"`
		SeverityNum int32             `json:"severityNum"`
	}

	type LogBatch struct {
		Records []LogRecord `json:"records"`
	}

	batch := LogBatch{
		Records: make([]LogRecord, 0, logs.LogRecordCount()),
	}

	resourceMap := func(res plog.Resource) map[string]string {
		result := make(map[string]string)
		res.Attributes().Range(func(k string, v plog.Value) bool {
			result[k] = v.AsString()
			return true
		})
		return result
	}

	attributeMap := func(attrs plog.Map) map[string]string {
		result := make(map[string]string)
		attrs.Range(func(k string, v plog.Value) bool {
			result[k] = v.AsString()
			return true
		})
		return result
	}

	// Iterate through resource logs
	resourceLogs := logs.ResourceLogs()
	for i := 0; i < resourceLogs.Len(); i++ {
		resourceLog := resourceLogs.At(i)
		resource := resourceMap(resourceLog.Resource())

		// Iterate through scoped logs
		scopeLogs := resourceLog.ScopeLogs()
		for j := 0; j < scopeLogs.Len(); j++ {
			scopeLog := scopeLogs.At(j)

			// Iterate through log records
			logRecords := scopeLog.LogRecords()
			for k := 0; k < logRecords.Len(); k++ {
				logRecord := logRecords.At(k)

				record := LogRecord{
					Timestamp:   logRecord.Timestamp().AsTime().UnixNano() / 1_000_000, // convert to milliseconds
					Severity:    logRecord.SeverityText(),
					SeverityNum: int32(logRecord.SeverityNumber()),
					Body:        logRecord.Body().AsString(),
					Attributes:  attributeMap(logRecord.Attributes()),
					Resource:    resource,
				}

				// Add trace context if available
				traceID := logRecord.TraceID()
				if !traceID.IsEmpty() {
					record.TraceID = traceID.String()
				}

				spanID := logRecord.SpanID()
				if !spanID.IsEmpty() {
					record.SpanID = spanID.String()
				}

				batch.Records = append(batch.Records, record)
			}
		}
	}

	return json.Marshal(batch)
}

// Package config handles configuration loading and validation
package config

import (
	"fmt"
	"io/ioutil"
	"os"
	"path/filepath"
	"strings"

	"gopkg.in/yaml.v2"
)

// Config represents the main configuration structure
type Config struct {
	Collector  CollectorConfig  `yaml:"collector"`
	Encryption EncryptionConfig `yaml:"encryption"`
	Exporter   ExporterConfig   `yaml:"exporter"`
}

// CollectorConfig contains settings for the log collector
type CollectorConfig struct {
	Receivers  map[string]interface{} `yaml:"receivers"`
	Processors map[string]interface{} `yaml:"processors"`
	Pipelines  map[string]Pipeline    `yaml:"pipelines"`
}

// Pipeline defines an OpenTelemetry processing pipeline
type Pipeline struct {
	Receivers  []string `yaml:"receivers"`
	Processors []string `yaml:"processors"`
	Exporters  []string `yaml:"exporters"`
}

// EncryptionConfig contains settings for the encryption engine
type EncryptionConfig struct {
	Enabled      bool   `yaml:"enabled"`
	KeyPath      string `yaml:"keyPath"`
	KeyRotation  int    `yaml:"keyRotationDays"`
	Algorithm    string `yaml:"algorithm"`
	ClientID     string `yaml:"clientId"`
	Compression  bool   `yaml:"compression"`
	BufferSizeMB int    `yaml:"bufferSizeMB"`
}

// ExporterConfig contains settings for the cloud exporter
type ExporterConfig struct {
	Endpoint       string `yaml:"endpoint"`
	Timeout        int    `yaml:"timeoutSeconds"`
	RetryMaxCount  int    `yaml:"retryMaxCount"`
	RetryDelaySec  int    `yaml:"retryDelaySeconds"`
	BatchSize      int    `yaml:"batchSize"`
	MaxQueueSize   int    `yaml:"maxQueueSize"`
	TLSCertPath    string `yaml:"tlsCertPath"`
	TLSVerify      bool   `yaml:"tlsVerify"`
	LocalCachePath string `yaml:"localCachePath"`
}

// LoadConfig loads configuration from a file
func LoadConfig(path string) (*Config, error) {
	absPath, err := filepath.Abs(path)
	if err != nil {
		return nil, fmt.Errorf("failed to resolve config path: %w", err)
	}

	data, err := ioutil.ReadFile(absPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read config file: %w", err)
	}

	// Expand environment variables
	expanded := os.ExpandEnv(string(data))

	var config Config
	if err := yaml.Unmarshal([]byte(expanded), &config); err != nil {
		return nil, fmt.Errorf("failed to parse config: %w", err)
	}

	// Apply defaults
	applyDefaults(&config)

	// Validate configuration
	if err := validateConfig(&config); err != nil {
		return nil, fmt.Errorf("invalid configuration: %w", err)
	}

	return &config, nil
}

// applyDefaults sets default values for unspecified settings
func applyDefaults(config *Config) {
	// Encryption defaults
	if config.Encryption.KeyRotation == 0 {
		config.Encryption.KeyRotation = 30
	}
	if config.Encryption.Algorithm == "" {
		config.Encryption.Algorithm = "XChaCha20-Poly1305"
	}
	if config.Encryption.BufferSizeMB == 0 {
		config.Encryption.BufferSizeMB = 100
	}

	// Exporter defaults
	if config.Exporter.Timeout == 0 {
		config.Exporter.Timeout = 30
	}
	if config.Exporter.RetryMaxCount == 0 {
		config.Exporter.RetryMaxCount = 5
	}
	if config.Exporter.RetryDelaySec == 0 {
		config.Exporter.RetryDelaySec = 10
	}
	if config.Exporter.BatchSize == 0 {
		config.Exporter.BatchSize = 100
	}
	if config.Exporter.MaxQueueSize == 0 {
		config.Exporter.MaxQueueSize = 10000
	}
	if config.Exporter.TLSVerify == false {
		// Default to secure configuration
		config.Exporter.TLSVerify = true
	}
}

// validateConfig checks if the configuration is valid
func validateConfig(config *Config) error {
	// Validate encryption config
	if config.Encryption.Enabled {
		if config.Encryption.KeyPath == "" {
			return fmt.Errorf("encryption.keyPath is required when encryption is enabled")
		}

		validAlgorithms := []string{"XChaCha20-Poly1305", "AES-256-GCM"}
		valid := false
		for _, alg := range validAlgorithms {
			if strings.EqualFold(config.Encryption.Algorithm, alg) {
				valid = true
				break
			}
		}
		if !valid {
			return fmt.Errorf("unsupported encryption algorithm: %s", config.Encryption.Algorithm)
		}

		if config.Encryption.ClientID == "" {
			return fmt.Errorf("encryption.clientId is required when encryption is enabled")
		}
	}

	// Validate exporter config
	if config.Exporter.Endpoint == "" {
		return fmt.Errorf("exporter.endpoint is required")
	}

	// Validate collector config
	if len(config.Collector.Pipelines) == 0 {
		return fmt.Errorf("at least one collector pipeline must be defined")
	}

	for name, pipeline := range config.Collector.Pipelines {
		if len(pipeline.Receivers) == 0 {
			return fmt.Errorf("pipeline '%s' must have at least one receiver", name)
		}
	}

	return nil
}

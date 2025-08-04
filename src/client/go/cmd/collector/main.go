// Package main implements the LogNarrator log collector
package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/lognarrator/client/internal/collector"
	"github.com/lognarrator/client/internal/config"
	"github.com/lognarrator/client/internal/encryption"
	"github.com/lognarrator/client/internal/exporter"
	"github.com/spf13/cobra"
	"go.uber.org/zap"
)

var (
	configPath string
	debug      bool
)

func main() {
	rootCmd := &cobra.Command{
		Use:   "collector",
		Short: "LogNarrator log collector",
		Long:  "Collects logs from various sources and securely transmits them to LogNarrator cloud",
		Run:   run,
	}

	rootCmd.Flags().StringVarP(&configPath, "config", "c", "/app/config/collector.yaml", "Path to the configuration file")
	rootCmd.Flags().BoolVarP(&debug, "debug", "d", false, "Enable debug logging")

	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}

func run(cmd *cobra.Command, args []string) {
	// Initialize logger
	logConfig := zap.NewProductionConfig()
	if debug {
		logConfig = zap.NewDevelopmentConfig()
	}

	logger, err := logConfig.Build()
	if err != nil {
		log.Fatalf("Failed to initialize logger: %v", err)
	}
	defer logger.Sync()

	log := logger.Sugar()
	log.Info("Starting LogNarrator collector")

	// Load configuration
	log.Infof("Loading configuration from %s", configPath)
	cfg, err := config.LoadConfig(configPath)
	if err != nil {
		log.Fatalf("Failed to load configuration: %v", err)
	}

	// Initialize encryption
	log.Info("Initializing encryption engine")
	encryptor, err := encryption.NewEncryptor(cfg.Encryption)
	if err != nil {
		log.Fatalf("Failed to initialize encryption: %v", err)
	}

	// Initialize cloud exporter
	log.Info("Initializing cloud exporter")
	exp, err := exporter.NewCloudExporter(cfg.Exporter, encryptor, log)
	if err != nil {
		log.Fatalf("Failed to initialize exporter: %v", err)
	}

	// Initialize and start collector
	log.Info("Initializing collector pipeline")
	col, err := collector.NewCollector(cfg.Collector, exp, log)
	if err != nil {
		log.Fatalf("Failed to initialize collector: %v", err)
	}

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	log.Info("Starting collector pipeline")
	if err := col.Start(ctx); err != nil {
		log.Fatalf("Failed to start collector: %v", err)
	}

	// Wait for termination signal
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGTERM, syscall.SIGINT)

	s := <-sigCh
	log.Infof("Received signal %s, shutting down", s)

	// Shutdown gracefully
	cancel()
	if err := col.Shutdown(ctx); err != nil {
		log.Errorf("Error during shutdown: %v", err)
	}

	log.Info("LogNarrator collector stopped")
}

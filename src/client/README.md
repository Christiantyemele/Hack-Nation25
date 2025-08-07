# LogNarrator Client

This directory contains the client-side components of LogNarrator:

- Log Collection (Rust implementation)
- Encryption Engine (libsodium)
- Local Storage (SQLite)
- MCP Client (Rust implementation)

## Development Setup

1. Install Rust
2. Set up the development environment with `make setup`
3. Build the client with `make build`
4. Run tests with `make test`

## Architecture

The client components are designed to run within a Docker container in the user's infrastructure. See the architecture documentation for details on component interactions.

## Log Collector

The LogNarrator client includes a Rust-based log collector that can collect logs from various sources:

- File logs
- Journald logs (Linux only)
- Docker container logs
- OpenTelemetry logs via HTTP endpoint

### Testing with Journald

To test the log collector with journald logs on a Linux system:

1. Ensure you have appropriate permissions to access the journal:

```bash
# Add your user to the systemd-journal group
sudo usermod -a -G systemd-journal $USER
# Log out and log back in for changes to take effect
```

2. Create a configuration file `collector_config.yaml` with journald source:

```yaml
sources:
  - source_type: journald
    name: system-journal
    directory: /var/log/journal
    units:
      - systemd
      - sshd

processors:
  - processor_type: filter
    name: error-filter
    logs:
      include:
        match_type: regexp
        regexp:
          - '.*error.*'
          - '.*warning.*'
          - '.*critical.*'
```

3. Run the client with this configuration:

```bash
cargo run -- --config collector_config.yaml
```

### Testing with File Logs

To test with file logs:

1. Create a configuration file with file source:

```yaml
sources:
  - source_type: file
    name: system-logs
    include:
      - /var/log/syslog
      - /var/log/messages
    exclude_filename_pattern: '.*\.gz$'
    start_at: end
```

2. Run the client with this configuration:

```bash
cargo run -- --config file_config.yaml
```

## Log Processing Pipeline

The log collector implements a flexible processing pipeline:

1. **Collection**: Logs are collected from configured sources
2. **Processing**: Logs are processed through configured processors
3. **Export**: Processed logs are exported to the LogNarrator cloud service

The pipeline is fully configurable through the YAML configuration file.

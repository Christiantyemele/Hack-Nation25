# LogNarrator Log Collector

## Overview

The LogNarrator Log Collector is a Rust-based component that efficiently collects logs from various sources, processes them, and prepares them for export to the LogNarrator cloud service. The collector is designed to be:

- **Efficient**: Minimal resource usage, leveraging Rust's performance
- **Flexible**: Support for multiple log sources and formats
- **Secure**: End-to-end encryption of collected logs
- **Resilient**: Local buffering to handle connectivity issues

## Architecture

The Log Collector is built with a pipeline architecture:

```
Log Sources → Processors → Exporters
```

### Log Sources

The collector can gather logs from several types of sources:

- **File Logs**: Standard log files on the filesystem
- **Journald**: SystemD journal logs (Linux only)
- **Docker**: Container logs from Docker
- **OTLP**: OpenTelemetry Protocol HTTP receiver

### Processors

Logs are processed through a configurable pipeline of processors:

- **Resource**: Adds metadata to logs (hostname, service name, etc.)
- **Filter**: Includes or excludes logs based on patterns
- **Batch**: Groups logs for efficient transmission
- **Transform**: Modifies log content (field extraction, masking, etc.)

### Exporters

Processed logs are sent to the configured export destinations:

- **LogNarrator Cloud**: The primary export destination
- **Local Cache**: Temporary storage for logs during connectivity issues

## Configuration

The collector is configured using YAML. Here's an example configuration:

```yaml
# Log collector configuration
collector:
  # Sources define where to collect logs from
  sources:
    - source_type: file
      name: system-logs
      include:
        - /var/log/syslog
        - /var/log/messages
      exclude_filename_pattern: '.*\.gz$'
      start_at: end

    - source_type: journald
      name: journal
      directory: /var/log/journal
      units:
        - systemd
        - sshd

  # Processors transform and filter logs
  processors:
    - processor_type: resource
      name: metadata
      attributes:
        - action: insert
          key: host.name
          value: ${HOSTNAME}
        - action: insert
          key: service.name
          value: "lognarrator-client"

    - processor_type: filter
      name: error-filter
      logs:
        include:
          match_type: regexp
          regexp:
            - '.*error.*'
            - '.*warning.*'
            - '.*critical.*'

    - processor_type: batch
      name: batcher
      timeout: 1s
      send_batch_size: 100
```

## Usage Examples

### Collecting Journald Logs

Journald is the logging system used by systemd on Linux systems. To collect journald logs:

```yaml
sources:
  - source_type: journald
    name: system-journal
    units:
      - systemd  # Collect logs from systemd unit
      - sshd     # Collect logs from SSH daemon
```

Accessing journald logs typically requires appropriate permissions. You may need to add your user to the `systemd-journal` group:

```bash
sudo usermod -a -G systemd-journal $USER
```

### Filtering Logs

To reduce the volume of collected logs, you can use filters:

```yaml
processors:
  - processor_type: filter
    name: error-filter
    logs:
      include:
        match_type: regexp
        regexp:
          - '.*error.*'
          - '.*warning.*'
```

This will only include logs containing "error" or "warning".

### Adding Metadata

You can enrich logs with additional metadata:

```yaml
processors:
  - processor_type: resource
    name: metadata
    attributes:
      - action: insert
        key: environment
        value: "production"
      - action: insert
        key: region
        value: "us-west-2"
```

## Troubleshooting

### Common Issues

1. **Permission Denied for Journald**
   - Solution: Add user to systemd-journal group

2. **Log Files Not Found**
   - Solution: Check paths and permissions

3. **High CPU or Memory Usage**
   - Solution: Increase batch size, add more restrictive filters

### Diagnostic Commands

To check if the collector can access journald:

```bash
journalctl --verify
```

To view the available journal fields that can be collected:

```bash
journalctl -o json -n 1 | jq
```

To test log file access:

```bash
ls -la /var/log/
```

## Performance Considerations

- **Batch Size**: Larger batch sizes reduce overhead but increase memory usage
- **Filter Early**: Apply filters as early as possible in the pipeline
- **Resource Limits**: Consider setting resource limits for the collector process

## Security Considerations

- **Permissions**: Run with minimal required permissions
- **Sensitive Data**: Use transform processors to mask sensitive information
- **Encryption**: Ensure encryption is enabled for data transmission

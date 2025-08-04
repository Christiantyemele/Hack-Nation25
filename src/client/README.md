# LogNarrator Client

This directory contains the client-side components of LogNarrator:

- Log Collection (OpenTelemetry Collector)
- Encryption Engine (libsodium)
- Local Storage (SQLite)
- MCP Client (Rust implementation)

## Development Setup

1. Install Rust and Go
2. Set up the development environment with `make setup`
3. Build the client with `make build`
4. Run tests with `make test`

## Architecture

The client components are designed to run within a Docker container in the user's infrastructure. See the architecture documentation for details on component interactions.

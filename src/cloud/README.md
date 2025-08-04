# LogNarrator Cloud

This directory contains the cloud-side components of LogNarrator:

- API Gateway (FastAPI)
- Authentication & Authorization (JWT, OAuth)
- Log Analysis Pipeline (Ray)
- Vector Database (Qdrant)
- Narrative Engine (Mistral 7B)

## Development Setup

1. Install Python 3.10+
2. Set up the development environment with `make setup`
3. Start local services with `docker-compose up -d`
4. Run the development server with `make run`
5. Run tests with `make test`

## Architecture

The cloud components are designed to be deployed to Kubernetes. See the architecture documentation for details on component interactions.

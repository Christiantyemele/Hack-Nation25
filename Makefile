# LogNarrator Makefile

.PHONY: help setup build test run clean docs lint

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

setup: ## Set up development environment
	@echo "Setting up development environment..."
	@echo "Installing Python dependencies..."
	cd src/cloud && pip install -r requirements-dev.txt
	@echo "Setting up Rust environment..."
	cd src/client/rust && cargo fetch
	@echo "Setting up Go environment..."
	cd src/client/go && go mod download
	@echo "Creating local config directories..."
	mkdir -p config/client data/client config/cloud data/cloud
	@echo "Setup complete!"

build: ## Build the project
	@echo "Building client components..."
	cd src/client/rust && cargo build
	cd src/client/go && go build -o ../../bin/collector ./cmd/collector
	@echo "Building Docker images..."
	docker-compose build
	@echo "Build complete!"

test: ## Run tests
	@echo "Running client tests..."
	cd src/client/rust && cargo test
	cd src/client/go && go test ./...
	@echo "Running cloud tests..."
	cd src/cloud && python -m pytest
	@echo "Tests complete!"

run: ## Run the project locally
	@echo "Starting services..."
	docker-compose up -d
	@echo "Services started! API available at http://localhost:8000"

clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	cd src/client/rust && cargo clean
	rm -rf src/client/bin
	docker-compose down -v
	@echo "Cleanup complete!"

docs: ## Generate documentation
	@echo "Generating API documentation..."
	cd src/cloud && python -c "import uvicorn; from api.main import app; print('API docs will be available at http://localhost:8000/docs')"
	cd src/cloud && uvicorn api.main:app --host 0.0.0.0 --port 8000

lint: ## Run linters
	@echo "Running Rust linter..."
	cd src/client/rust && cargo clippy
	@echo "Running Go linter..."
	cd src/client/go && go vet ./...
	@echo "Running Python linter..."
	cd src/cloud && flake8 .
	@echo "Running Python formatter..."
	cd src/cloud && black --check .
	@echo "Linting complete!"

.DEFAULT_GOAL := help

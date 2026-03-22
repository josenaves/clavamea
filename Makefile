.PHONY: check clippy fmt test build ci prepare

# Run all CI checks locally
ci: fmt clippy build test
	@echo "✅ All CI checks passed locally!"

# Check formatting
fmt:
	@echo "Running cargo fmt..."
	cargo fmt -- --check

# Run linter (denying warnings, just like GitHub Actions)
clippy:
	@echo "Running cargo clippy..."
	cargo clippy -- -D warnings

# Run tests
test:
	@echo "Running cargo test..."
	cargo test

# Build the project
build:
	@echo "Running cargo build..."
	cargo build

# Prepare the offline sqlx query cache (run this when changing database queries)
prepare:
	@echo "Preparing sqlx offline metadata..."
	cargo sqlx prepare

.PHONY: test lint fmt fmt-fix build ci clean help

# Run all tests
test:
	cargo test --workspace

# Run clippy linter
lint:
	cargo clippy --workspace -- -D warnings

# Check formatting
fmt:
	cargo fmt --all -- --check

# Fix formatting
fmt-fix:
	cargo fmt --all

# Build all crates
build:
	cargo build --workspace

# Run all CI checks
ci: fmt lint test

# Clean build artifacts
clean:
	cargo clean

help:
	@echo "Available targets:"
	@echo "  test     - Run all tests"
	@echo "  lint     - Run clippy linter"
	@echo "  fmt      - Check code formatting"
	@echo "  fmt-fix  - Fix code formatting"
	@echo "  build    - Build all crates"
	@echo "  ci       - Run all CI checks (fmt + lint + test)"
	@echo "  clean    - Clean build artifacts"


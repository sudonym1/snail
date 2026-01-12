.PHONY: all test build install clean

# Default target: test, build, and install
all: test build install

# Run all tests
test:
	cargo fmt --check
	RUSTFLAGS="-D warnings" cargo build --features run-proptests
	cargo clippy -- -D warnings
	RUSTFLAGS="-D warnings" cargo test
	python -m pytest python/tests

# Build release wheels
build:
	maturin build --release

# Install into the active Python environment
install:
	maturin develop

# Clean build artifacts
clean:
	cargo clean

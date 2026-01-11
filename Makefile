.PHONY: all test build install clean

# Default target: test, build, and install
all: test build install

# Run all tests
test:
	cargo fmt --check
	RUSTFLAGS="-D warnings" cargo build --features run-proptests
	cargo clippy -- -D warnings
	RUSTFLAGS="-D warnings" cargo test

# Build release binary
build:
	cargo build --release

# Install to ~/.local/bin/
install: build
	cp target/release/snail ~/.local/bin/snail
	@echo "Installed snail to ~/.local/bin/snail"

# Clean build artifacts
clean:
	cargo clean

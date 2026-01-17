.PHONY: all test build install clean sync benchmark develop

UV ?= uv

# Default target: test, build, and install
all: test build install

sync:
	$(UV) sync --extra dev

develop: sync
	$(UV) run -- python -m maturin develop

# Run all tests
test: develop
	cargo fmt --check
	RUSTFLAGS="-D warnings" cargo build
	RUSTFLAGS="-D warnings" cargo build --features run-proptests
	cargo clippy -- -D warnings
	RUSTFLAGS="-D warnings" cargo test
	$(UV) run -- python -m pytest python/tests

# Build release wheels
build: sync
	$(UV) run -- python -m maturin build --release

# Install into the active Python environment
install: sync
	$(UV) tool install --force --reinstall .

benchmark: develop
	SNAIL_PROFILE_NATIVE=1 $(UV) run -- python benchmarks/startup.py --profile-imports -- snail 'print("hello")'

# Clean build artifacts
clean:
	cargo clean
	rm -rf .venv
	rm -rf python/snail/_native*.so
	rm -rf uv.lock
	find . -name __pycache__ -type d -exec rm -rf '{}' \;

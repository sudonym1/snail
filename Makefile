.PHONY: all test build install clean venv

UV ?= uv
VENV_DIR ?= .venv
REQUIREMENTS ?= requirements.txt

# Default target: test, build, and install
all: test build install

venv:
	@if [ ! -d "$(VENV_DIR)" ]; then \
		$(UV) venv $(VENV_DIR); \
	fi
	@if [ -f "$(REQUIREMENTS)" ]; then \
		$(UV) pip install -r "$(REQUIREMENTS)" --python "$(VENV_DIR)/bin/python"; \
	fi

# Run all tests
test: venv
	cargo fmt --check
	RUSTFLAGS="-D warnings" cargo build --features run-proptests
	cargo clippy -- -D warnings
	RUSTFLAGS="-D warnings" cargo test
	$(UV) run -- python -m pytest python/tests

# Build release wheels
build: venv
	$(UV) run -- python -m maturin build --release

# Install into the active Python environment
install: venv
	$(UV) run -- python -m maturin develop

# Clean build artifacts
clean:
	cargo clean

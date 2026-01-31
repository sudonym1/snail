.PHONY: all test build install clean sync benchmark develop test-rust

UV ?= uv

# Any use of snail will be automatically dropped in the case where snail isn't
# on the path.
SNAIL := $(if $(shell command -v snail 2>/dev/null),snail,bash -c 'cat > /dev/null' --)

# Default target: test, build, and install
all: test build install

test-rust:
	cargo fmt --check
	CARGO_TARGET_DIR=target/ci cargo clippy
	CARGO_TARGET_DIR=target/ci cargo test

sync:
	$(UV) sync --extra dev

develop: sync
	$(UV) run -- python -m maturin develop

# Run all tests
test: test-rust develop
	$(UV) run -- python -m pytest python/tests
	sha1sum CLAUDE.md AGENTS.md | $(UV) run -- $(SNAIL) -a '{assert $$1 == prev:$$1?, "CLAUDE.md and AGENTS.md MUST BE THE SAME. AGENTS.md is canonical"; prev=$$1}'

# Build release wheels
build: sync
	$(UV) run -- python -m maturin build --release

# Install into the active Python environment
install: sync
	$(UV) tool install --force --reinstall --python "$$(command -v python3)" .

benchmark: develop
	SNAIL_PROFILE_NATIVE=1 $(UV) run -- python benchmarks/startup.py --profile-imports -- snail 'print("hello")'

# Clean build artifacts
clean:
	cargo clean
	rm -rf .venv
	rm -rf python/snail/_native*.so
	rm -rf uv.lock
	find . -name __pycache__ -type d -exec rm -rf '{}' \;

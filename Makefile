.PHONY: all test build install clean sync benchmark develop test-rust test-python

UV ?= uv

# Default target: test, build, and install
all: test build install

test-rust:
	cargo fmt --check
	CARGO_TARGET_DIR=target/ci cargo clippy
	CARGO_TARGET_DIR=target/ci cargo test

test-python: sync
	#$(UV) run -- black --check python
	$(UV) run -- isort --check-only python
	$(UV) run -- ruff check python
	if $(UV) run -- python -c 'import sys; raise SystemExit(0 if sys.version_info >= (3, 9) else 1)'; then \
		$(UV) run -- mypy python/snail; \
	else \
		echo "Skipping mypy (requires >= 3.9)"; \
	fi

sync:
	$(UV) sync --extra dev

develop: sync
	$(UV) run -- python -m maturin develop

# Run all tests
test: test-rust test-python develop
	$(UV) run -- python -m pytest python/tests
	$(UV) run -- python -c 'from pathlib import Path; import hashlib; files=["CLAUDE.md", "AGENTS.md"]; digests=[hashlib.sha1(Path(f).read_bytes()).hexdigest() for f in files]; assert digests[0] == digests[1], "CLAUDE.md and AGENTS.md MUST BE THE SAME. AGENTS.md is canonical"'

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

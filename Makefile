.PHONY: all test build install clean

# Discover libpython path for LD_PRELOAD (needed for tests that use PyO3 directly)
LIBPYTHON := $(shell python3 -c "import sysconfig, os; print(os.path.join(sysconfig.get_config_var('LIBDIR'), sysconfig.get_config_var('LDLIBRARY')))")

# Default target: test, build, and install
all: test build install

# Run all tests
test:
	cargo fmt --check
	cargo clippy -- -D warnings
	LD_PRELOAD="$(LIBPYTHON)" cargo test

# Build release binary
build:
	cargo build --release

# Install to ~/.local/bin/
install: build
	cp target/release/snail ~/.local/bin/snail
	cp target/release/snail-core ~/.local/bin/snail-core
	@echo "Installed snail and snail-core to ~/.local/bin/"

# Clean build artifacts
clean:
	cargo clean

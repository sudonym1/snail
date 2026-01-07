# snail-cli

Command-line interface for the Snail programming language.

## Purpose

This crate provides the `snail` executable that users interact with to run Snail programs. It handles command-line argument parsing, source file reading, compilation, and execution of generated Python code via subprocess.

## Key Features

- **File execution**: `snail -f script.snail`
- **One-liner execution**: `snail 'print("hello")'`
- **Awk mode**: `snail --awk -f script.snail`
- **Debug mode**: `snail --python` shows generated Python code
- **Auto-print control**: `-P` flag disables auto-print of last expression
- **Argument passing**: Arguments after source are passed to the Snail script
- **Virtual environment support**: Automatically respects active Python virtual environments

## Command-Line Interface

```
snail [options] -f <file> [args]...
snail [options] <code> [args]...
```

Options:
- `-f <file>`: Run a source file
- `-a, --awk`: Enable awk mode
- `-p, --python`: Print generated Python instead of executing
- `-P, --no-print`: Disable auto-printing of last expression
- `-v, --version`: Show version information

## Execution Model

1. Parse command-line arguments using clap
2. Read source from file or command-line argument
3. Compile Snail source to Python using `snail-core`
4. If `--python` flag: print Python code and exit
5. Otherwise: execute Python code via subprocess
   - Uses `python3` by default (configurable via `PYTHON` env var)
   - Pipes generated Python to subprocess stdin
   - Forwards stdout/stderr to user
   - Returns subprocess exit code

## Dependencies

- **snail-core**: Uses compilation API (`compile_snail_source_with_auto_print`, `format_snail_error`)
- **clap**: Command-line argument parsing with derive macros
- **pest/pest_derive**: Inherited for compatibility (may be removable)

## Used By

- End users invoke the `snail` binary directly
- Can be called from shell scripts with shebangs: `#!/usr/bin/env -S snail [--awk] -f`

## Design

The CLI uses subprocess execution to run generated Python code, which:
- Respects virtual environments automatically
- Avoids pyo3 runtime dependencies in the CLI binary
- Supports configurable Python interpreter via `PYTHON` env var
- Provides clean separation between compilation and execution

The binary name is `snail`, built from this crate's main.rs.

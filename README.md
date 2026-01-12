<p align="center">
  <img src="logo.png" alt="Snail logo" width="200">
</p>

<h1 align="center">Snail</h1>
<p align="center"><em>What do you get when you shove a snake in a shell?</em></p>

<h1>Snail, while I hope it is useful to myself and others, is my attempt at
improving my knowledge of AI code developement. Things are probably broken
in interesting and horrible ways.</h1>

---

**Snail** is a programming language that compiles to Python, combining Python's power with Perl/awk-inspired syntax for quick scripts and one-liners. No more whitespace sensitivity—just curly braces and concise expressions.

## ✨ What Makes Snail Unique

### Curly Braces, Not Indentation

Write Python logic without worrying about tabs vs spaces:

```snail
def process(items) {
    for item in items {
        if item > 0 { print(item) }
        else { continue }
    }
}
```

### Built-in Subprocess Pipelines

Shell commands are first-class citizens with `$()` capture and `|` piping:

```snail
# Capture command output with interpolation
name = "world"
greeting = $(echo hello {name})

# Pipe data through commands
result = "foo\nbar\nbaz" | $(grep bar) | $(cat -n)

# Check command status
@(make build)?  # returns exit code on failure instead of raising
```

### Compact Error Handling

The `?` operator makes error handling terse yet expressive:

```snail
# Swallow exception, get the error object
err = risky_operation()?

# Provide a fallback value (exception available as $e)
value = parse_json(data):{}?
details = fetch_url(url):"Error: {$e}"?

# Access attributes directly
name = risky()?.__class__.__name__
args = risky()?.args[0]
```

### Regex Literals

Pattern matching without `import re`:

```snail
if email in /^[\w.]+@[\w.]+$/ {
    print("Valid email")
}

# Compiled regex for reuse
pattern = /\d{3}-\d{4}/
match = pattern.search(phone)
```

### Awk Mode

Process files line-by-line with familiar awk semantics:

```snail
#!/usr/bin/env -S snail --awk -f
BEGIN { total = 0 }
/^[0-9]+/ { total = total + int($f[0]) }
END { print("Sum:", total) }
```

Built-in variables: `$l` (line), `$f` (fields), `$n` (line number), `$fn` (per-file line number), `$p` (file path), `$m` (last match).

### Pipeline Operator

The `|` operator enables data pipelining through objects that implement `__pipeline__`:

```snail
# Pipe data to subprocess stdin
result = "hello\nworld" | $(grep hello)

# Chain multiple transformations
output = "foo\nbar" | $(grep foo) | $(wc -l)

# Custom pipeline handlers
class Doubler {
    def __pipeline__(self, x) { return x * 2 }
}
doubled = 21 | Doubler()  # yields 42
```

### JSON Queries with JMESPath

Parse and query JSON data with the `json()` function and structured pipeline accessor:

```snail
# Parse JSON and query with $[jmespath]
data = json($(curl -s api.example.com/users))
names = data | $[users[*].name]
first_email = data | $[users[0].email]

# Inline parsing and querying
result = json('{"foo": 12}') | $[foo]
```

### Full Python Interoperability

Snail compiles to Python AST—import any Python module, use any library:

```snail
import pandas as pd
from pathlib import Path

df = pd.read_csv(Path("data.csv"))
filtered = df[df["value"] > 100]
```

## 🚀 Quick Start

```bash
# Install from PyPI
pip install snail

# Run a one-liner
snail "print('Hello, Snail!')"

# Execute a script
snail -f script.snail

# Awk mode for text processing
cat data.txt | snail --awk '/error/ { print($l) }'
```

## 🏗️ Architecture

Snail compiles to Python through a multi-stage pipeline:

```mermaid
flowchart TB
    subgraph Input
        A[Snail Source Code]
    end

    subgraph Parsing["Parsing (Pest PEG Parser)"]
        B1[crates/snail-parser/src/snail.pest<br/>Grammar Definition]
        B2[crates/snail-parser/<br/>Parser Implementation]
    end

    subgraph AST["Abstract Syntax Tree"]
        C1[crates/snail-ast/src/ast.rs<br/>Program AST]
        C2[crates/snail-ast/src/awk.rs<br/>AwkProgram AST]
    end

    subgraph Lowering["Lowering & Code Generation"]
        D1[crates/snail-lower/<br/>AST → Python AST Transform]
        D2[python/snail/runtime/<br/>Runtime Helpers]
        D3[crates/snail-codegen/<br/>Python AST → Source Code]
    end

    subgraph Execution
        E1[python/snail/cli.py<br/>CLI Interface]
        E2[pyo3 extension<br/>in-process exec]
    end

    A -->|Regular Mode| B1
    A -->|Awk Mode| B1
    B1 --> B2
    B2 -->|Regular| C1
    B2 -->|Awk| C2
    C1 --> D1
    C2 --> D1
    D1 --> D2
    D1 --> D3
    D2 --> D3
    D3 --> E1
    E1 --> E2
    E2 --> F[Python Execution]

    style A fill:#e1f5ff
    style F fill:#e1ffe1
    style D2 fill:#fff4e1
```

**Key Components:**

- **Parser**: Uses [Pest](https://pest.rs/) parser generator with PEG grammar defined in `src/snail.pest`
- **AST**: Separate representations for regular Snail (`Program`) and awk mode (`AwkProgram`) with source spans for error reporting
- **Lowering**: Transforms Snail AST into Python AST, emitting helper calls backed by `snail.runtime`
  - `?` operator → `__snail_compact_try`
  - `$(cmd)` subprocess capture → `__SnailSubprocessCapture`
  - `@(cmd)` subprocess status → `__SnailSubprocessStatus`
  - Regex literals → `__snail_regex_search` and `__snail_regex_compile`
- **Code Generation**: Converts Python AST to Python source for in-process execution
- **CLI**: Python wrapper (`python/snail/cli.py`) that executes via the extension module

## 📚 Documentation

- **[Language Reference](docs/REFERENCE.md)** — Complete syntax and semantics
- **[examples/all_syntax.snail](examples/all_syntax.snail)** — Every feature in one file
- **[examples/awk.snail](examples/awk.snail)** — Awk mode examples

## 🔌 Editor Support

Vim/Neovim plugin with syntax highlighting, formatting, and run commands:

```vim
Plug 'sudonym1/snail', { 'rtp': 'extras/vim' }
```

See [extras/vim/README.md](extras/vim/README.md) for details. Tree-sitter grammar available in `extras/tree-sitter-snail/`.

## 🛠️ Building from Source

### Prerequisites

**Python 3.10+** (required at runtime)

Snail runs in-process via a Pyo3 extension module, so it uses the active Python environment.

Installation per platform:
- **Ubuntu/Debian**: `sudo apt install python3 python3-dev`
- **Fedora/RHEL**: `sudo dnf install python3 python3-devel`
- **macOS**: `brew install python@3.12` (or use the system Python 3)
- **Windows**: Download from [python.org](https://www.python.org/downloads/)

**No Python packages required**: Snail vendors jmespath as part of the Python package.

**Rust toolchain** (cargo and rustc)

Install Rust using [rustup](https://rustup.rs):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

This installs `cargo` (Rust's package manager) and `rustc` (the Rust compiler). After installation, restart your shell or run:

```bash
source $HOME/.cargo/env
```

Verify installation:

```bash
cargo --version  # Should show cargo 1.70+
rustc --version  # Should show rustc 1.70+
python3 --version  # Should show Python 3.10+
```

**maturin** (build tool)

```bash
pip install maturin
```

### Build and Install

```bash
# Clone the repository
git clone https://github.com/sudonym1/snail.git
cd snail

# Create and activate a venv (recommended)
python3 -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate

# Build and install into the venv
maturin develop

# Or build wheels for distribution
maturin build --release
```

### Running Tests

```bash
# Run all Rust tests (parser, lowering, awk mode; excludes proptests by default)
cargo test

# Run tests including property-based tests (proptests)
cargo test --features run-proptests

# Check code formatting and linting
cargo fmt --check
cargo clippy -- -D warnings

# Build with all features enabled (required before committing)
cargo build --features run-proptests

# Run Python CLI tests
python -m pytest python/tests
```

**Note on Proptests**: The `snail-proptest` crate contains property-based tests that are skipped by default to keep development iteration fast. Use `--features run-proptests` to run them. Before committing, verify that `cargo build --features run-proptests` compiles successfully.

### Troubleshooting

**Using with virtual environments:**

Activate the environment before running snail so it uses the same interpreter:

```bash
# Create and activate a venv
python3 -m venv myenv
source myenv/bin/activate  # On Windows: myenv\Scripts\activate

# Install and run
pip install snail
snail "import sys; print(sys.prefix)"
```

## 📋 Project Status

See [docs/PLANNING.md](docs/PLANNING.md) for the development roadmap.

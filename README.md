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
# Run a one-liner
snail "print('Hello, Snail!')"

# Execute a script
snail -f script.snail

# Awk mode for text processing
cat data.txt | snail --awk '/error/ { print($l) }'

# See the generated Python
snail --python "x = risky()? ; print(x)"

# Self-update to latest release
snail --update
```

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

## 🛠️ Building

Snail is written in Rust and uses pyo3 to execute generated Python:

```bash
cargo build
cargo test

# Requires Python 3 on PATH
export PYO3_PYTHON=python3
```

## 📋 Project Status

See [docs/PLANNING.md](docs/PLANNING.md) for the development roadmap.

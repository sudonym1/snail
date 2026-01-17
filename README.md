<p align="center">
  <img src="logo.png" alt="Snail logo" width="200">
</p>
<p align="center"><em>What do you get when you shove a snake in a shell?</em></p>

<h1 align="center">Snail</h1>

**Snail** is a programming language that compiles to Python, combining Python's familiarity and extensive libraries with Perl/awk-inspired syntax for quick scripts and one-liners.

## Installing Snail

Install [uv](https://docs.astral.sh/uv/getting-started/installation/) and then run:

```bash
uv tool install -p 3.12 snail-lang
```

That installs the `snail` CLI for your user; try it with `snail "print('hello')"` once the install completes.

## ✨ What Makes Snail Unique

### Curly Braces, Not Indentation

Write Python logic without worrying about whitespace:

```snail
def process(items) {
    for item in items {
        if item > 0 { print(item) }
        else { continue }
    }
}
```

Note, since it is jarring to write python with semicolons everywhere,
semicolons are optional. You can separate statements with newlines.

### Awk Mode

Process files line-by-line with familiar awk semantics:

```snail-awk("5\n4\n3\n2\n1\nbanana\n")
BEGIN { total = 0 }
/^[0-9]+/ { total = total + int($1) }
END { print("Sum:", total); assert total == 15}
```

Built-in variables: `$0` (line), `$1`, `$2` etc (access fields), `$n` (line number), `$fn` (per-file line number), `$p` (file path), `$m` (last match).


### Compact Error Handling

The `?` operator makes error handling terse yet expressive:

```snail
# Swallow exception, return None
err = risky()?

# Swallow exception, return exception object
err = risky():$e?

# Provide a fallback value (exception available as $e)
value = js("malformed json"):{"error": "invalid json"}?
details = fetch_url("foo.com"):"default html"?
exception_info = fetch_url("example.com"):$e.http_response_code?

# Access attributes directly
name = risky("")?.__class__.__name__
args = risky("becomes a list"):[1,2,3]?[0]
```

### Pipeline Operator

The `|` operator enables data pipelining as syntactic sugar for nested
function calls. `x | y | z` becomes `z(y(x))`. This lets you stay in a
shell mindset.

```snail
# Pipe data to subprocess stdin
result = "hello\nworld" | $(grep hello)

# Chain multiple transformations
output = "foo\nbar" | $(grep foo) | $(wc -l)

# Custom pipeline handlers
class Doubler {
    def __call__(self, x) { return x * 2 }
}
doubled = 21 | Doubler()  # yields 42
```

Arbitrary callables make up pipelines, even if they have multiple parameters.
Snail supports this via placeholders.
```snail
greeting = "World" | greet("Hello ", _)  # greet("Hello ", "World")
excited = "World" | greet(_, "!")        # greet("World", "!")
formal = "World" | greet("Hello ", suffix=_)  # greet("Hello ", "World")
```

When a pipeline targets a call expression, the left-hand value is passed to the
resulting callable. If the call includes a single `_` placeholder, Snail substitutes
the piped value at that position (including keyword arguments). Only one
placeholder is allowed in a piped call. Outside of pipeline calls, `_` remains a
normal identifier.

### Built-in Subprocess

Shell commands are first-class citizens with capturing and non-capturing
forms.

```snail
# Capture command output with interpolation
greeting = $(echo hello {name})

# Pipe data through commands
result = "foo\nbar\nbaz" | $(grep bar) | $(cat -n)

# Check command status
@(make build)?  # returns exit code on failure instead of raising
```


### Regex Literals

Snail supports first class patterns. Think of them as an infinte set.

```snail
if bad_email in /^[\w.]+@[\w.]+$/ {
    print("Valid email")
}

# Compiled regex for reuse
pattern = /\d{3}-\d{4}/
match = pattern.search(phone)
```

NOTE: this feature is WIP.

### JSON Queries with JMESPath

Parse and query JSON data with the `js()` function and structured pipeline accessor:

```snail
# Parse JSON and query with $[jmespath]

# JSON query with JMESPath
data = js($(curl -s https://api.github.com/repos/sudonym1/snail))
counts = data | $[stargazers_count]

# Inline parsing and querying
result = js('{{"foo": 12}}') | $[foo]

# JSONL parsing returns a list
names = js('{{"name": "Ada"}}\n{{"name": "Lin"}}') | $[[*].name]
```

### Full Python Interoperability

Snail compiles to Python AST—import any Python module, use any library, in any
environment. Assuming that you are using Python 3.10 or later.

## 🚀 Quick Start

```bash
# One-liner: arithmetic + interpolation
snail 'name="Snail"; print("{name} says: {6 * 7}")'

# JSON query with JMESPath
snail 'js($(curl -s https://api.github.com/repos/sudonym1/snail)) | $[stargazers_count]'

# Compact error handling with fallback
snail 'result = int("oops"):"bad int {$e}"?; print(result)'

# Regex match and capture
snail 'm = "user@example.com" in /^[\\w.]+@([\\w.]+)$/; if m { print(m[1]) }'

# Awk mode: print line numbers for matches
rg -n "TODO" README.md | snail --awk '/TODO/ { print("{$n}: {$0}") }'
```

## 🏗️ Architecture

**Key Components:**

- **Parser**: Uses [Pest](https://pest.rs/) parser generator with PEG grammar defined in `src/snail.pest`
- **AST**: Separate representations for regular Snail (`Program`) and awk mode (`AwkProgram`) with source spans for error reporting
- **Lowering**: Transforms Snail AST into Python AST, emitting helper calls backed by `snail.runtime`
  - `?` operator → `__snail_compact_try`
  - `$(cmd)` subprocess capture → `__SnailSubprocessCapture`
  - `@(cmd)` subprocess status → `__SnailSubprocessStatus`
  - Regex literals → `__snail_regex_search` and `__snail_regex_compile`
- **Execution**: Compiles Python AST directly for in-process execution
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

### Build, Test, and Install

```bash
# Clone the repository
git clone https://github.com/sudonym1/snail.git
cd snail

make test
make install
```


**Note on Proptests**: The `snail-proptest` crate contains property-based tests that are skipped by default to keep development iteration fast.

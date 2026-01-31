<p align="center">
  <img src="logo.png" alt="Snail logo" width="200">
</p>

<h1 align="center">Snail</h1>

**Snail** is a programming language that compiles to Python, combining Python's familiarity and extensive libraries with Perl/awk-inspired syntax for quick scripts and one-liners. Its what you get when you shove a snake in a shell.

## AI Slop!

Snail is me learning how to devlop code using LLMs. I think its neat, and
maybe useful. I don't think this is high quality. I am going to try and LLM my
way into something good, but its certainly not there yet.

## Installing Snail

```bash
pip install snail-lang
-or-
uv tool install snail-lang
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

```snail-awk("hello world\nfoo bar\n")
BEGIN { print("start") }
/hello/ { print("matched:", $0) }
{ print($1, "->", $2) }
END { print("done") }
```

**Built-in variables:**

| Variable | Description |
|----------|-------------|
| `$0` | Current line (with newline stripped) |
| `$1`, `$2`, ... | Individual fields (whitespace-split) |
| `$f` | All fields as a list |
| `$n` | Global line number (across all files) |
| `$fn` | Per-file line number |
| `$p` | Current file path |
| `$m` | Last regex match object |

Begin/end blocks can live in the source file (`BEGIN { ... }` / `END { ... }`) or be supplied
via CLI flags (`-b`/`--begin`, `-e`/`--end`) for setup and teardown. CLI BEGIN blocks run
before in-file BEGIN blocks; CLI END blocks run after in-file END blocks.
`BEGIN` and `END` are reserved keywords in all modes.
BEGIN/END blocks are regular Snail blocks, so awk/map-only `$` variables are not available inside them.
```bash
echo -e "5\n4\n3\n2\n1" | snail --awk --begin 'total = 0' --end 'print("Sum:", total)' '/^[0-9]+/ { total = total + int($1) }'
```

### Map Mode

Process files one at a time instead of line-by-line:

```snail-map
BEGIN { print("start") }
print("File:", $src)
print("Size:", len($text), "bytes")
END { print("done") }
```

**Built-in variables:**

| Variable | Description |
|----------|-------------|
| `$src` | Current file path |
| `$fd` | Open file handle for the current file |
| `$text` | Lazy text view of the current file contents |

Begin/end blocks can live in the source file (`BEGIN { ... }` / `END { ... }`) or be supplied
via CLI flags (`-b`/`--begin`, `-e`/`--end`) for setup and teardown. CLI BEGIN blocks run
before in-file BEGIN blocks; CLI END blocks run after in-file END blocks.
BEGIN/END blocks are regular Snail blocks, so awk/map-only `$` variables are not available inside them.
`BEGIN` and `END` are reserved keywords in all modes.
```bash
snail --map --begin "print('start')" --end "print('done')" "print($src)" *.txt
```

### Compact Error Handling

The `?` operator makes error handling terse yet expressive:

```snail
# Swallow exception, return None
err = risky()?

# Swallow exception, return exception object
err = risky():$e?

# Provide a fallback value (exception available as $e)
value = js("malformed json"):%{"error": "invalid json"}?
details = fetch_url("foo.com"):"default html"?
exception_info = fetch_url("example.com"):$e.http_response_code?

# Access attributes directly
name = risky("")?.__class__.__name__
args = risky("becomes a list"):[1,2,3]?[0]
```

### Destructuring + `if let` / `while let`

Unpack tuples and lists directly, including Python-style rest bindings:

```snail
x, *xs = [1, 2, 3]

if let [head, *tail] = [1, 2, 3]; head > 0 {
    print(head, tail)
}
```

`if let`/`while let` only enter the block when the destructuring succeeds. A guard
after `;` lets you add a boolean check that runs after the bindings are created.

Note that this syntax is more powerful than the walrus operator as that does
not allow for destructuring.


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
match2 = "555-1212" in pattern
```

Snail regexes don't return a match object, rather they return a tuple
containing all of the match groups, including group 0. Both `search` and `in`
return the same tuple (or `()` when there is no match).

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
environment. Assuming that you are using Python 3.8 or later.

## 🚀 Quick Start

```bash
# One-liner: arithmetic + interpolation
snail 'name="Snail"; print("{name} says: {6 * 7}")'

# JSON query with JMESPath
snail 'js($(curl -s https://api.github.com/repos/sudonym1/snail)) | $[stargazers_count]'

# Compact error handling with fallback
snail 'result = int("oops"):"bad int {$e}"?; print(result)'

# Regex match and capture
snail 'if let [_, user, domain] = "user@example.com" in /^[\w.]+@([\w.]+)$/ { print(domain) }'

# Awk mode: print line numbers for matches
rg -n "TODO" README.md | snail --awk '/TODO/ { print("{$n}: {$0}") }'
```

## 📚 Documentation

Documentation is WIP

- **[Language Reference](docs/REFERENCE.md)** — Complete syntax and semantics
- **[examples/all_syntax.snail](examples/all_syntax.snail)** — Every feature in one file
- **[examples/awk.snail](examples/awk.snail)** — Awk mode examples
- **[examples/map.snail](examples/map.snail)** — Map mode examples

## 🔌 Editor Support

Vim/Neovim plugin with syntax highlighting, formatting, and run commands:

```vim
Plug 'sudonym1/snail', { 'rtp': 'extras/vim' }
```

See [extras/vim/README.md](extras/vim/README.md) for details. Tree-sitter grammar available in `extras/tree-sitter-snail/`.

## Performance

Section is WIP

Startup performance is benchmarked with `./benchmarks/startup.py`. On my
machine snail adds 5 ms of overhead above the regular python3 interpreter.

## 🛠️ Building from Source

### Prerequisites

**Python 3.8+** (required at runtime)

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

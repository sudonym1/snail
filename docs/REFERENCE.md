# Snail language reference

Snail keeps Python's runtime and evaluation rules but swaps indentation-based
blocks for curly braces. The examples here mirror the runnable tour in
`examples/all_syntax.snail`.

## Running Snail code
- Run a one-liner: `snail "print('hi')"`
- Execute a file: `snail -f path/to/script.snail`

## Map mode
Map mode processes input files one at a time:
```bash
snail --map "print($src)" file_a.txt file_b.txt
```

Map mode provides three special variables:
- `$src`: current file path
- `$fd`: open file handle for the current file
- `$text`: lazy text view of the current file contents

Map mode opens files lazily: the file is only opened when `$fd` or `$text` is
first accessed. Scripts that only use `$src` won't attempt to open files, and
missing/unreadable paths only error once `$fd`/`$text` are used. The file handle
is closed when the per-file `with` scope ends, so `$fd`/`$text` behave like a
closed file if accessed afterward.

Begin/end blocks can live in the source file:
```snail
BEGIN { print("start") }
print($src)
END { print("done") }
```

You can also run setup/teardown blocks with `--begin` and `--end`, which execute
once before the first file and once after the last file. CLI BEGIN blocks run before
in-file BEGIN blocks; CLI END blocks run after in-file END blocks:
```bash
snail --map --begin "print('start')" --end "print('done')" "print($src)" *.txt
```
`BEGIN` and `END` are reserved keywords in all modes.
BEGIN/END blocks are regular Snail blocks, so awk/map-only variables are not available inside them.

## Modules and imports
Snail uses Python's import semantics and exposes the same namespaces:
```snail
import math, sys as sysmod
from math import sqrt as root
```

### Auto-import
Common standard library modules are available without explicit imports:
- `sys` — system-specific parameters and functions
- `os` — operating system interface
- `Path` — `pathlib.Path` for filesystem paths

```snail
# No import needed
print(sys.version)
print(os.getcwd())
config = Path("config.json")
```

Auto-imports are lazy: modules are only loaded when first accessed. To disable
auto-import (for example, to ensure scripts are explicit about dependencies),
use the `-I` or `--no-auto-import` flag:
```bash
snail -I "print(sys.version)"  # NameError: name 'sys' is not defined
```

User assignments shadow auto-imported names, so `sys = "custom"` works as
expected without conflict.

## Statements and expressions
- Assignments mirror Python (`value = 1`). Multiple statements can be separated
  with semicolons.
- Destructuring assignment works for tuples and lists:
  `x, y = pair`, `[a, b] = items`, and starred rest bindings such as
  `x, *xs = values`.
- Boolean operators, comparisons, membership checks, and arithmetic follow
  Python's precedence and short-circuiting rules.
- Conditional expressions are supported: `fallback = "yes" if flag else "no"`.
- Compound expressions `(expr1; expr2; expr3)` evaluate each expression from
  left to right and return the final value. Newlines after the opening `(` and
  between expressions are allowed, making them convenient for bundling setup
  work and a risky call into a single expression alongside Snail's `?`
  fallback operator.
- Tuple literals plus slicing use Python syntax: `(1, 2)`,
  `data[1:3]`, `data[:2]`, and `data[2:]`.

## Pipeline operator
Snail repurposes the `|` operator for generic data pipelining through
pipeline-aware callables. Any object can define how it consumes values from the
left-hand side by implementing `__call__`:

```snail
class Doubler {
    def __call__(self, x) {
        return x * 2
    }
}

result = 21 | Doubler()  # yields 42

# Use placeholders to control where piped values land in calls
greeting = "World" | greet("Hello ", _)  # greet("Hello ", "World")
excited = "World" | greet(_, "!")        # greet("World", "!")
formal = "World" | greet("Hello ", suffix=_)  # greet("Hello ", "World")
```

The pipeline operator has precedence between boolean operators and comparisons,
allowing natural chaining of transformations.

When piping into a call expression, the left-hand value is passed to the
callable result. If the call contains a single `_` placeholder, Snail substitutes
the piped value at that position (including keyword arguments). Only one
placeholder is allowed in a piped call. Outside of pipeline calls, `_` behaves
as a normal identifier.

## Functions, parameters, and calls
Define functions with braces instead of indentation:
```snail
def join_all(head, tail=0, *rest, **extras) {
    total = head + tail
    for n in rest { total = total + n }
    if "bonus" in extras { total = total + extras["bonus"] }
    return total
}

joined = join_all(1, 2, *values, **extras)
```
Default values, variadic `*args`, and `**kwargs` work as in Python. Calls accept
positional and keyword arguments interchangeably.

## Classes and methods
Classes use the same block style. Methods take `self` explicitly and interact
with Python's attribute model:
```snail
class Counter {
    def __init__(self, start) { self.start = start }
    def inc(self, step) { return self.start + step }
}

counter = Counter(10)
value = counter.inc(5)
```

## Control flow
`if`/`elif`/`else` blocks, `for`/`while` loops, and loop `else` clauses behave
like Python. `break` and `continue` are valid inside `try` blocks. Examples:
```snail
while i < 4 {
    try {
        if i == 1 { continue }
        elif i == 2 { break }
    } finally { cleanup() }
    i = i + 1
} else {
    loop_done = True
}
```

`if let` and `while let` bind destructured values in the condition, optionally
followed by a guard after a semicolon:
```snail
if let [user, domain] = pair; domain {
    print(domain)
}
```

## Comprehensions
List and dict comprehensions match Python's structure:
```snail
evens = [n for n in nums if n % 2 == 0]
lookup = {n: n * 2 for n in nums if n > 1}
```

## Strings
Single-line, raw, and triple-quoted strings are available. Raw strings keep
backslashes intact, and triple-quoted strings preserve newlines. Byte strings
use a `b` prefix (`b"..."`, `rb"..."`, `br"..."`) and produce Python `bytes`.
Byte strings interpolate `{}` like regular strings; interpolated byte strings
are UTF-8 encoded.

## Context managers
`with` uses the same protocol as Python and supports aliasing:
```snail
class SimpleCtx {
    def __enter__(self) { return "context" }
    def __exit__(self, exc_type, exc, tb) { return False }
}

with SimpleCtx() as message { ctx_msg = message }
```

## Exceptions and fallback expressions
Snail mirrors Python's exception handling and adds compact fallbacks:
- `<expr>?` swallows an exception and yields `None`. If the exception provides a
  `__fallback__` method, it will be called instead.
- `<expr>:<fallback>?` evaluates the fallback when `<expr>` raises, binding the
  exception object to `$e`.

The postfix `?` binds tightly to the expression on its left, before attributes,
calls, or other infix operators. For example, `a + risky():5?` evaluates as
`a + (risky():5?)`, and `boom():$e?.args[0]` accesses the exception produced by
`boom()`. To include additional infix operators in the fallback, wrap them in
parentheses; otherwise `a:0? + 1` parses as `(a:0?) + 1`.

Examples:
```snail
safe_value = risky()?                # returns None on failure
safe_fallback = risky():$e?          # returns exception object via $e
safe_details = risky():$e.args[0]?   # pull data from the exception
prefer_lambda = risky_fallback():"lambda"?
dunder_only = risky_fallback()?
```

## Regex expressions
Use regex literals for concise searches:

- `string in /<pattern>/` runs `re.search` and returns a tuple containing the
  full match followed by capture groups (`()` when there is no match), so
  truthiness checks work naturally.
- `/pattern/` alone produces a Snail regex object with a `search` method that
  returns the same tuple. You can also use `"value" in pattern` to return the
  same tuple (or `()` when there is no match).
- Regex literals are treated as raw strings and do not interpolate `{}`
  expressions, so backslashes stay intact.
- Escape `/` inside the pattern as `\/`.

In awk mode, regex patterns can stand alone. A bare `/pattern/` matches against
`$0` implicitly and binds the match tuple to `$m` for use inside the action
block.
Numeric group access is available via attribute shorthand: `$m.1` maps to
`$m[1]`.

## Containment hooks
Snail can delegate `in` checks to user-defined hooks:

- `left in right` calls `right.__snail_contains__(left)` when present and
  returns its value (truthiness is used for conditionals).
- `left not in right` calls `right.__snail_contains__(left)` when present and
  returns `not bool(result)` as a Python `bool`.
- When `__snail_contains__` is absent, Snail falls back to Python `in` and
  `not in` semantics.

## Subprocess expressions
Snail provides succinct subprocess helpers that work seamlessly with the pipeline
operator:
- `$(<command>)` runs the command, captures stdout, and returns it as a string.
  It raises on non-zero exit unless a fallback is provided.
- `@(<command>)` runs the command without capturing output and returns `0` on
  success. On failure it raises unless a fallback is specified; the injected
  `__fallback__` returns the process return code.

Both forms treat the command body as an f-string, so variables inside `{}` are
interpolated directly:
```snail
cmd_name = "snail"
echoed = $(echo {cmd_name})
status_ok = @(echo ready)
status_fail = @(false)?           # yields return code because of __fallback__
```

### Subprocess pipelines
Subprocess expressions are callables, enabling data to be piped to their stdin:
```snail
# Pipe string to command
result = "hello" | $(cat)         # "hello"

# Chain multiple commands
output = "foo\nbar" | $(grep foo) | $(wc -w)  # "1"

# Pipe any data type (auto-converted to string)
number = 42 | $(cat)              # "42"

# Works with @() too
"data" | @(cat > /tmp/file)       # writes to file, returns 0
```

When used standalone, subprocess expressions run with no stdin (current behavior).
When used on the right side of `|`, the left-hand value is piped to the command's
stdin.

## JSON queries
Snail provides first-class JSON support through JMESPath queries using the
`js()` function and `$[query]` structured pipeline accessor syntax.

```snail
# Parse JSON and query with $[jmespath]
json_obj = js(r'{"users": [{"name": "Alice"}, {"name": "Bob"}]}')
first_name = json_obj | $[users[0].name]  # yields "Alice"
names = json_obj | $[users[*].name]       # yields ["Alice", "Bob"]

# Inline parsing and querying
result = js(r'{"foo": 12}') | $[foo]    # yields 12
```

The `js()` function parses JSON strings (including JSONL) and returns a
queryable object. For JSONL input, the result is a list of parsed objects. The
`$[query]` accessor produces a callable that applies the JMESPath query to the
input data.

```snail
# JSONL input parses into a list
records = js('{"name": "Ada"}\n{"name": "Lin"}')
names = records | $[[*].name]  # yields ["Ada", "Lin"]
```

## Assertions and deletion
`assert` and `del` mirror Python. Assertions may include an optional message:
```snail
assert total == 6, "total"
del temp_value
```

## Awk mode
Invoke Snail's awk mode with `snail --awk`. Awk sources are composed of
pattern/action pairs evaluated for each input line. A rule with only a pattern
prints matching lines by default, and a lone block runs for every line.

Begin and end blocks can live in the source file (`BEGIN { ... }` / `END { ... }`) or be
specified via CLI flags:
- `-b <code>` or `--begin <code>`: Code to run before processing lines (repeatable)
- `-e <code>` or `--end <code>`: Code to run after processing lines (repeatable)
CLI BEGIN blocks run before in-file BEGIN blocks; CLI END blocks run after in-file END blocks.

Example:
```bash
echo "hello" | snail --awk --begin 'print("start")' --end 'print("done")' '{ print($0) }'
# Output: start\nhello\ndone
```

Multiple `-b`/`--begin` and `-e`/`--end` flags are processed in order. `BEGIN` and `END`
are reserved keywords in all modes. BEGIN/END blocks are regular Snail blocks, so
awk/map-only variables are not available inside them. See `examples/awk.snail`
for a runnable sample program.

While processing, Snail populates awk-style variables:
- `$0`: the current line with the trailing newline removed.
- `$1`, `$2`, ...: fields from `$0.split()` on whitespace.
- `$n`: global line counter across all files.
- `$fn`: per-file line counter.
- `$p`: the active filename, with `"-"` representing stdin.
- `$m`: the last regex match tuple (`$m.1` maps to `$m[1]`).

These `$` variables are injected by the language; user-defined identifiers
cannot start with `$`. They are only available in awk mode—using them in
regular Snail code requires `--awk`.

Input files come from `sys.argv[1:]`; when none are provided, awk mode reads
stdin. Pass `--` to the CLI to forward filenames or other arguments into the
Snail script.

## Interoperability
Snail code runs through Python's AST and execution engine. Functions, classes,
and modules exported from Snail are standard Python callables and namespaces, so
Snail and Python modules can import each other seamlessly.

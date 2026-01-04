# Snail language reference

Snail keeps Python's runtime and evaluation rules but swaps indentation-based
blocks for curly braces. The examples here mirror the runnable tour in
`examples/all_syntax.snail`.

## Running Snail code
- Run a one-liner: `snail "print('hi')"`
- Execute a file: `snail -f path/to/script.snail`
- Import from Python: `import demo.snail` works through the provided import hook
  and produces the same module objects as native Python code.

## Modules and imports
Snail uses Python's import semantics and exposes the same namespaces:
```snail
import math, sys as sysmod
from math import sqrt as root
```

## Statements and expressions
- Assignments mirror Python (`value = 1`). Multiple statements can be separated
  with semicolons.
- Boolean operators, comparisons, membership checks, and arithmetic follow
  Python's precedence and short-circuiting rules.
- Conditional expressions are supported: `fallback = "yes" if flag else "no"`.
- Compound expressions `(expr1; expr2; expr3)` evaluate each expression from
  left to right and return the final value. Newlines after the opening `(` and
  between expressions are allowed, making them convenient for bundling setup
  work and a risky call into a single expression alongside Snail's `?`
  fallback operator.
- Tuple and set literals plus slicing use Python syntax: `(1, 2)`, `{True, False}`,
  `data[1:3]`, `data[:2]`, and `data[2:]`.

## Pipeline operator
Snail repurposes the `|` operator for generic data pipelining through the
`__pipeline__` dunder method. Any object can define how it consumes values from
the left-hand side:

```snail
class Doubler {
    def __pipeline__(self, x) {
        return x * 2
    }
}

result = 21 | Doubler()  # yields 42
```

The pipeline operator has precedence between boolean operators and comparisons,
allowing natural chaining of transformations.

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

## Comprehensions
List and dict comprehensions match Python's structure:
```snail
evens = [n for n in nums if n % 2 == 0]
lookup = {n: n * 2 for n in nums if n > 1}
```

## Strings
Single-line, raw, and triple-quoted strings are available. Raw strings keep
backslashes intact, and triple-quoted strings preserve newlines.

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
- `<expr>?` swallows an exception and yields the exception object.
- `<expr> ? <fallback>` evaluates the fallback when `<expr>` raises, binding the
  exception object to `$e`.
- If an exception provides a `__fallback__` method, the fallback expression uses
  it when present.

The postfix `?` binds tightly to the expression on its left, before attributes,
calls, or other infix operators. For example, `a + risky() ? 5` evaluates as
`a + (risky() ? 5)`, and `boom()? .args[0]` accesses the exception produced by
`boom()`. To include additional infix operators in the fallback, wrap them in
parentheses; otherwise `a?0 + 1` parses as `(a?0) + 1`.

Examples:
```snail
safe_value = risky()?                # returns exception object on failure
safe_fallback = risky() ? $e         # returns exception object via $e
safe_details = risky() ? $e.args[0]  # pull data from the exception
prefer_lambda = risky_fallback() ? "lambda"
dunder_only = risky_fallback()?
```

## Regex expressions
Use regex literals for concise searches:

- `string in /<pattern>/` runs `re.search` and returns the match object (or
  `None`), so truthiness checks work naturally.
- `/pattern/` alone produces a compiled regex object you can reuse.
- Regex literals are treated as raw strings and do not interpolate `{}`
  expressions, so backslashes stay intact.
- Escape `/` inside the pattern as `\/`.

In awk mode, regex patterns can stand alone. A bare `/pattern/` matches against
`$l` implicitly and binds the match object to `$m` for use inside the action
block.

## Subprocess expressions
Snail provides succinct subprocess helpers:
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

## JSON queries
Snail provides first-class JSON support through JMESPath queries using the
`@j(<query>)` syntax. The query body is treated as a raw string without
interpolation:

```snail
# Query a Python dict directly
data = {"users": [{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}]}
names = data | @j(users[*].name)     # yields ["Alice", "Bob"]
first_name = data | @j(users[0].name)  # yields "Alice"

# Parse JSON from a string
json_result = "{{\"foo\": 12}}" | @j(foo)  # yields 12
```

The `@j()` construct implements `__pipeline__` and handles multiple input types:
- Python dicts and lists: queries them directly
- Strings: attempts JSON parsing first, falls back to treating as a file path
- File-like objects: reads and parses JSON content
- `None`: reads and parses JSON from stdin

This integrates naturally with Snail's pipeline operator for composable data
transformations.

## Assertions and deletion
`assert` and `del` mirror Python. Assertions may include an optional message:
```snail
assert total == 6, "total"
del temp_value
```

## Awk mode
Invoke Snail's awk mode with `snail --awk`. Awk sources are composed of
pattern/action pairs evaluated for each input line. `BEGIN` and `END` blocks
run before and after the line loop, a rule with only a pattern prints matching
lines by default, and a lone block runs for every line.

See `examples/awk.snail` for a runnable sample program.

While processing, Snail populates awk-style variables:
- `$l`: the current line with the trailing newline removed.
- `$f`: `$l.split()` on whitespace.
- `$n`: global line counter across all files.
- `$fn`: per-file line counter.
- `$p`: the active filename, with `"-"` representing stdin.
- `$m`: the last regex match object.

These `$` variables are injected by the language; user-defined identifiers
cannot start with `$`.

Input files come from `sys.argv[1:]`; when none are provided, awk mode reads
stdin. Pass `--` to the CLI to forward filenames or other arguments into the
Snail script.

## Interoperability
Snail code runs through Python's AST and execution engine. Functions, classes,
and modules exported from Snail are standard Python callables and namespaces, so
Snail and Python modules can import each other seamlessly.

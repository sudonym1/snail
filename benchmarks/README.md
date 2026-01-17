# Benchmarks

Startup timing harness for Snail. The default benchmark runs:

```
snail 'print("hello")'
```

## Usage

```
python benchmarks/startup.py
```

Use a specific command (everything after `--` is the command):

```
python benchmarks/startup.py -- uv run -- snail 'print("hello")'
```

Tune warmup/iterations:

```
python benchmarks/startup.py --warmup 10 --iterations 50
```

By default, output from the command is discarded. Use `--show-output` to see it.

## Python import profiling

```
python benchmarks/startup.py --profile-imports
```

When enabled, the script runs one profiled invocation before warmups/iterations
so import timing output is shown once.

## Rust profiling

Set `SNAIL_PROFILE_NATIVE=1` to emit Rust-side timing data to stderr:

```
SNAIL_PROFILE_NATIVE=1 uv run -- snail 'print("hello")'
```

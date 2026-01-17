#!/usr/bin/env python3
from __future__ import annotations

import argparse
import math
import os
import shlex
import statistics
import subprocess
import sys
import time


def _percentile(sorted_values: list[float], percentile: float) -> float:
    if not sorted_values:
        raise ValueError("no values to summarize")
    if percentile <= 0.0:
        return sorted_values[0]
    if percentile >= 1.0:
        return sorted_values[-1]
    index = (len(sorted_values) - 1) * percentile
    low = int(math.floor(index))
    high = int(math.ceil(index))
    if low == high:
        return sorted_values[low]
    weight = index - low
    return sorted_values[low] + (sorted_values[high] - sorted_values[low]) * weight


def _run_once(cmd: list[str], stdout, stderr, env: dict[str, str]) -> float:
    start = time.perf_counter()
    subprocess.run(cmd, check=True, stdout=stdout, stderr=stderr, env=env)
    end = time.perf_counter()
    return (end - start) * 1000.0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Benchmark Snail startup time by running a command repeatedly.",
    )
    parser.add_argument("--iterations", type=int, default=30)
    parser.add_argument("--warmup", type=int, default=5)
    parser.add_argument(
        "--show-output",
        action="store_true",
        help="Show command output instead of discarding it.",
    )
    parser.add_argument(
        "--profile-imports",
        action="store_true",
        help="Enable Python import timing via PYTHONPROFILEIMPORTTIME=1.",
    )
    parser.add_argument(
        "command",
        nargs=argparse.REMAINDER,
        help='Command to run; default: snail \'print("hello")\'',
    )
    args = parser.parse_args(argv)

    cmd = list(args.command)
    if cmd and cmd[0] == "--":
        cmd = cmd[1:]
    if not cmd:
        cmd = ["snail", 'print("hello")']
    if not cmd:
        parser.error("no command provided")

    if args.iterations <= 0:
        parser.error("--iterations must be positive")
    if args.warmup < 0:
        parser.error("--warmup cannot be negative")

    stdout = None if args.show_output else subprocess.DEVNULL
    stderr = None if args.show_output else subprocess.DEVNULL
    env = os.environ.copy()
    profile_env = None
    if args.profile_imports:
        profile_env = env.copy()
        profile_env["PYTHONPROFILEIMPORTTIME"] = "1"

    if profile_env is not None:
        _run_once(cmd, stdout=None, stderr=None, env=profile_env)

    for _ in range(args.warmup):
        _run_once(cmd, stdout=stdout, stderr=stderr, env=env)

    samples: list[float] = []
    for _ in range(args.iterations):
        samples.append(_run_once(cmd, stdout=stdout, stderr=stderr, env=env))

    samples.sort()
    mean = statistics.fmean(samples)
    median = statistics.median(samples)
    p95 = _percentile(samples, 0.95)
    stdev = statistics.pstdev(samples)

    print(f"command: {shlex.join(cmd)}")
    print(f"warmup: {args.warmup} iterations: {args.iterations}")
    if args.profile_imports:
        print("python_import_time: single run")
    print(f"mean: {mean:.3f} ms")
    print(f"median: {median:.3f} ms")
    print(f"p95: {p95:.3f} ms")
    print(f"stdev: {stdev:.3f} ms")
    print(f"min: {samples[0]:.3f} ms")
    print(f"max: {samples[-1]:.3f} ms")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

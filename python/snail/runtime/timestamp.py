from __future__ import annotations

from datetime import datetime, timedelta
from functools import total_ordering


@total_ordering
class SnailTimestamp:
    """A lightweight timestamp wrapper supporting arithmetic and comparisons."""

    __slots__ = ("dt",)

    def __init__(self, dt: datetime) -> None:
        self.dt = dt

    # -- Arithmetic --

    def __sub__(self, other: object) -> float | SnailTimestamp:
        if isinstance(other, SnailTimestamp):
            return (self.dt - other.dt).total_seconds()
        if isinstance(other, (int, float)):
            return SnailTimestamp(self.dt - timedelta(seconds=other))
        return NotImplemented

    def __add__(self, other: object) -> SnailTimestamp:
        if isinstance(other, (int, float)):
            return SnailTimestamp(self.dt + timedelta(seconds=other))
        return NotImplemented

    def __radd__(self, other: object) -> SnailTimestamp:
        if isinstance(other, (int, float)):
            return SnailTimestamp(self.dt + timedelta(seconds=other))
        return NotImplemented

    # -- Comparisons (total_ordering fills in the rest) --

    def __eq__(self, other: object) -> bool:
        if isinstance(other, SnailTimestamp):
            return self.dt == other.dt
        return NotImplemented

    def __lt__(self, other: object) -> bool:
        if isinstance(other, SnailTimestamp):
            return self.dt < other.dt
        return NotImplemented

    def __hash__(self) -> int:
        return hash(self.dt)

    # -- Display --

    def __str__(self) -> str:
        return self.dt.strftime("%Y-%m-%d %H:%M:%S")

    def __repr__(self) -> str:
        return f'ts("{self}")'

    # -- Convenience attributes --

    @property
    def year(self) -> int:
        return self.dt.year

    @property
    def month(self) -> int:
        return self.dt.month

    @property
    def day(self) -> int:
        return self.dt.day

    @property
    def hour(self) -> int:
        return self.dt.hour

    @property
    def minute(self) -> int:
        return self.dt.minute

    @property
    def second(self) -> int:
        return self.dt.second

    def timestamp(self) -> float:
        """Return the POSIX timestamp (epoch seconds) as a float."""
        return self.dt.timestamp()

    def format(self, fmt: str) -> str:
        """Format the timestamp using strftime."""
        return self.dt.strftime(fmt)


def ts(value: object = None) -> SnailTimestamp:
    """Create a SnailTimestamp.

    - ts()          -> current time
    - ts(string)    -> parse with dateutil
    - ts(number)    -> from epoch seconds
    - ts(datetime)  -> wrap existing datetime
    """
    if value is None:
        return SnailTimestamp(datetime.now())
    if isinstance(value, datetime):
        return SnailTimestamp(value)
    if isinstance(value, (int, float)):
        return SnailTimestamp(datetime.fromtimestamp(value))
    if isinstance(value, str):
        from dateutil.parser import parse

        return SnailTimestamp(parse(value))
    raise TypeError(
        f"ts() expects str, number, or datetime, got {type(value).__name__}"
    )

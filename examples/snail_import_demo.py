NAME = "snail"
ANSWER = 42


def greet(who: str) -> str:
    return f"hello {who}"


__all__ = ["NAME", "ANSWER", "greet"]

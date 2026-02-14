from __future__ import annotations

import shutil
import uuid
from pathlib import Path
from typing import Iterator

import pytest


@pytest.fixture
def tmp_path() -> Iterator[Path]:
    """Workspace-local tmp_path that avoids platform-specific temp ACL issues."""
    repo_root = Path(__file__).resolve().parents[2]
    tmp_root = repo_root / "target" / "pytest-tmp"
    tmp_root.mkdir(parents=True, exist_ok=True)
    case_dir = tmp_root / f"case-{uuid.uuid4().hex}"
    case_dir.mkdir()
    try:
        yield case_dir
    finally:
        shutil.rmtree(case_dir, ignore_errors=True)

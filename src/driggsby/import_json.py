"""Helpers for placeholder JSON import command."""

from dataclasses import dataclass
import json
from pathlib import Path


@dataclass(frozen=True, slots=True)
class ImportInput:
    source: str
    bytes_read: int


def read_json_input(file_path: Path | None, stdin_text: str | None) -> ImportInput:
    if file_path is not None:
        raw_input = file_path.read_text(encoding="utf-8")
        source = str(file_path)
    else:
        raw_input = stdin_text if stdin_text is not None else ""
        source = "stdin"

    stripped = raw_input.strip()
    if stripped:
        json.loads(stripped)

    return ImportInput(source=source, bytes_read=len(raw_input.encode("utf-8")))

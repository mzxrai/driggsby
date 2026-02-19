"""Domain models used by CLI placeholders."""

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class TransactionFilters:
    account: str | None
    category: str | None
    start: str | None
    end: str | None

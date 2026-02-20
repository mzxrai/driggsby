"""Dedupe helpers."""

import hashlib


def _normalize_text(value: str) -> str:
    return " ".join(value.strip().split()).lower()


def compute_dedupe_fingerprint(
    *,
    posted_date: str,
    amount_cents: int,
    currency: str,
    description: str,
    external_id: str | None,
) -> str:
    parts = (
        posted_date,
        str(amount_cents),
        currency.upper(),
        _normalize_text(description),
        external_id or "",
    )
    joined = "\x1f".join(parts)
    return hashlib.sha256(joined.encode("utf-8")).hexdigest()

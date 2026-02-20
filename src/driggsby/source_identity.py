"""Source/provider normalization helpers."""

import re

_NON_ALPHANUMERIC = re.compile(r"[^a-z0-9]+")
_MULTIPLE_UNDERSCORES = re.compile(r"_+")

_PROVIDER_ALIASES: dict[str, str] = {
    "apple_card": "apple_card",
    "applecard": "apple_card",
    "apple": "apple_card",
    "chase": "chase",
    "chase_bank": "chase",
    "jpmorgan": "jpmorgan",
    "jp_morgan": "jpmorgan",
    "j_p_morgan": "jpmorgan",
    "jp_morgan_chase": "jpmorgan",
    "vanguard": "vanguard",
    "fidelity": "fidelity",
    "amex": "amex",
    "american_express": "amex",
    "plaid": "plaid",
    "manual": "manual",
}


def _slugify(value: str) -> str:
    lowered = value.strip().lower()
    slug = _NON_ALPHANUMERIC.sub("_", lowered)
    slug = _MULTIPLE_UNDERSCORES.sub("_", slug)
    return slug.strip("_")


def normalize_source_provider(value: str) -> str:
    slug = _slugify(value)
    if not slug:
        return "other"
    return _PROVIDER_ALIASES.get(slug, "other")

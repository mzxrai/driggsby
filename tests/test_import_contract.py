from typing import Any, cast

from driggsby.dedupe import compute_dedupe_fingerprint
from driggsby.import_contract import build_dry_run_result
from driggsby.source_identity import normalize_source_provider


def _valid_payload() -> dict[str, object]:
    return {
        "source_provider": "Apple Card",
        "source_account_ref": "apple-card-1234",
        "transactions": [
            {
                "posted_date": "2026-01-10",
                "description": "Coffee Shop",
                "amount_cents": -650,
            }
        ],
    }


def test_build_dry_run_result_for_valid_payload() -> None:
    result = build_dry_run_result(_valid_payload())

    assert result.valid is True
    assert result.normalized_source_provider == "apple_card"
    assert result.source_account_ref == "apple-card-1234"
    assert result.transaction_count == 1
    assert result.errors == ()
    assert len(result.fingerprints) == 1


def test_build_dry_run_result_for_invalid_payload() -> None:
    payload = _valid_payload()
    payload.pop("source_account_ref")
    transactions = cast(list[dict[str, Any]], payload["transactions"])
    first_transaction = transactions[0]
    first_transaction["posted_date"] = "2026/01/10"
    first_transaction["description"] = " "
    first_transaction["currency"] = "US"

    result = build_dry_run_result(payload)

    assert result.valid is False
    assert result.normalized_source_provider is None
    assert result.source_account_ref is None
    assert result.transaction_count == 0
    assert result.errors

    error_paths = {error.path for error in result.errors}
    assert "source_account_ref" in error_paths
    assert "transactions[0].posted_date" in error_paths
    assert "transactions[0].description" in error_paths
    assert "transactions[0].currency" in error_paths


def test_normalize_source_provider_aliases() -> None:
    assert normalize_source_provider("Chase Bank") == "chase"
    assert normalize_source_provider("JP Morgan") == "jpmorgan"
    assert normalize_source_provider("j_p_morgan") == "jpmorgan"
    assert normalize_source_provider("Unknown New Provider") == "other"


def test_compute_dedupe_fingerprint_is_deterministic() -> None:
    same_a = compute_dedupe_fingerprint(
        posted_date="2026-01-10",
        amount_cents=-650,
        currency="USD",
        description="Coffee Shop",
        external_id=None,
    )
    same_b = compute_dedupe_fingerprint(
        posted_date="2026-01-10",
        amount_cents=-650,
        currency="USD",
        description="Coffee Shop",
        external_id=None,
    )
    different = compute_dedupe_fingerprint(
        posted_date="2026-01-10",
        amount_cents=-650,
        currency="USD",
        description="Bakery",
        external_id=None,
    )

    assert same_a == same_b
    assert same_a != different


def test_compute_dedupe_fingerprint_ignores_description_case() -> None:
    upper_case = compute_dedupe_fingerprint(
        posted_date="2026-01-10",
        amount_cents=-650,
        currency="USD",
        description="COFFEE SHOP",
        external_id=None,
    )
    lower_case = compute_dedupe_fingerprint(
        posted_date="2026-01-10",
        amount_cents=-650,
        currency="USD",
        description="coffee shop",
        external_id=None,
    )

    assert upper_case == lower_case

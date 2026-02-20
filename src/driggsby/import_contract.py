"""Import payload contract validation."""

from dataclasses import dataclass
from datetime import date
from typing import Any, Literal

from pydantic import BaseModel, ConfigDict, Field, ValidationError, field_validator

from driggsby.dedupe import compute_dedupe_fingerprint
from driggsby.source_identity import normalize_source_provider


@dataclass(frozen=True, slots=True)
class ValidationIssue:
    path: str
    message: str

    def to_dict(self) -> dict[str, str]:
        return {"path": self.path, "message": self.message}


@dataclass(frozen=True, slots=True)
class DryRunResult:
    valid: bool
    normalized_source_provider: str | None
    source_account_ref: str | None
    transaction_count: int
    errors: tuple[ValidationIssue, ...]
    fingerprints: tuple[str, ...]

    def to_dict(self) -> dict[str, Any]:
        return {
            "valid": self.valid,
            "normalized_source_provider": self.normalized_source_provider,
            "source_account_ref": self.source_account_ref,
            "transaction_count": self.transaction_count,
            "errors": [error.to_dict() for error in self.errors],
            "fingerprints": list(self.fingerprints),
        }


class TransactionPayloadModel(BaseModel):
    model_config = ConfigDict(extra="forbid")

    posted_date: date
    description: str
    amount_cents: int
    currency: str = "USD"
    settled_date: date | None = None
    merchant: str | None = None
    normalized_merchant: str | None = None
    category: str | None = None
    transaction_type: str | None = None
    status: str | None = None
    owner_name: str | None = None
    external_id: str | None = None
    metadata_json: dict[str, Any] | None = None

    @field_validator("description")
    @classmethod
    def validate_description(cls, value: str) -> str:
        stripped = value.strip()
        if not stripped:
            raise ValueError("must not be empty")
        return stripped

    @field_validator("currency")
    @classmethod
    def validate_currency(cls, value: str) -> str:
        normalized = value.strip().upper()
        if len(normalized) != 3 or not normalized.isalpha():
            raise ValueError("must be a 3-letter currency code")
        return normalized


class ImportPayloadModel(BaseModel):
    model_config = ConfigDict(extra="forbid")

    source_provider: str
    source_account_ref: str
    transactions: list[TransactionPayloadModel] = Field(min_length=1)
    source_name: str | None = None
    source_type: Literal["pdf", "csv", "json", "api"] | None = None
    parser_name: str | None = None
    parser_version: str | None = None
    period_start: date | None = None
    period_end: date | None = None
    metadata_json: dict[str, Any] | None = None

    @field_validator("source_provider", "source_account_ref")
    @classmethod
    def validate_non_empty_strings(cls, value: str) -> str:
        stripped = value.strip()
        if not stripped:
            raise ValueError("must not be empty")
        return stripped


def _format_error_path(path_parts: tuple[int | str, ...]) -> str:
    path = ""
    for part in path_parts:
        if isinstance(part, int):
            path += f"[{part}]"
            continue
        if path:
            path += "."
        path += part
    return path


def _validation_issues_from_error(
    error: ValidationError,
) -> tuple[ValidationIssue, ...]:
    issues: list[ValidationIssue] = []
    for detail in error.errors():
        location = detail.get("loc", ())
        path_parts: tuple[int | str, ...] = tuple(location)
        path = _format_error_path(path_parts)
        message = str(detail.get("msg", "Invalid value"))
        issues.append(ValidationIssue(path=path, message=message))
    return tuple(issues)


def build_dry_run_result(payload: object) -> DryRunResult:
    try:
        validated_payload = ImportPayloadModel.model_validate(payload)
    except ValidationError as error:
        return DryRunResult(
            valid=False,
            normalized_source_provider=None,
            source_account_ref=None,
            transaction_count=0,
            errors=_validation_issues_from_error(error),
            fingerprints=(),
        )

    normalized_provider = normalize_source_provider(validated_payload.source_provider)
    fingerprints = tuple(
        compute_dedupe_fingerprint(
            posted_date=transaction.posted_date.isoformat(),
            amount_cents=transaction.amount_cents,
            currency=transaction.currency,
            description=transaction.description,
            external_id=transaction.external_id,
        )
        for transaction in validated_payload.transactions
    )

    return DryRunResult(
        valid=True,
        normalized_source_provider=normalized_provider,
        source_account_ref=validated_payload.source_account_ref,
        transaction_count=len(validated_payload.transactions),
        errors=(),
        fingerprints=fingerprints,
    )

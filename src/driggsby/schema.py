"""Schema placeholders for MVP stubs."""

from pydantic import BaseModel, Field


class SchemaPlaceholder(BaseModel):
    toy: bool = True
    version: str = "0.1.0-dev"
    message: str = "Toy schema placeholder. Not production-ready."
    entities: list[str] = Field(default_factory=list)


def build_schema_placeholder() -> SchemaPlaceholder:
    return SchemaPlaceholder()

"""Structured error handling for fabio CLI.

Defines error codes and a base exception that commands use to signal
failures in a machine-readable way. Errors are caught at the top level
and rendered as structured JSON to stderr.
"""

from __future__ import annotations

from enum import Enum


class ErrorCode(str, Enum):
    """Machine-readable error codes for all fabio operations."""

    # Authentication
    AUTH_REQUIRED = "AUTH_REQUIRED"
    AUTH_EXPIRED = "AUTH_EXPIRED"
    AUTH_FAILED = "AUTH_FAILED"

    # API errors
    API_ERROR = "API_ERROR"
    NOT_FOUND = "NOT_FOUND"
    FORBIDDEN = "FORBIDDEN"
    RATE_LIMITED = "RATE_LIMITED"
    SERVER_ERROR = "SERVER_ERROR"
    CAPACITY_INACTIVE = "CAPACITY_INACTIVE"

    # Input validation
    INVALID_INPUT = "INVALID_INPUT"
    MISSING_PARAM = "MISSING_PARAM"
    CONFLICT = "CONFLICT"

    # Internal
    INTERNAL = "INTERNAL"
    TIMEOUT = "TIMEOUT"


class FabioError(Exception):
    """Base exception for all fabio errors.

    Carries a machine-readable code and human-readable message.
    Commands raise this; the top-level handler catches and renders.
    """

    def __init__(self, code: ErrorCode, message: str, *, status: int | None = None) -> None:
        super().__init__(message)
        self.code = code
        self.message = message
        self.status = status

    def to_dict(self) -> dict[str, object]:
        """Serialize to the standard error envelope."""
        d: dict[str, object] = {"code": self.code.value, "message": self.message}
        if self.status is not None:
            d["status"] = self.status
        return {"error": d}

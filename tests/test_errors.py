"""Tests for fabio.errors - structured error handling."""

from __future__ import annotations

from fabio.errors import ErrorCode, FabioError


class TestFabioError:
    def test_basic_error(self) -> None:
        err = FabioError(ErrorCode.AUTH_REQUIRED, "Not authenticated")
        assert str(err) == "Not authenticated"
        assert err.code == ErrorCode.AUTH_REQUIRED
        assert err.message == "Not authenticated"
        assert err.status is None

    def test_error_with_status(self) -> None:
        err = FabioError(ErrorCode.NOT_FOUND, "Workspace not found", status=404)
        assert err.status == 404

    def test_to_dict(self) -> None:
        err = FabioError(ErrorCode.API_ERROR, "Something failed", status=500)
        d = err.to_dict()
        assert d == {
            "error": {
                "code": "API_ERROR",
                "message": "Something failed",
                "status": 500,
            }
        }

    def test_to_dict_no_status(self) -> None:
        err = FabioError(ErrorCode.AUTH_REQUIRED, "Login required")
        d = err.to_dict()
        assert d == {
            "error": {
                "code": "AUTH_REQUIRED",
                "message": "Login required",
            }
        }

    def test_error_codes_are_strings(self) -> None:
        assert ErrorCode.AUTH_REQUIRED.value == "AUTH_REQUIRED"
        assert ErrorCode.NOT_FOUND.value == "NOT_FOUND"
        assert ErrorCode.RATE_LIMITED.value == "RATE_LIMITED"

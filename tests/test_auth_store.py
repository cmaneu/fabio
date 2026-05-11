"""Tests for fabio.auth_store - token persistence layer."""

from __future__ import annotations

import json
import time
from typing import TYPE_CHECKING

import pytest

from fabio.auth_store import AuthRecord, clear_record, load_record, save_record

if TYPE_CHECKING:
    from pathlib import Path


@pytest.fixture()
def auth_file(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """Redirect AUTH_FILE to a temp location for isolation."""
    fake = tmp_path / "auth.json"
    monkeypatch.setattr("fabio.auth_store.AUTH_FILE", fake)
    return fake


# -- AuthRecord -------------------------------------------------------------


class TestAuthRecord:
    def test_fields(self) -> None:
        rec = AuthRecord(
            username="user@contoso.com",
            tenant_id="tid",
            authority="https://login.microsoftonline.com/tid",
        )
        assert rec.username == "user@contoso.com"
        assert rec.tenant_id == "tid"
        assert rec.authority == "https://login.microsoftonline.com/tid"
        assert isinstance(rec.login_time, float)

    def test_age_human_seconds(self) -> None:
        rec = AuthRecord(username="u", tenant_id="t", authority="a", login_time=time.time() - 30)
        assert rec.age_human().endswith("s ago")

    def test_age_human_minutes(self) -> None:
        rec = AuthRecord(username="u", tenant_id="t", authority="a", login_time=time.time() - 120)
        assert "m ago" in rec.age_human()

    def test_age_human_hours(self) -> None:
        rec = AuthRecord(username="u", tenant_id="t", authority="a", login_time=time.time() - 7200)
        assert "h" in rec.age_human()


# -- Persistence helpers -----------------------------------------------------


class TestPersistence:
    def test_save_and_load(self, auth_file: Path) -> None:
        rec = AuthRecord(username="a@b.com", tenant_id="t1", authority="auth")
        save_record(rec)
        assert auth_file.exists()

        loaded = load_record()
        assert loaded is not None
        assert loaded.username == "a@b.com"
        assert loaded.tenant_id == "t1"

    def test_load_returns_none_when_missing(self, auth_file: Path) -> None:
        assert load_record() is None

    def test_load_returns_none_on_corrupt_json(self, auth_file: Path) -> None:
        auth_file.parent.mkdir(parents=True, exist_ok=True)
        auth_file.write_text("NOT JSON")
        assert load_record() is None

    def test_load_returns_none_on_missing_keys(self, auth_file: Path) -> None:
        auth_file.parent.mkdir(parents=True, exist_ok=True)
        auth_file.write_text(json.dumps({"unexpected": "data"}))
        assert load_record() is None

    def test_clear_removes_file(self, auth_file: Path) -> None:
        save_record(AuthRecord(username="u", tenant_id="t", authority="a"))
        assert auth_file.exists()
        assert clear_record() is True
        assert not auth_file.exists()

    def test_clear_returns_false_when_no_file(self, auth_file: Path) -> None:
        assert clear_record() is False

"""Tests for fabio.auth_store - token persistence layer."""

from __future__ import annotations

import json
import time
from typing import TYPE_CHECKING

import pytest

from fabio.auth_store import (
    AuthRecord,
    clear_record,
    load_azure_record,
    load_record,
    save_azure_record,
    save_record,
)

if TYPE_CHECKING:
    from pathlib import Path


@pytest.fixture()
def auth_files(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> tuple[Path, Path]:
    """Redirect auth files to a temp location for isolation."""
    fake_auth = tmp_path / "auth.json"
    fake_azure = tmp_path / "azure_auth_record.json"
    monkeypatch.setattr("fabio.auth_store.AUTH_FILE", fake_auth)
    monkeypatch.setattr("fabio.auth_store.AZURE_RECORD_FILE", fake_azure)
    return fake_auth, fake_azure


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


# -- Metadata record persistence ---------------------------------------------


class TestMetadataPersistence:
    def test_save_and_load(self, auth_files: tuple[Path, Path]) -> None:
        auth_file, _ = auth_files
        rec = AuthRecord(username="a@b.com", tenant_id="t1", authority="auth")
        save_record(rec)
        assert auth_file.exists()

        loaded = load_record()
        assert loaded is not None
        assert loaded.username == "a@b.com"
        assert loaded.tenant_id == "t1"

    def test_load_returns_none_when_missing(self, auth_files: tuple[Path, Path]) -> None:
        assert load_record() is None

    def test_load_returns_none_on_corrupt_json(self, auth_files: tuple[Path, Path]) -> None:
        auth_file, _ = auth_files
        auth_file.parent.mkdir(parents=True, exist_ok=True)
        auth_file.write_text("NOT JSON")
        assert load_record() is None

    def test_load_returns_none_on_missing_keys(self, auth_files: tuple[Path, Path]) -> None:
        auth_file, _ = auth_files
        auth_file.parent.mkdir(parents=True, exist_ok=True)
        auth_file.write_text(json.dumps({"unexpected": "data"}))
        assert load_record() is None


# -- Azure AuthenticationRecord persistence -----------------------------------


class TestAzureRecordPersistence:
    def test_save_and_load(self, auth_files: tuple[Path, Path]) -> None:
        _, azure_file = auth_files
        save_azure_record('{"serialized": "record"}')
        assert azure_file.exists()

        loaded = load_azure_record()
        assert loaded == '{"serialized": "record"}'

    def test_load_returns_none_when_missing(self, auth_files: tuple[Path, Path]) -> None:
        assert load_azure_record() is None

    def test_load_returns_none_on_empty(self, auth_files: tuple[Path, Path]) -> None:
        _, azure_file = auth_files
        azure_file.parent.mkdir(parents=True, exist_ok=True)
        azure_file.write_text("")
        assert load_azure_record() is None


# -- Cleanup ------------------------------------------------------------------


class TestClearRecord:
    def test_clear_removes_both_files(self, auth_files: tuple[Path, Path]) -> None:
        auth_file, azure_file = auth_files
        save_record(AuthRecord(username="u", tenant_id="t", authority="a"))
        save_azure_record("data")
        assert auth_file.exists()
        assert azure_file.exists()

        assert clear_record() is True
        assert not auth_file.exists()
        assert not azure_file.exists()

    def test_clear_removes_only_existing(self, auth_files: tuple[Path, Path]) -> None:
        auth_file, _ = auth_files
        save_record(AuthRecord(username="u", tenant_id="t", authority="a"))

        assert clear_record() is True
        assert not auth_file.exists()

    def test_clear_returns_false_when_no_files(self, auth_files: tuple[Path, Path]) -> None:
        assert clear_record() is False

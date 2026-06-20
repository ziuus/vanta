"""Tests for process_service — kernel filtering, sorting, signal helpers."""
import threading
from unittest.mock import patch

import pytest

from monitor.core.models import ProcessRow
from monitor.core.process_service import (
    KERNEL_PREFIXES,
    ProcessService,
    looks_like_kernel,
    next_sort_column,
    sort_key_for,
)


class TestLooksLikeKernel:
    def test_prefix_filter_rejects_kworker(self):
        assert looks_like_kernel("kworker/0:0") is True

    def test_prefix_filter_rejects_ksoftirqd(self):
        assert looks_like_kernel("ksoftirqd/0") is True

    def test_default_filter_allows_python(self):
        assert looks_like_kernel("python3") is False

    def test_default_filter_allows_firefox(self):
        assert looks_like_kernel("firefox") is False

    def test_slash_makes_it_kernel(self):
        assert looks_like_kernel("irq/128-iwlwifi") is True

    def test_systemd_is_not_filtered(self):
        assert looks_like_kernel("systemd") is False

    def test_every_prefix_is_lowercase(self):
        bad = [p for p in KERNEL_PREFIXES if p != p.lower()]
        assert not bad, f"Prefixes should be lowercase: {bad}"

    def test_every_prefix_starts_without_slash(self):
        bad = [p for p in KERNEL_PREFIXES if p.startswith("/")]
        assert not bad, f"Prefixes should not start with slash: {bad}"


class TestSortKeyFor:
    @pytest.fixture
    def a_row(self):
        return ProcessRow(
            pid=100,
            name="python3",
            cpu_percent=42.5,
            memory_percent=12.3,
            status="running",
            threads=4,
            username="zius",
        )

    def test_sort_by_cpu_uses_cpu_percent(self, a_row):
        fn = sort_key_for("cpu")
        assert fn(a_row) == 42.5

    def test_sort_by_mem_uses_memory_percent(self, a_row):
        fn = sort_key_for("mem")
        assert fn(a_row) == 12.3

    def test_sort_by_pid(self, a_row):
        fn = sort_key_for("pid")
        assert fn(a_row) == 100

    def test_sort_by_threads(self, a_row):
        fn = sort_key_for("threads")
        assert fn(a_row) == 4

    def test_sort_by_name(self, a_row):
        fn = sort_key_for("name")
        assert fn(a_row) == "python3"

    def test_next_sort_column_cycles_through_operator_order(self):
        assert next_sort_column("cpu") == "memory"
        assert next_sort_column("memory") == "pid"
        assert next_sort_column("pid") == "threads"
        assert next_sort_column("threads") == "name"
        assert next_sort_column("name") == "cpu"


class TestProcessServiceUnit:
    def test_terminate_raises_on_bad_pid(self):
        svc = ProcessService()
        with pytest.raises(Exception):
            svc.terminate_process(999999999)

    def test_list_processes_returns_at_least_one(self):
        svc = ProcessService()
        rows = svc.list_processes(include_kernel=False, limit=5)
        assert len(rows) > 0
        assert all(isinstance(r, ProcessRow) for r in rows)

    def test_list_processes_with_query_filters(self):
        svc = ProcessService()
        rows = svc.list_processes(query="python", limit=5)
        if rows:
            assert "python" in rows[0].name.lower()

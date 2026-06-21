"""Tests for the Flask web dashboard and API surface."""

from __future__ import annotations

import psutil

from monitor.core.models import ProcessRow
from monitor.server import app


class _FakeSnapshot:
    class CPU:
        total_percent = 12.5
        load_avg_1m = 1.2
        frequency_mhz = 4200.0
        core_count = 16

    class Memory:
        percent = 43.0

    class Network:
        upload_bps = 1024.0
        download_bps = 2048.0

    cpu = CPU()
    memory = Memory()
    network = Network()
    disks = [type("Disk", (), {"percent": 61.0})()]
    temperature_c = 55.0
    uptime_seconds = 12345.0
    process_count = 321
    thread_count = 654


def test_health_endpoint_reports_ok() -> None:
    client = app.test_client()
    response = client.get("/api/health")
    assert response.status_code == 200
    payload = response.get_json()
    assert payload["status"] == "ok"
    assert payload["surface"] == "web"


def test_stats_endpoint_returns_snapshot_payload(monkeypatch) -> None:
    client = app.test_client()
    monkeypatch.setattr("monitor.server.collector.sample", lambda: _FakeSnapshot())
    response = client.get("/api/stats")
    assert response.status_code == 200
    payload = response.get_json()
    assert payload["cpu"] == 12.5
    assert payload["mem"] == 43.0
    assert payload["disk"] == 61.0
    assert payload["net_up"] == 1024.0
    assert payload["processes"] == 321


def test_processes_endpoint_clamps_bad_limit_and_supports_filters(monkeypatch) -> None:
    client = app.test_client()
    seen: dict[str, object] = {}

    def fake_list_processes(**kwargs):
        seen.update(kwargs)
        return [
            ProcessRow(
                pid=101,
                name="python",
                cpu_percent=17.2,
                memory_percent=2.3,
                status="running",
                threads=5,
                username="zius",
            )
        ]

    monkeypatch.setattr("monitor.server.process_service.list_processes", fake_list_processes)
    response = client.get("/api/processes?limit=bogus&sort=wat&include_kernel=yes&ascending=1&username=zius&query=python")
    assert response.status_code == 200
    payload = response.get_json()
    assert payload[0]["name"] == "python"
    assert seen["limit"] == 15
    assert seen["sort_by"] == "cpu"
    assert seen["include_kernel"] is True
    assert seen["descending"] is False
    assert seen["username"] == "zius"
    assert seen["query"] == "python"


def test_process_detail_endpoint_returns_404_on_missing_process(monkeypatch) -> None:
    client = app.test_client()
    monkeypatch.setattr(
        "monitor.server.process_service.get_process_detail",
        lambda pid: {"pid": pid, "error": "gone", "name": "<gone>"},
    )
    response = client.get("/api/process/999999")
    assert response.status_code == 404
    payload = response.get_json()
    assert payload["error"] == "gone"


def test_process_action_endpoint_maps_access_denied(monkeypatch) -> None:
    client = app.test_client()

    def boom(pid: int):
        raise psutil.AccessDenied(pid=pid)

    monkeypatch.setattr("monitor.server.process_service.terminate_process", boom)
    response = client.post("/api/process/55/kill")
    assert response.status_code == 403
    payload = response.get_json()
    assert payload["success"] is False
    assert "denied" in payload["message"].lower()

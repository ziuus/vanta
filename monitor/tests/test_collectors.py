from monitor.core.collectors import SystemCollector


def test_system_collector_returns_snapshot_with_expected_sections():
    snapshot = SystemCollector().sample()

    assert snapshot.cpu.core_count >= 1
    assert snapshot.memory.total_bytes > 0
    assert snapshot.network.bytes_recv >= 0
    assert snapshot.process_count >= 1
    assert isinstance(snapshot.disks, list)

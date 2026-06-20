from monitor.core.history import HistoryBuffer


def test_history_buffer_keeps_latest_values_only():
    buf = HistoryBuffer(size=3)
    for value in [10, 20, 30, 40]:
        buf.push(value)

    assert buf.values() == [20.0, 30.0, 40.0]
    assert buf.latest() == 40.0


def test_history_buffer_empty_when_no_values():
    buf = HistoryBuffer(size=10)
    assert buf.values() == []
    assert buf.latest() is None

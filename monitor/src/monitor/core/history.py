from collections import deque


class HistoryBuffer:
    def __init__(self, size: int = 120):
        self._values = deque(maxlen=size)

    def push(self, value: float) -> None:
        self._values.append(float(value))

    def values(self) -> list[float]:
        return list(self._values)

    def latest(self) -> float | None:
        return self._values[-1] if self._values else None

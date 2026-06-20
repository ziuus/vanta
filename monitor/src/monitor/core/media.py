"""Media playback detection via playerctl."""

import shutil
import subprocess
from typing import Any


class MediaDetector:
    """Detect what media is currently playing via playerctl/MPRIS."""

    def __init__(self) -> None:
        self._has_playerctl = shutil.which("playerctl") is not None

    def detect(self) -> dict[str, Any] | None:
        """Return current playback info or *None* if nothing playing."""
        if not self._has_playerctl:
            return None

        try:
            title = subprocess.run(
                ["playerctl", "metadata", "title"],
                capture_output=True, text=True, timeout=1,
            ).stdout.strip()
            artist = subprocess.run(
                ["playerctl", "metadata", "artist"],
                capture_output=True, text=True, timeout=1,
            ).stdout.strip()
            album = subprocess.run(
                ["playerctl", "metadata", "album"],
                capture_output=True, text=True, timeout=1,
            ).stdout.strip()
            status = subprocess.run(
                ["playerctl", "status"],
                capture_output=True, text=True, timeout=1,
            ).stdout.strip()

            if not title and not artist:
                return None

            return {
                "title": title or "Unknown",
                "artist": artist or "Unknown",
                "album": album or "",
                "status": status or "Playing",
            }
        except (subprocess.TimeoutExpired, FileNotFoundError):
            return None

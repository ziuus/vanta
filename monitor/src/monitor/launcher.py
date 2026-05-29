#!/usr/bin/env python3
"""
Vanta Monitor Launcher — start the TUI and/or web dashboard.
Usage: vanta-monitor [tui|web|both]
"""
import sys
import os
import subprocess
from pathlib import Path

MONITOR_DIR = Path(__file__).parent


def run_mode(mode: str):
    if mode == "web":
        print("Starting Web Dashboard on http://localhost:5000 ...")
        subprocess.run([sys.executable, str(MONITOR_DIR / "server.py")])
    elif mode == "tui":
        print("Starting Vanta Monitor TUI...")
        subprocess.run([sys.executable, str(MONITOR_DIR / "app.py")])
    elif mode == "both":
        print("Starting both Web + TUI...")
        web = subprocess.Popen([sys.executable, str(MONITOR_DIR / "server.py")])
        import time
        time.sleep(2)
        subprocess.run([sys.executable, str(MONITOR_DIR / "app.py")])
        web.kill()
    else:
        print("Usage: python3 launcher.py [tui|web|both]")


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "tui"
    run_mode(mode)


if __name__ == "__main__":
    main()

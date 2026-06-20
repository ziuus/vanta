from __future__ import annotations

import os
import stat
from datetime import datetime
from pathlib import Path

from textual.app import ComposeResult
from textual.screen import Screen
from textual.widgets import Footer, Header, Static, ListView, ListItem
from textual.binding import Binding
from textual.containers import Horizontal, Vertical

from monitor.core.theme import DARK, LIGHT


def _fmt_size(size: int) -> str:
    if size < 1024:
        return f"{size}B"
    if size < 1024**2:
        return f"{size/1024:.1f}K"
    if size < 1024**3:
        return f"{size/1024**2:.1f}M"
    return f"{size/1024**3:.1f}G"


def _fmt_time(ts: float) -> str:
    return datetime.fromtimestamp(ts).strftime("%Y-%m-%d %H:%M")


def _icon_for(path: Path) -> str:
    """Simple icon based on file type."""
    if path.is_dir():
        return "📁"
    if path.is_symlink():
        return "🔗"
    name = path.name.lower()
    for ext, icon in [
        (".py", "🐍"),
        (".rs", "🦀"),
        (".js", "📜"),
        (".ts", "📘"),
        (".html", "🌐"),
        (".css", "🎨"),
        (".json", "📋"),
        (".yaml", "📋"),
        (".yml", "📋"),
        (".toml", "⚙"),
        (".md", "📝"),
        (".txt", "📄"),
        (".jpg", "🖼"),
        (".jpeg", "🖼"),
        (".png", "🖼"),
        (".gif", "🖼"),
        (".webp", "🖼"),
        (".svg", "🖼"),
        (".mp3", "🎵"),
        (".flac", "🎵"),
        (".wav", "🎵"),
        (".mp4", "🎬"),
        (".mkv", "🎬"),
        (".mov", "🎬"),
        (".zip", "📦"),
        (".tar", "📦"),
        (".gz", "📦"),
        (".xz", "📦"),
        (".bz2", "📦"),
        (".7z", "📦"),
        (".pdf", "📕"),
        (".epub", "📕"),
        (".deb", "📦"),
        (".rpm", "📦"),
        (".AppImage", "📦"),
        (".sh", "⚡"),
        (".fish", "⚡"),
        (".exe", "⚙"),
    ]:
        if name.endswith(ext):
            return icon
    if os.access(path, os.X_OK) and not path.is_dir():
        return "⚙"
    return "📄"


class FileManagerScreen(Screen):
    """Keyboard-driven file browser: navigate, preview, open."""

    BINDINGS = [
        Binding("j", "cursor_down", "Down", show=False),
        Binding("k", "cursor_up", "Up", show=False),
        Binding("l", "enter_dir", "Enter", show=False),
        Binding("h", "parent_dir", "Back", show=False),
        Binding("/", "search", "Search", show=False),
        Binding("g", "top", "Top", show=False),
        Binding("G", "bottom", "Bottom", show=False),
        Binding("q", "dismiss", "Close", show=False),
        Binding("escape", "dismiss", "Close", show=True),
        Binding("~", "home_dir", "Home", show=False),
    ]

    def __init__(self, start_dir: str | None = None):
        super().__init__()
        self._cwd = Path(os.path.expanduser(start_dir or "~")).resolve()
        self._entries: list[Path] = []
        self._cursor = 0
        self._theme_name = "light"

    @property
    def pal(self) -> dict[str, str]:
        return LIGHT if self._theme_name == "light" else DARK

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        with Horizontal(id="fm-body"):
            with Vertical(id="fm-list-col"):
                yield Static(id="fm-path", classes="fm-path")
                yield ListView(id="fm-list")
            with Vertical(id="fm-preview-col"):
                yield Static(id="fm-preview", classes="fm-preview")
        yield Footer()

    def on_mount(self):
        self._dom_ready = True
        self._load_dir()
        self._update_preview()

    def _load_dir(self):
        p = self.pal
        try:
            entries = sorted(
                self._cwd.iterdir(),
                key=lambda e: (not e.is_dir(), e.name.lower()),
            )
        except PermissionError:
            entries = []
        self._entries = entries
        self._cursor = 0

        self.query_one("#fm-path", Static).update(
            f"[{p['accent']}]{self._cwd}[/]"
        )

        list_view = self.query_one("#fm-list", ListView)
        list_view.clear()
        items = []
        for entry in entries[:200]:  # cap at 200
            icon = _icon_for(entry)
            name = entry.name[:40]
            if entry.is_dir():
                name += "/"
            try:
                st = entry.stat()
            except OSError:
                st = None
            if st:
                size = _fmt_size(st.st_size)
                mtime = _fmt_time(st.st_mtime)
                label = (
                    f"{icon} [{p['text']}]{name:<42}[/]"
                    f"[{p['text_dim']}]{size:>6}  {mtime}[/]"
                )
            else:
                label = f"{icon} [{p['text']}]{name:<42}[/]"
            items.append(ListItem(Static(label)))
        list_view.extend(items)

    def _update_preview(self):
        p = self.pal
        preview_widget = self.query_one("#fm-preview", Static)
        if not self._entries:
            preview_widget.update(f"[{p['text_dim']}]empty directory[/]")
            return
        idx = min(self._cursor, len(self._entries) - 1)
        target = self._entries[idx]
        if target.is_dir():
            try:
                sub = list(target.iterdir())
                total = len(sub)
                dirs = sum(1 for e in sub if e.is_dir())
                files = total - dirs
                preview_widget.update(
                    f"[{p['accent']}]Directory[/]\n"
                    f"[{p['text']}]{target.name}[/]\n"
                    f"[{p['text_muted']}]{dirs} dirs  {files} files[/]"
                )
            except PermissionError:
                preview_widget.update(f"[{p['red']}]Permission denied[/]")
        else:
            try:
                st = target.stat()
                preview_widget.update(
                    f"[{p['accent']}]File[/]\n"
                    f"[{p['text']}]{target.name}[/]\n"
                    f"[{p['text_muted']}]Size: {_fmt_size(st.st_size)}[/]\n"
                    f"[{p['text_muted']}]Modified: {_fmt_time(st.st_mtime)}[/]"
                )
            except OSError:
                preview_widget.update(f"[{p['red']}]Cannot stat[/]")

    def action_cursor_down(self):
        if self._entries and self._cursor < len(self._entries) - 1:
            self._cursor += 1
            self._update_preview()
            list_view = self.query_one("#fm-list", ListView)
            if self._cursor < len(list_view.children):
                list_view.index = self._cursor

    def action_cursor_up(self):
        if self._cursor > 0:
            self._cursor -= 1
            self._update_preview()
            list_view = self.query_one("#fm-list", ListView)
            if self._cursor < len(list_view.children):
                list_view.index = self._cursor

    def action_enter_dir(self):
        if not self._entries:
            return
        idx = min(self._cursor, len(self._entries) - 1)
        target = self._entries[idx]
        if target.is_dir():
            self._cwd = target.resolve()
            self._load_dir()
            self._update_preview()

    def action_parent_dir(self):
        parent = self._cwd.parent
        if parent != self._cwd:
            old_name = self._cwd.name
            self._cwd = parent
            self._load_dir()
            # Try to restore cursor to the directory we came from
            for i, e in enumerate(self._entries):
                if e.name == old_name:
                    self._cursor = i
                    break
            self._update_preview()

    def action_top(self):
        self._cursor = 0
        self._update_preview()
        list_view = self.query_one("#fm-list", ListView)
        if list_view.children:
            list_view.index = 0

    def action_bottom(self):
        if self._entries:
            self._cursor = len(self._entries) - 1
            self._update_preview()
            list_view = self.query_one("#fm-list", ListView)
            if list_view.children:
                list_view.index = len(list_view.children) - 1

    def action_home_dir(self):
        self._cwd = Path.home()
        self._load_dir()
        self._update_preview()

    def action_dismiss(self) -> None:
        self.app.push_screen("overview")

    def apply_theme(self, theme: str) -> None:
        self._theme_name = theme
        is_light = theme == "light"
        if is_light:
            self.add_class("vanta-light")
        else:
            self.remove_class("vanta-light")
        if hasattr(self, "_dom_ready") and self._dom_ready:
            self._load_dir()
            self._update_preview()

    CSS = """
    #fm-body {
        padding: 0 1;
    }
    #fm-list-col {
        width: 2fr;
        margin-right: 1;
    }
    .fm-path {
        height: 1;
        color: #06b6d4;
        text-style: bold;
        margin-bottom: 1;
    }
    #fm-list {
        height: 1fr;
        border: solid #1e1e3f;
        background: #0f0f1a;
    }
    #fm-preview-col {
        width: 1fr;
    }
    .fm-preview {
        height: 1fr;
        border: solid #1e1e3f;
        background: #0f0f1a;
        padding: 1;
        color: #cbd5e1;
    }
    ListView {
        background: #0f0f1a;
    }
    ListView > ListItem {
        padding: 0 1;
        color: #cbd5e1;
    }
    ListView > ListItem:hover {
        background: #1e1e3f;
    }
    ListView > ListItem.--highlight {
        background: #1e1e3f;
    }

    /* Light theme */
    .vanta-light .fm-preview,
    .vanta-light #fm-list {
        border: solid #d1d5db;
        background: #ffffff;
        color: #1a1a1a;
    }
    .vanta-light ListView {
        background: #ffffff;
    }
    .vanta-light ListView > ListItem {
        color: #1a1a1a;
    }
    .vanta-light ListView > ListItem:hover {
        background: #e5e7eb;
    }
    .vanta-light ListView > ListItem.--highlight {
        background: #e5e7eb;
    }
    """

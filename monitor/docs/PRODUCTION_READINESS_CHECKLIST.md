# Production Readiness Checklist

Verified on this pass for /home/zius/Projects/vanta/monitor.

## Setup
- [x] Project builds into sdist and wheel with `uv build`
- [x] CLI entrypoints exist for TUI, web, and combined launch
- [x] Config defaults load cleanly when `config.json` is present

## Build / Packaging
- [x] `uv build` succeeds
- [x] `uv run pytest -q` succeeds
- [x] Test suite covers CLI, collectors, presenters, process service, screens, and web API

## TUI operator loop
- [x] Overview screen shows current state fast
- [x] Process list supports move selection up/down
- [x] Process list supports search/filter
- [x] Process list supports sort and reverse-sort workflows
- [x] Process list supports user/kernel scope toggles
- [x] Process detail modal exists
- [x] Safe signal/action workflow exists
- [x] Theme switching is focus-safe under Textual test pilot
- [x] Dedicated graphs screen exists
- [x] File browser screen exists
- [x] Layout degrades through full / compact / tiny modes

## Web API surface
- [x] Health endpoint exists at `/api/health`
- [x] Stats endpoint returns JSON snapshot data
- [x] Process listing endpoint supports query/sort/filter parameters
- [x] Process detail endpoint exists
- [x] Process action endpoints map access-denied / missing-process states cleanly

## Input validation / safety
- [x] Bad process list limit values are clamped instead of crashing
- [x] Unknown sort values fall back to a safe default
- [x] Process actions use shared service logic instead of ad hoc endpoint code
- [x] Process detail environment preview redacts secret-like keys

## Documentation honesty
- [x] README describes the implemented screens and keybinds
- [x] README lists the verified API endpoints
- [x] Architecture doc reflects actual runtime surfaces
- [x] Placeholder TODO copy was removed from shipped config defaults

## Live verification
- [x] Web server was started and queried successfully
- [x] `/api/health` returned 200 with `{"status":"ok"...}`
- [x] `/api/stats` returned live machine data
- [x] `/api/processes?limit=3&query=python` returned filtered process rows
- [x] Textual test pilot previously verified screen switching, search, user filter, tree view, and theme/preset changes

## Remaining high-priority blockers
- [x] None identified in this pass

## Known non-blockers / scope limits
- [ ] This is not a full btop clone in breadth
- [ ] Unicode sparkline graphs are still simpler than a native high-density graph renderer
- [ ] Web dashboard is secondary to the TUI and not as deep as the TUI workflow

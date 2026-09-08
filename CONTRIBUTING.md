# Contributing to vanta

## Setup

```bash
# libdbus is needed for MPRIS media control
sudo pacman -S dbus            # Arch
sudo apt install libdbus-1-dev # Debian/Ubuntu

git clone https://github.com/ziuus/vanta && cd vanta
cargo run --release
```

Optional runtime tools that light up extra panels: `cava` (visualizer),
`nmcli` (wifi), `docker`, `checkupdates` (Arch), `nvidia-smi`.

## Before opening a PR

CI runs exactly this; make it pass locally first:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## How the code is organised

- `src/monitors/` — data collection. Each module owns a `static` snapshot and a
  `sample()` that runs on the **sampler thread** (`monitors/mod.rs`). Never do
  I/O in a `render()`; read the snapshot instead. Do the slow work *before*
  taking the lock so the UI never waits on it.
- `src/widgets/` — pure drawing helpers (clock glyphs, gauges, graphs, meters).
- `src/screens/` — one file per page; `screens::panel()` is the shared chrome.
- `src/app.rs` — state, key handling, title/status bars.
- `src/theme.rs` — palettes. Add a theme by adding a constructor and its name
  to `THEME_NAMES`.

## Conventions

- Panels must degrade, not overflow: check `area.width/height` and return
  early or drop detail when tight. Test at 100×34 and 200×50 (`tmux -x -y`).
- Truncate strings with `meter::ellipsize` (char-safe), never byte slicing.
- Colours come from the theme (`theme.usage(pct)`, `theme.temp(c)`), not
  literals.
- Commit messages: imperative subject, body explains *why*.

## Releasing

Publishing is tag-driven: `.github/workflows/release.yml` builds the linux-x64
binary, attaches it to a GitHub release, and publishes the npm wrapper.

```bash
# 1. bump the version in Cargo.toml, then refresh the lockfile
cargo build --release
git commit -am "Release v0.3.0"

# 2. tag it — this is what triggers everything
git tag v0.3.0 && git push origin main --tags
```

The workflow syncs `npm/package.json` to the tag, so the version lives in
`Cargo.toml` only. `npm/install.js` derives its download URL from that version,
which is why the tag must be `v<version>`.

One-time setup: add an npm automation token as the `NPM_TOKEN` repository
secret. Without it the release still publishes, and the npm job logs a warning
and skips.

The npm package is `@ziuus/vanta` — both `vanta` and `vanta-tui` were already
taken, and a scope is the one namespace nobody can race us for. It ships only a
Node shim plus `install.js`; the native binary is downloaded from the release on
`postinstall`. Scoped packages default to restricted, so the publish needs
`--access public`.

The shim installs one command, `vanta`. A `vtui` alias was tried and reverted:
`vtui` is already an npm package and already a pip console script, and when a
stale copy of either shadows ours the user gets someone else's traceback and
files the bug against us. Bin names are cheap to add and expensive to collide.

`npm/package.json`'s `prepack` script copies `README.md` and `LICENSE` into
`npm/` and rewrites the screenshot paths to raw.githubusercontent URLs, so both
CI and a manual `npm publish` get the full package. Those two copies are
gitignored — they are build output, not source.

By contributing you agree your work is licensed under the MIT License.

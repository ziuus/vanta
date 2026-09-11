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
  taking the lock so the UI never waits on it. A TTL cache is not enough for an
  *unbounded* wait — anything that shells out to the network (`checkupdates`,
  `nmcli`) gets its own thread, or it stalls every other monitor behind it and
  `Summary` (assigned last) never lands. That shipped once as `cpu 0%` in the
  title bar; `monitors::facts_thread` is the pattern.
- `src/widgets/` — pure drawing helpers (clock glyphs, gauges, graphs, meters).
- `src/custom/` — user-defined widgets from `[[custom_widgets]]` in the config.
  Each enabled widget gets its own `DataWorker` thread (`source.rs`) that polls a
  command or file and writes a `FetchResult` the renderer only reads — never the
  sampler thread, so a user's slow command can't stall telemetry. Commands run
  through `Command::new` with an argv split, **never** `sh -c`: a config file is
  not a trust boundary you can hand a shell. Timeouts must `wait()` after
  `kill()` or the child is left a zombie.
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

`node npm/sync-version.js` copies the version from `Cargo.toml` into
`npm/package.json`, so the version lives in `Cargo.toml` only. Pass the expected
version (`sync-version.js 0.3.0`) and it fails instead of publishing a mismatch
— that is how the release workflow checks the tag. `npm/install.js` derives its
download URL from that version, which is why the tag must be `v<version>`.

**Order matters.** The npm package's `postinstall` downloads the binary from the
GitHub release, so the release has to exist *before* the npm publish. Tag, let
the `binary` job finish, then publish. Publishing first ships a package whose
every install 404s.

If you publish by hand, use the script — never a bare `npm publish`:

```bash
npm --prefix npm run release   # sync-version.js, then npm publish --access public
```

The sync cannot live in `prepack`: npm reads the manifest *before* running it, so
a version rewritten there is packed under the old number while the tarball's own
`package.json` carries the new one. That mismatch is what made a manual 0.2.2
publish fail with `403 cannot publish over 0.2.1`. `prepack` is still the right
hook for file *contents* — see below.

One-time setup: add an npm automation token as the `NPM_TOKEN` repository
secret. Without it the release still publishes the binary, and the npm job logs
a warning and skips — which means every publish stays manual.

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

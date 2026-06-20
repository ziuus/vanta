"""Entry points for vanta-monitor CLI commands."""

import argparse
import sys


def run_tui() -> None:
    """Launch the Textual TUI app."""
    from monitor.app import VantaMonitorTUI as App

    app = App()
    sys.exit(app.run())


def run_web() -> None:
    """Launch the Flask web dashboard."""
    from monitor.server import app as flask_app

    flask_app.run(host="0.0.0.0", port=5001, debug=False, use_reloader=False)


def run_both() -> None:
    """Launch the TUI + web dashboard simultaneously."""
    import multiprocessing

    from monitor.app import VantaMonitorTUI as App
    from monitor.server import app as flask_app

    def _web() -> None:
        flask_app.run(host="0.0.0.0", port=5001, debug=False, use_reloader=False)

    p = multiprocessing.Process(target=_web, daemon=True)
    p.start()
    sys.exit(App().run())


def build_parser() -> argparse.ArgumentParser:
    """Build the CLI argument parser."""
    parser = argparse.ArgumentParser(
        prog="vmon",
        description="Vanta Monitor — Textual TUI and optional Flask web dashboard.",
    )
    parser.add_argument(
        "mode",
        nargs="?",
        default="tui",
        choices=["tui", "web", "both", "help"],
        help="Launch mode: tui (default), web, both, or help.",
    )
    return parser


def main(argv: list[str] | None = None) -> None:
    """Main dispatcher for CLI subcommands."""
    args = build_parser().parse_args(argv)

    if args.mode == "tui":
        run_tui()
    elif args.mode == "web":
        run_web()
    elif args.mode == "both":
        run_both()
    else:
        build_parser().print_help()
        raise SystemExit(0)


if __name__ == "__main__":
    main()

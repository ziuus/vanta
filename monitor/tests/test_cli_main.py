"""Tests for the vanta-monitor CLI entrypoint."""

from unittest.mock import patch

import pytest

from monitor import __main__ as cli


def test_main_defaults_to_tui_when_no_args() -> None:
    with patch.object(cli, "run_tui") as run_tui:
        cli.main([])
        run_tui.assert_called_once_with()


def test_main_runs_web_subcommand() -> None:
    with patch.object(cli, "run_web") as run_web:
        cli.main(["web"])
        run_web.assert_called_once_with()


def test_main_runs_both_subcommand() -> None:
    with patch.object(cli, "run_both") as run_both:
        cli.main(["both"])
        run_both.assert_called_once_with()


@pytest.mark.parametrize("args", [["-h"], ["--help"], ["help"]])
def test_main_help_exits_cleanly(args, capsys) -> None:
    with pytest.raises(SystemExit) as exc:
        cli.main(args)
    assert exc.value.code == 0
    output = capsys.readouterr().out
    assert "usage:" in output.lower()
    assert "tui" in output
    assert "web" in output
    assert "both" in output


def test_main_unknown_subcommand_exits_with_error(capsys) -> None:
    with pytest.raises(SystemExit) as exc:
        cli.main(["wat"])
    assert exc.value.code == 2
    err = capsys.readouterr().err.lower()
    assert "invalid choice" in err or "unrecognized arguments" in err

# Contributing to Vanta

First off, thank you for considering contributing to Vanta! It's people like you that make open-source such a great community.

## How Can I Contribute?

### Reporting Bugs
If you find a bug, please create an issue on GitHub. Include as much detail as possible:
* Your operating system and terminal emulator.
* Steps to reproduce the bug.
* Screenshots if applicable.

### Suggesting Enhancements
We love new ideas! If you have an idea for a new feature, mode, or widget:
* Open an issue using the "enhancement" label.
* Describe how it should work and why it would be useful.

### Pull Requests
1. Fork the repo and create your branch from `main`.
2. If you've added code that should be tested, add tests.
3. Ensure the test suite passes.
4. Update documentation if your change adds new functionality.
5. Submit your Pull Request!

## Development Setup

```bash
# Clone the repo
git clone https://github.com/ziuus/vanta.git
cd vanta/monitor

# Create a virtual environment and install
python3 -m venv .venv
source .venv/bin/activate
pip install -e ".[dev]"

# Optional: GPU support
pip install -e ".[gpu]"

# Run tests
pytest tests/ -v

# Launch the TUI
vtui
```

## Code of Conduct
Please note that this project is released with a Contributor Code of Conduct. By participating in this project you agree to abide by its terms. Be respectful and constructive!

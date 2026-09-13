import re

with open("src/monitors/system_info.rs", "r") as f:
    content = f.read()

content = re.sub(
    r"fn braille_art.*?fn logo_lines",
    "fn logo_lines",
    content,
    flags=re.DOTALL
)

# Remove #[cfg(test)] and everything after it.
test_idx = content.find("#[cfg(test)]")
if test_idx != -1:
    content = content[:test_idx]

with open("src/monitors/system_info.rs", "w") as f:
    f.write(content)

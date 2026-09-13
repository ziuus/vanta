import re

with open("src/monitors/system_info.rs", "r") as f:
    content = f.read()

# Replace logo_lines
content = re.sub(
    r"fn logo_lines\(id: &str\) -> Vec<String> \{.*?^\}",
    "fn logo_lines(id: &str) -> Vec<String> {\n    block_logo(id).into_iter().map(String::from).collect()\n}",
    content,
    flags=re.DOTALL | re.MULTILINE
)

# Replace block_logo with new block_logo that includes arch
arch_block = """fn block_logo(id: &str) -> Vec<&'static str> {
    match id {
        "arch" | "archarm" | "endeavouros" | "manjaro" | "cachyos" => vec![
            "     /\\\\     ",
            "    /  \\\\    ",
            "   /____\\\\   ",
            "  /  __  \\\\  ",
            " /  /  \\\\  \\\\ ",
            "/__/    \\\\__\\\\",
        ],"""

content = content.replace("fn block_logo(id: &str) -> Vec<&'static str> {\n    match id {", arch_block)

with open("src/monitors/system_info.rs", "w") as f:
    f.write(content)

import re

with open("src/widgets/video.rs", "r") as f:
    content = f.read()

content = re.sub(
    r"let now = SystemTime::now\(\).*?as_secs_f64\(\);",
    "let now = (_tick as f64) / 30.0;",
    content,
    flags=re.DOTALL
)

with open("src/widgets/video.rs", "w") as f:
    f.write(content)

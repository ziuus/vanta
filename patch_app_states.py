import re
with open("src/app.rs", "r") as f: content = f.read()

content = re.sub(r'pub process_compact_cmd: bool,', 'pub process_compact_cmd: bool,\n    pub obsidian_scroll: usize,\n    pub obsidian_selected: usize,', content, count=1)
content = re.sub(r'process_compact_cmd: false,', 'process_compact_cmd: false,\n            obsidian_scroll: 0,\n            obsidian_selected: 0,', content, count=1)

with open("src/app.rs", "w") as f: f.write(content)


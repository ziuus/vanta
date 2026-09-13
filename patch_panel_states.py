import re
with open("src/app.rs", "r") as f: content = f.read()

content = re.sub(r'process_compact_cmd: true,\n            process_selected_pid: None,', 'process_compact_cmd: true,\n            obsidian_scroll: 0,\n            obsidian_selected: 0,\n            process_selected_pid: None,', content, count=1)
with open("src/app.rs", "w") as f: f.write(content)


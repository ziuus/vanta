import re
with open("src/monitors/mod.rs", "r") as f: content = f.read()

content = re.sub(r'pub mod news;', 'pub mod news;\npub mod obsidian;', content, count=1)
content = re.sub(r'news::start\(url\);', 'news::start(url);\n    let vault = crate::config::Config::load().ui.obsidian_vault;\n    obsidian::start(vault);', content, count=1)

with open("src/monitors/mod.rs", "w") as f: f.write(content)


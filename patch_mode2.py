import re

with open("src/mode.rs", "r") as f: content = f.read()

content = re.sub(r'Self::Aesthetic => "Aesthetic",', 'Self::Aesthetic => "Aesthetic",\n            Self::Obsidian => "Obsidian",', content, count=1)
content = re.sub(r'Self::Aesthetic => \'3\',', "Self::Aesthetic => '3',\n            Self::Obsidian => '4',", content, count=1)

with open("src/mode.rs", "w") as f: f.write(content)

with open("src/screens/mod.rs", "r") as f: content = f.read()
content = content.replace("pub mod obsidian;\npub mod obsidian;", "pub mod obsidian;")
with open("src/screens/mod.rs", "w") as f: f.write(content)


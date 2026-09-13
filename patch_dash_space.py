import re

with open("src/screens/dashboard.rs", "r") as f:
    content = f.read()

# Add spacing(1) to all Layout::vertical
content = re.sub(r'Layout::vertical\(\[(.*?)\]\)', r'Layout::vertical([\1]).spacing(1)', content, flags=re.DOTALL)

# Add spacing(1) to all Layout::horizontal (except if it already has it)
# We already added .spacing(1).split(main_area); in the sed command. Let's fix that.
content = re.sub(r'\.spacing\(1\)\.split\(main_area\);', '.split(main_area);', content) # undo sed
content = re.sub(r'Layout::horizontal\(\[(.*?)\]\)', r'Layout::horizontal([\1]).spacing(1)', content, flags=re.DOTALL)
content = re.sub(r'Layout::horizontal\(constraints\)', r'Layout::horizontal(constraints).spacing(1)', content, flags=re.DOTALL)

# Remove footers
content = re.sub(r'Some\("← → month"\)', 'None', content)
content = re.sub(r'Some\("↑ ↓ scroll • k kill"\)', 'None', content)

with open("src/screens/dashboard.rs", "w") as f:
    f.write(content)

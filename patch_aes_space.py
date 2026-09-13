import re

with open("src/screens/aesthetic.rs", "r") as f:
    content = f.read()

content = re.sub(r'Layout::vertical\(\[(.*?)\]\)', r'Layout::vertical([\1]).spacing(1)', content, flags=re.DOTALL)
content = re.sub(r'Layout::horizontal\(\[(.*?)\]\)', r'Layout::horizontal([\1]).spacing(1)', content, flags=re.DOTALL)
content = re.sub(r'Layout::horizontal\(constraints\)', r'Layout::horizontal(constraints).spacing(1)', content, flags=re.DOTALL)

content = re.sub(r'Some\("← → month"\)', 'None', content)

with open("src/screens/aesthetic.rs", "w") as f:
    f.write(content)

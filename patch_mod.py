import re
with open("src/screens/mod.rs", "r") as f:
    content = f.read()

content = re.sub(r'P::Video => video::render\(f, inner, theme, app\.frame\),', 'P::Video => video::render(f, inner, theme, app.frame),\n        P::Weather => crate::widgets::weather::render(f, inner, theme),', content, count=1)

with open("src/screens/mod.rs", "w") as f:
    f.write(content)

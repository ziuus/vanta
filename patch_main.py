import re

with open("src/main.rs", "r") as f:
    content = f.read()

# Make sure app.tick is incremented!
# But wait, where is app.tick? Let's check app.rs to see if there's a tick field.

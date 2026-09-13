import re
with open("src/config.rs", "r") as f: content = f.read()

content = re.sub(r'pub clock_style: String,', 'pub clock_style: String,\n    pub timezones: Vec<String>,', content, count=1)
content = re.sub(r'clock_style: "standard"\.to_string\(\),', 'clock_style: "standard".to_string(),\n            timezones: vec!["UTC".to_string(), "America/New_York".to_string(), "Asia/Tokyo".to_string()],', content, count=1)

with open("src/config.rs", "w") as f: f.write(content)

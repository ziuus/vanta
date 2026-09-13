import re
with open("src/screens/settings.rs", "r") as f:
    content = f.read()

# Add ClockStyle enum
content = re.sub(r'ClockFont,', 'ClockFont,\n    ClockStyle,', content, count=1)

# Add ClockStyle string
content = re.sub(r'\(SettingType::ClockFont, "Clock Font"\),', '(SettingType::ClockFont, "Clock Font"),\n    (SettingType::ClockStyle, "Clock Style"),', content, count=1)

# Add ClockStyle display
content = re.sub(r'SettingType::ClockFont => app\.config\.ui\.clock_font\.clone\(\),', 'SettingType::ClockFont => app.config.ui.clock_font.clone(),\n            SettingType::ClockStyle => app.config.ui.clock_style.clone(),', content, count=1)

# Add ClockStyle toggle
toggle_code = """        SettingType::ClockStyle => {
            let styles = ["solid", "dotted", "hollow"];
            let pos = styles.iter().position(|&x| x == app.config.ui.clock_style).unwrap_or(0);
            app.config.ui.clock_style = if forward { styles[(pos + 1) % styles.len()].to_string() } else { styles[(pos + styles.len() - 1) % styles.len()].to_string() };
        }"""
content = re.sub(r'(SettingType::ClockFont => \{.*?\n        \})', r'\1\n' + toggle_code, content, flags=re.DOTALL, count=1)

with open("src/screens/settings.rs", "w") as f:
    f.write(content)

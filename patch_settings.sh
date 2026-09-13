awk '
/ClockFont,/ {
    print "    ClockFont,"
    print "    ClockStyle,"
    next
}
/ClockFont, "Clock Font"/ {
    print "    (SettingType::ClockFont, \"Clock Font\"),"
    print "    (SettingType::ClockStyle, \"Clock Style\"),"
    next
}
/SettingType::ClockFont => app.config.ui.clock_font.clone(),/ {
    print "            SettingType::ClockFont => app.config.ui.clock_font.clone(),"
    print "            SettingType::ClockStyle => app.config.ui.clock_style.clone(),"
    next
}
/SettingType::ClockFont => \{/ {
    print "        SettingType::ClockFont => {"
    print "            let fonts = [\"standard\", \"rounded\", \"digital\"];"
    print "            let pos = fonts.iter().position(|&x| x == app.config.ui.clock_font).unwrap_or(0);"
    print "            app.config.ui.clock_font = if forward { fonts[(pos + 1) % fonts.len()].to_string() } else { fonts[(pos + fonts.len() - 1) % fonts.len()].to_string() };"
    print "        }"
    print "        SettingType::ClockStyle => {"
    print "            let styles = [\"solid\", \"dotted\", \"hollow\"];"
    print "            let pos = styles.iter().position(|&x| x == app.config.ui.clock_style).unwrap_or(0);"
    print "            app.config.ui.clock_style = if forward { styles[(pos + 1) % styles.len()].to_string() } else { styles[(pos + styles.len() - 1) % styles.len()].to_string() };"
    print "        }"
    next
}
/            let fonts = \["standard", "rounded", "digital"\];/ { next }
/            let pos = fonts.iter().position(|&x| x == app.config.ui.clock_font).unwrap_or(0);/ { next }
/            app.config.ui.clock_font = if forward \{ fonts\[(pos \+ 1) % fonts.len()\].to_string() \} else \{ fonts\[(pos \+ fonts.len() - 1) % fonts.len()\].to_string() \};/ { next }
/        \}/ { 
    if (skip_brace) { skip_brace = 0; next } 
}
{ print }
' src/screens/settings.rs > temp.rs
mv temp.rs src/screens/settings.rs

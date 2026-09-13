import re
with open("src/widgets/clock.rs", "r") as f: content = f.read()

# Add chrono-tz import
content = re.sub(r'use chrono::\{Local, Timelike\};', 'use chrono::{Local, Utc, Timelike};\nuse chrono_tz::Tz;', content, count=1)

# Update render signature
content = re.sub(r'pub fn render\(f: &mut Frame, area: Rect, theme: &Theme, h24: bool, font: &str, style: &str\) \{', 
                 r'pub fn render(f: &mut Frame, area: Rect, theme: &Theme, h24: bool, font: &str, style: &str, timezones: &[String]) {', content, count=1)

# Before the early return for fallback, check if we need space for timezones
content = re.sub(r'    let glyph_h_avail = \(area\.height as usize\)\.saturating_sub\(2\);',
                 r'    let tz_height = if timezones.is_empty() { 0 } else { timezones.len() + 2 };\n    let glyph_h_avail = (area.height as usize).saturating_sub(2 + tz_height);', content, count=1)

# After rendering the main clock+date, render the timezones
tz_render = """    let date_y = top + n + 1;
    if date_y < area.y + area.height {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                date,
                Style::default().fg(theme.dim),
            )))
            .alignment(Alignment::Center),
            Rect::new(area.x, date_y, area.width, 1),
        );
    }
    
    // Render timezones
    if !timezones.is_empty() && area.height >= date_y - area.y + 2 + timezones.len() as u16 {
        let mut tz_lines = Vec::new();
        tz_lines.push(Line::from(vec![])); // Spacer
        tz_lines.push(Line::from(vec![
            Span::styled("ZONE", Style::default().fg(theme.dim)),
            Span::raw(" ".repeat(area.width.saturating_sub(12) as usize)),
            Span::styled("TIME", Style::default().fg(theme.dim)),
        ]));
        
        for tz_str in timezones {
            if let Ok(tz) = tz_str.parse::<Tz>() {
                let time_in_tz = Utc::now().with_timezone(&tz);
                let time_fmt = if h24 { time_in_tz.format("%H:%M:%S") } else { time_in_tz.format("%I:%M:%S %p") };
                let tz_name = if tz_str.len() > 15 { &tz_str[..15] } else { tz_str };
                
                let time_str = time_fmt.to_string();
                let pad = area.width.saturating_sub((tz_name.len() + time_str.len()) as u16) as usize;
                
                tz_lines.push(Line::from(vec![
                    Span::styled(tz_name, Style::default().fg(theme.text)),
                    Span::raw(" ".repeat(pad)),
                    Span::styled(time_str, Style::default().fg(theme.accent)),
                ]));
            }
        }
        
        f.render_widget(
            Paragraph::new(tz_lines),
            Rect::new(area.x, date_y + 1, area.width, area.height.saturating_sub(date_y + 1 - area.y)),
        );
    }"""
content = re.sub(r'    let date_y = top \+ n \+ 1;\n    if date_y < area\.y \+ area\.height \{\n        f\.render_widget\(\n            Paragraph::new\(Line::from\(Span::styled\(\n                date,\n                Style::default\(\)\.fg\(theme\.dim\),\n            \)\)\)\n            \.alignment\(Alignment::Center\),\n            Rect::new\(area\.x, date_y, area\.width, 1\),\n        \);\n    \}', tz_render, content, count=1)

with open("src/widgets/clock.rs", "w") as f: f.write(content)

# Update screens/mod.rs
with open("src/screens/mod.rs", "r") as f: content = f.read()
content = re.sub(r'P::Clock => clock::render\(f, inner, theme, app\.config\.ui\.clock_24h, &app\.config\.ui\.clock_font, &app\.config\.ui\.clock_style\),',
                 r'P::Clock => clock::render(f, inner, theme, app.config.ui.clock_24h, &app.config.ui.clock_font, &app.config.ui.clock_style, &app.config.ui.timezones),', content, count=1)
with open("src/screens/mod.rs", "w") as f: f.write(content)

# Update screens/dashboard.rs
with open("src/screens/dashboard.rs", "r") as f: content = f.read()
content = re.sub(r'Constraint::Length\(if cfg\.clock \{ 9 \} else \{ 0 \}\), // CLOCK: 5 glyph \+ gap \+ date',
                 r'Constraint::Length(if cfg.clock { if app.config.ui.timezones.is_empty() { 9 } else { 9 + app.config.ui.timezones.len() as u16 + 2 } } else { 0 }), // CLOCK', content, count=1)
content = re.sub(r'clock::render\(f, inner, theme, app\.config\.ui\.clock_24h, &app\.config\.ui\.clock_font, &app\.config\.ui\.clock_style\);',
                 r'clock::render(f, inner, theme, app.config.ui.clock_24h, &app.config.ui.clock_font, &app.config.ui.clock_style, &app.config.ui.timezones);', content, count=1)
with open("src/screens/dashboard.rs", "w") as f: f.write(content)


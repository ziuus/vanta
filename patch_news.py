import re

# monitors/mod.rs
with open("src/monitors/mod.rs", "r") as f: content = f.read()
content = re.sub(r'pub mod agenda;', 'pub mod agenda;\npub mod news;', content, count=1)
content = re.sub(r'agenda::start\(\);', 'agenda::start();\n    // News needs the feed URL from config, so we will start it in app.rs, or pass a hardcoded one here if not initialized. Actually, best to fetch config inside start() or pass it.', content, count=1)
with open("src/monitors/mod.rs", "w") as f: f.write(content)

# We'll just patch monitors/mod.rs again to read config for news URL directly, or hardcode it since monitors start before app loads
with open("src/monitors/mod.rs", "r") as f: content = f.read()
# Replace the comment with the actual startup
start_news = """agenda::start();
    let url = crate::config::load().map(|c| c.ui.news_feed).unwrap_or_else(|_| "https://news.ycombinator.com/rss".to_string());
    news::start(url);"""
content = content.replace("agenda::start();\n    // News needs the feed URL from config, so we will start it in app.rs, or pass a hardcoded one here if not initialized. Actually, best to fetch config inside start() or pass it.", start_news)
with open("src/monitors/mod.rs", "w") as f: f.write(content)

# widgets/mod.rs
with open("src/widgets/mod.rs", "r") as f: content = f.read()
content = re.sub(r'pub mod agenda;', 'pub mod agenda;\npub mod news;', content, count=1)
with open("src/widgets/mod.rs", "w") as f: f.write(content)

# config.rs
with open("src/config.rs", "r") as f: content = f.read()
content = re.sub(r'pub agenda: bool,', 'pub agenda: bool,\n    pub news: bool,\n    pub news_feed: String,', content, count=1)
content = re.sub(r'agenda: true,', 'agenda: true,\n            news: true,\n            news_feed: "https://news.ycombinator.com/rss".to_string(),', content, count=1)
with open("src/config.rs", "w") as f: f.write(content)

# settings.rs
with open("src/screens/settings.rs", "r") as f: content = f.read()
content = re.sub(r'WidgetAgenda,', 'WidgetAgenda,\n    WidgetNews,', content, count=1)
content = re.sub(r'&SettingType::WidgetAgenda => "Agenda",', '&SettingType::WidgetAgenda => "Agenda",\n            &SettingType::WidgetNews => "News",', content, count=1)
content = re.sub(r'&SettingType::WidgetAgenda => app.config.ui.agenda,', '&SettingType::WidgetAgenda => app.config.ui.agenda,\n            &SettingType::WidgetNews => app.config.ui.news,', content, count=1)
content = re.sub(r'&SettingType::WidgetAgenda => app.config.ui.agenda = !app.config.ui.agenda,', '&SettingType::WidgetAgenda => app.config.ui.agenda = !app.config.ui.agenda,\n            &SettingType::WidgetNews => app.config.ui.news = !app.config.ui.news,', content, count=1)
content = re.sub(r'SettingType::WidgetAgenda,', 'SettingType::WidgetAgenda,\n    SettingType::WidgetNews,', content, count=1)
with open("src/screens/settings.rs", "w") as f: f.write(content)

# app.rs
with open("src/app.rs", "r") as f: content = f.read()
content = re.sub(r'Agenda,', 'Agenda,\n    News,', content, count=1)
content = re.sub(r'\(PanelId::Agenda, w.agenda\),', '(PanelId::Agenda, w.agenda),\n                (PanelId::News, w.news),', content, count=1)
content = re.sub(r'&PanelId::Agenda => "agenda",', '&PanelId::Agenda => "agenda",\n            &PanelId::News => "news",', content, count=1)
with open("src/app.rs", "w") as f: f.write(content)

# screens/mod.rs
with open("src/screens/mod.rs", "r") as f: content = f.read()
content = re.sub(r'P::Agenda => " AGENDA ",', 'P::Agenda => " AGENDA ",\n        P::News => " NEWS ",', content, count=1)
content = re.sub(r'P::Agenda => crate::widgets::agenda::render\(f, inner, theme\),', 'P::Agenda => crate::widgets::agenda::render(f, inner, theme),\n        P::News => crate::widgets::news::render(f, inner, theme),', content, count=1)
with open("src/screens/mod.rs", "w") as f: f.write(content)

# dashboard.rs
with open("src/screens/dashboard.rs", "r") as f: content = f.read()
content = re.sub(r'Constraint::Length\(if cfg\.agenda \{ 10 \} else \{ 0 \}\),   // AGENDA',
                 r'Constraint::Length(if cfg.agenda { 10 } else { 0 }),   // AGENDA\n            Constraint::Length(if cfg.news { 8 } else { 0 }),      // NEWS', content, count=1)

render_news = """        if cfg.news {
            let snap = crate::monitors::news::snapshot();
            let source = if snap.channel_title.is_empty() { " fetching ".to_string() } else { format!(" {} ", snap.channel_title) };
            let inner = panel_full(f, rows[6], "news", Some(&source), None, theme, focus(PanelId::News));
            crate::widgets::news::render(f, inner, theme);
        }
    }"""
content = re.sub(r'        if cfg.agenda \{.*?crate::widgets::agenda::render\(f, inner, theme\);\n        \}\n    \}',
                 r'        if cfg.agenda {\n            let snap = crate::monitors::agenda::snapshot();\n            let count = snap.events.len();\n            let agenda_rt = if count == 0 { " no events ".to_string() } else { format!(" {} upcoming ", count) };\n            let inner = panel_full(f, rows[5], "agenda", Some(&agenda_rt), None, theme, focus(PanelId::Agenda));\n            crate::widgets::agenda::render(f, inner, theme);\n        }\n\n' + render_news, content, flags=re.DOTALL, count=1)
with open("src/screens/dashboard.rs", "w") as f: f.write(content)


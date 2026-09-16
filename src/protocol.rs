use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum UiColor {
    Reset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
    Rgb(u8, u8, u8),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UiStyle {
    pub fg: Option<UiColor>,
    pub bg: Option<UiColor>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underlined: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UiSpan {
    pub content: String,
    pub style: Option<UiStyle>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UiLine {
    pub spans: Vec<UiSpan>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UiBlock {
    pub title: Option<String>,
    pub bordered: bool,
    pub border_color: Option<UiColor>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum UiWidget {
    Paragraph {
        lines: Vec<UiLine>,
        block: Option<UiBlock>,
        wrap: bool,
    },
    Gauge {
        ratio: f64,
        label: Option<String>,
        block: Option<UiBlock>,
        color: Option<UiColor>,
    },
    List {
        items: Vec<UiLine>,
        block: Option<UiBlock>,
    },
    Column {
        children: Vec<UiWidget>,
        /// Ratios or percentages for the layout, if absent, distributes evenly
        percentages: Option<Vec<u16>>,
    },
    Row {
        children: Vec<UiWidget>,
        percentages: Option<Vec<u16>>,
    },
}

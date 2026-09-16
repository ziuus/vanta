pub const MAX_TEXT_LENGTH: usize = 100000;
use serde::{Deserialize, Serialize};

pub const MAX_UI_TREE_DEPTH: usize = 16;
pub const MAX_UI_SPANS: usize = 2000;
pub const MAX_PAYLOAD_SIZE: usize = 1024 * 1024; // 1 MB limit for JSON payload

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum UiColor {
    Reset, Black, Red, Green, Yellow, Blue, Magenta, Cyan, Gray,
    DarkGray, LightRed, LightGreen, LightYellow, LightBlue,
    LightMagenta, LightCyan, White,
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
        percentages: Option<Vec<u16>>,
    },
    Row {
        children: Vec<UiWidget>,
        percentages: Option<Vec<u16>>,
    },
}

impl UiWidget {
    /// Validates the UI tree against safety limits to prevent malicious/buggy WASM from crashing Vanta.
    pub fn validate(&self) -> Result<(), &'static str> {
        let mut spans = 0;
        self.check_limits(1, &mut spans, &mut 0)
    }

    fn check_limits(&self, depth: usize, spans: &mut usize, text_len: &mut usize) -> Result<(), &'static str> {
        if depth > MAX_UI_TREE_DEPTH {
            return Err("UI tree exceeded maximum depth");
        }
        
        match self {
            UiWidget::Paragraph { lines, .. } | UiWidget::List { items: lines, .. } => {
                for line in lines {
                    *spans += line.spans.len();
                    if *spans > MAX_UI_SPANS {
                        return Err("UI tree exceeded maximum spans");
                    }
                    for span in &line.spans {
                        *text_len += span.content.len();
                        if *text_len > MAX_TEXT_LENGTH {
                            return Err("UI tree exceeded maximum text length");
                        }
                    }
                }
            }
            UiWidget::Gauge { .. } => {
                *spans += 1;
            }
            UiWidget::Column { children, .. } | UiWidget::Row { children, .. } => {
                for child in children {
                    child.check_limits(depth + 1, spans, text_len)?;
                }
            }
        }
        
        Ok(())
    }
}

use ratatui::style::Color;

/// Named palettes, in the order `T` cycles through them.
pub const THEME_NAMES: [&str; 8] = [
    "dark",
    "catppuccin",
    "tokyo-night",
    "nord",
    "gruvbox",
    "dracula",
    "light",
    "solarized-light",
];

#[derive(Clone)]
pub struct Theme {
    pub bg: Color,
    pub accent: Color,
    pub secondary: Color,
    pub surface: Color,
    pub text: Color,
    pub dim: Color,
    pub green: Color,
    pub yellow: Color,
    pub red: Color,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

/// Linear blend from `a` to `b`, `t` in 0..1.
///
/// Every palette here is truecolor, so this is exact in practice. An indexed
/// or named colour has no meaningful midpoint, so it snaps at the halfway
/// point rather than inventing one — a wrong-but-stable colour beats a
/// gradient that flickers between two palette entries.
pub fn blend(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (a, b) {
        (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) => {
            let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
            Color::Rgb(mix(ar, br), mix(ag, bg), mix(ab, bb))
        }
        _ if t < 0.5 => a,
        _ => b,
    }
}

impl Theme {
    pub fn from_name(name: &str) -> Self {
        match name {
            "light" => Self::light(),
            "dracula" => Self::dracula(),
            "solarized-light" => Self::solarized_light(),
            "catppuccin" | "catppuccin-mocha" => Self::catppuccin(),
            "tokyo-night" => Self::tokyo_night(),
            "nord" => Self::nord(),
            "gruvbox" => Self::gruvbox(),
            _ => Self::dark(),
        }
    }

    /// Name that follows `current` in the cycle order.
    pub fn next_name(current: &str) -> &'static str {
        let idx = THEME_NAMES.iter().position(|&n| n == current).unwrap_or(0);
        THEME_NAMES[(idx + 1) % THEME_NAMES.len()]
    }

    pub fn dark() -> Self {
        Self {
            bg: rgb(10, 10, 15),
            accent: rgb(120, 220, 150),
            secondary: rgb(100, 120, 200),
            surface: rgb(22, 22, 32),
            text: rgb(220, 220, 230),
            dim: rgb(80, 80, 95),
            green: rgb(80, 200, 120),
            yellow: rgb(220, 200, 60),
            red: rgb(220, 80, 80),
        }
    }

    pub fn catppuccin() -> Self {
        Self {
            bg: rgb(30, 30, 46),
            accent: rgb(203, 166, 247),
            secondary: rgb(137, 180, 250),
            surface: rgb(49, 50, 68),
            text: rgb(205, 214, 244),
            dim: rgb(108, 112, 134),
            green: rgb(166, 227, 161),
            yellow: rgb(249, 226, 175),
            red: rgb(243, 139, 168),
        }
    }

    pub fn tokyo_night() -> Self {
        Self {
            bg: rgb(26, 27, 38),
            accent: rgb(122, 162, 247),
            secondary: rgb(187, 154, 247),
            surface: rgb(41, 46, 66),
            text: rgb(192, 202, 245),
            dim: rgb(86, 95, 137),
            green: rgb(158, 206, 106),
            yellow: rgb(224, 175, 104),
            red: rgb(247, 118, 142),
        }
    }

    pub fn nord() -> Self {
        Self {
            bg: rgb(46, 52, 64),
            accent: rgb(136, 192, 208),
            secondary: rgb(129, 161, 193),
            surface: rgb(59, 66, 82),
            text: rgb(236, 239, 244),
            dim: rgb(97, 110, 136),
            green: rgb(163, 190, 140),
            yellow: rgb(235, 203, 139),
            red: rgb(191, 97, 106),
        }
    }

    pub fn gruvbox() -> Self {
        Self {
            bg: rgb(40, 40, 40),
            accent: rgb(250, 189, 47),
            secondary: rgb(131, 165, 152),
            surface: rgb(60, 56, 54),
            text: rgb(235, 219, 178),
            dim: rgb(124, 111, 100),
            green: rgb(184, 187, 38),
            yellow: rgb(254, 128, 25),
            red: rgb(251, 73, 52),
        }
    }

    pub fn dracula() -> Self {
        Self {
            bg: rgb(40, 42, 54),
            accent: rgb(189, 147, 249),
            secondary: rgb(139, 233, 253),
            surface: rgb(68, 71, 90),
            text: rgb(248, 248, 242),
            dim: rgb(98, 114, 164),
            green: rgb(80, 250, 123),
            yellow: rgb(255, 203, 107),
            red: rgb(255, 121, 198),
        }
    }

    pub fn light() -> Self {
        Self {
            bg: rgb(245, 245, 240),
            accent: rgb(0, 100, 200),
            secondary: rgb(60, 80, 180),
            surface: rgb(228, 228, 222),
            text: rgb(20, 20, 30),
            dim: rgb(140, 140, 145),
            green: rgb(40, 160, 80),
            yellow: rgb(180, 140, 20),
            red: rgb(200, 50, 50),
        }
    }

    pub fn solarized_light() -> Self {
        Self {
            bg: rgb(253, 246, 227),
            accent: rgb(42, 161, 152),
            secondary: rgb(108, 113, 196),
            surface: rgb(238, 232, 213),
            text: rgb(88, 110, 117),
            dim: rgb(147, 161, 161),
            green: rgb(133, 153, 0),
            yellow: rgb(181, 137, 0),
            red: rgb(220, 50, 47),
        }
    }

    /// Green → yellow → red by threshold. Shared by every "usage %" readout so
    /// colours mean the same thing everywhere.
    pub fn usage(&self, pct: f64) -> Color {
        if pct >= 90.0 {
            self.red
        } else if pct >= 75.0 {
            self.yellow
        } else {
            self.accent
        }
    }

    pub fn temp(&self, c: f64) -> Color {
        if c >= 90.0 {
            self.red
        } else if c >= 75.0 {
            self.yellow
        } else {
            self.accent
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_hits_both_ends_exactly() {
        let a = rgb(0, 0, 0);
        let b = rgb(200, 100, 50);
        assert_eq!(blend(a, b, 0.0), a);
        assert_eq!(blend(a, b, 1.0), b);
        assert_eq!(blend(a, b, 0.5), rgb(100, 50, 25));
    }

    #[test]
    fn blend_clamps_out_of_range_t() {
        let a = rgb(10, 10, 10);
        let b = rgb(20, 20, 20);
        assert_eq!(blend(a, b, -5.0), a);
        assert_eq!(blend(a, b, 5.0), b);
    }

    #[test]
    fn blend_snaps_for_non_rgb() {
        // No meaningful midpoint between palette entries — must not panic or
        // invent one.
        assert_eq!(blend(Color::Red, Color::Blue, 0.2), Color::Red);
        assert_eq!(blend(Color::Red, Color::Blue, 0.8), Color::Blue);
    }
}

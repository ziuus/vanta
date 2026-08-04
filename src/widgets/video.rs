use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

/// Render a rotating 3D torus (donut) as an ASCII "video" demo.
#[allow(non_snake_case)]
pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme, _tick: u64) {
    if area.width < 10 || area.height < 5 {
        return;
    }

    let width = area.width as usize;
    let height = area.height as usize;

    let mut z_buffer = vec![0.0f32; width * height];
    let mut b_buffer = vec![' '; width * height];

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as f32;
    let A = now * 0.0012; // X rotation speed
    let B = now * 0.0008; // Y rotation speed

    let (sinA, cosA) = A.sin_cos();
    let (sinB, cosB) = B.sin_cos();

    let R1 = 1.0;
    let R2 = 2.0;
    let K2 = 5.0;
    // Scale K1 dynamically based on available area to fit perfectly
    let K1 = (width.min(height * 2) as f32) * K2 * 3.0 / (8.0 * (R1 + R2));

    for j in 0..90 {
        let theta = (j as f32) * 0.07;
        let (sinTheta, cosTheta) = theta.sin_cos();

        for i in 0..314 {
            let phi = (i as f32) * 0.02;
            let (sinPhi, cosPhi) = phi.sin_cos();

            let circleX = R2 + R1 * cosTheta;
            let circleY = R1 * sinTheta;

            let x = circleX * (cosB * cosPhi + sinA * sinB * sinPhi) - circleY * cosA * sinB;
            let y = circleX * (sinB * cosPhi - sinA * cosB * sinPhi) + circleY * cosA * cosB;
            let z = K2 + cosA * circleX * sinPhi + circleY * sinA;
            let ooz = 1.0 / z;

            // adjust aspect ratio (multiply Y by 0.5 because terminal chars are roughly 2x1)
            let xp = (width as f32 / 2.0 + K1 * ooz * x) as i32;
            let yp = (height as f32 / 2.0 - (K1 * 0.5) * ooz * y) as i32;

            let L = cosPhi * cosTheta * sinB - cosA * cosTheta * sinPhi - sinA * sinTheta
                + cosB * (cosA * sinTheta - cosTheta * sinA * sinPhi);

            if L > 0.0 && xp >= 0 && xp < width as i32 && yp >= 0 && yp < height as i32 {
                let idx = (yp * width as i32 + xp) as usize;
                if ooz > z_buffer[idx] {
                    z_buffer[idx] = ooz;
                    let lum_idx = (L * 8.0) as usize;
                    b_buffer[idx] = ".,-~:;=!*#$@"
                        .chars()
                        .nth(lum_idx.clamp(0, 11))
                        .unwrap_or('@');
                }
            }
        }
    }

    let mut lines = Vec::with_capacity(height);
    for y in 0..height {
        let row: String = b_buffer[y * width..(y + 1) * width].iter().collect();
        lines.push(Line::from(Span::styled(
            row,
            Style::default().fg(theme.accent),
        )));
    }

    let p = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(p, area);
}

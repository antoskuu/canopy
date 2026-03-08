use eframe::egui::{self, Color32, Rect, Sense, Stroke, StrokeKind, Ui};

use crate::scanner::DirView;
use crate::treemap::{layout, TreemapRect};
use crate::utils::{color_for_dir, color_for_file, human_readable_size, BG_DARK};

const GAP: f32 = 2.0;
const ROUNDING: f32 = 4.0;

pub struct RenderResult {
    pub hovered: Option<(String, u64)>,
    pub clicked_dir: Option<usize>,
}

pub fn draw_treemap(
    ui: &mut Ui,
    view: &DirView,
    cached_layout: &mut Option<(Rect, Vec<TreemapRect>)>,
    disk_info: Option<(u64, u64)>,
) -> RenderResult {
    let available = ui.available_rect_before_wrap();
    let (response, painter) = ui.allocate_painter(available.size(), Sense::hover());
    let full_area = response.rect;

    painter.rect_filled(full_area, 0.0, BG_DARK);

    // Split off free-space block on the right
    let (area, free_rect) = if let Some((total, free)) = disk_info {
        if total > 0 {
            let ratio = (free as f64 / total as f64).clamp(0.03, 0.20) as f32;
            let free_w = full_area.width() * ratio;
            let treemap_area = Rect::from_min_max(
                full_area.min,
                egui::pos2(full_area.max.x - free_w, full_area.max.y),
            );
            let free_area = Rect::from_min_max(
                egui::pos2(full_area.max.x - free_w, full_area.min.y),
                full_area.max,
            );
            (treemap_area, Some((free_area, free)))
        } else {
            (full_area, None)
        }
    } else {
        (full_area, None)
    };

    // Draw free space block
    if let Some((fr, free_bytes)) = free_rect {
        let inner = fr.shrink(GAP);
        let free_color = Color32::from_rgb(35, 35, 40);
        painter.rect_filled(inner, ROUNDING, free_color);

        // Diagonal stripe pattern
        let stripe_color = Color32::from_rgb(45, 45, 52);
        let spacing = 12.0;
        let min_x = inner.min.x;
        let min_y = inner.min.y;
        let max_x = inner.max.x;
        let max_y = inner.max.y;
        let total_span = (max_x - min_x) + (max_y - min_y);
        let mut offset = 0.0;
        while offset < total_span {
            let x0 = min_x + offset;
            let y0 = min_y;
            let x1 = min_x;
            let y1 = min_y + offset;
            // Clip to rect
            let (sx, sy, ex, ey) = clip_line_to_rect(x0, y0, x1, y1, min_x, min_y, max_x, max_y);
            painter.line_segment(
                [egui::pos2(sx, sy), egui::pos2(ex, ey)],
                Stroke::new(1.0, stripe_color),
            );
            offset += spacing;
        }

        // Label
        if inner.width() > 30.0 && inner.height() > 18.0 {
            let padding = 6.0;
            let text_pos = inner.left_top() + egui::vec2(padding, padding);
            painter.text(
                text_pos,
                egui::Align2::LEFT_TOP,
                "free",
                egui::FontId::monospace(11.0),
                Color32::from_rgb(120, 120, 130),
            );
            if inner.height() > 34.0 {
                let size_pos = inner.left_top() + egui::vec2(padding, padding + 14.0);
                painter.text(
                    size_pos,
                    egui::Align2::LEFT_TOP,
                    human_readable_size(free_bytes),
                    egui::FontId::monospace(9.5),
                    Color32::from_rgb(100, 100, 110),
                );
            }
        }
    }

    let rects = match cached_layout {
        Some((cached_area, ref rects)) if *cached_area == area => rects,
        _ => {
            let rects = layout(&view.entries, area);
            *cached_layout = Some((area, rects));
            &cached_layout.as_ref().unwrap().1
        }
    };

    let hover_pos = ui.input(|i| i.pointer.hover_pos());
    let clicked = ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary));

    let mut result = RenderResult {
        hovered: None,
        clicked_dir: None,
    };

    for tr in rects.iter() {
        if tr.rect.width() < 2.0 || tr.rect.height() < 2.0 {
            continue;
        }

        let inner = tr.rect.shrink(GAP);
        if inner.width() <= 0.0 || inner.height() <= 0.0 {
            continue;
        }

        let base_color = if tr.is_dir {
            color_for_dir(tr.child_index)
        } else {
            color_for_file(&tr.extension)
        };

        let is_hovered = hover_pos.map_or(false, |p| inner.contains(p));

        let fill = if is_hovered {
            lighten(base_color, 0.25)
        } else if tr.is_dir && !tr.size_known {
            // Dims further for computing dirs
            dim(base_color, 0.5)
        } else {
            dim(base_color, 0.75)
        };

        painter.rect_filled(inner, ROUNDING, fill);

        if is_hovered {
            painter.rect_stroke(
                inner,
                ROUNDING,
                Stroke::new(2.0, lighten(base_color, 0.5)),
                StrokeKind::Inside,
            );
        }

        // Label
        if inner.width() > 36.0 && inner.height() > 18.0 {
            let padding = 6.0;
            let avail_w = inner.width() - padding * 2.0;
            let max_chars = (avail_w / 7.0) as usize;

            if max_chars > 0 {
                let label = if tr.label.len() > max_chars && max_chars > 2 {
                    format!("{}..", &tr.label[..max_chars - 2])
                } else {
                    tr.label.clone()
                };

                let text_color = Color32::from_rgba_premultiplied(255, 255, 255, 220);

                let text_pos = inner.left_top() + egui::vec2(padding, padding);
                painter.text(
                    text_pos,
                    egui::Align2::LEFT_TOP,
                    &label,
                    egui::FontId::monospace(11.0),
                    text_color,
                );

                // Size below name
                if inner.height() > 34.0 {
                    let size_text = if tr.size_known {
                        human_readable_size(tr.size)
                    } else {
                        "...".to_string()
                    };
                    let size_pos = inner.left_top() + egui::vec2(padding, padding + 14.0);
                    painter.text(
                        size_pos,
                        egui::Align2::LEFT_TOP,
                        size_text,
                        egui::FontId::monospace(9.5),
                        Color32::from_rgba_premultiplied(200, 200, 200, 160),
                    );
                }

                // Directory indicator
                if tr.is_dir && inner.height() > 34.0 && inner.width() > 50.0 {
                    let indicator_pos = inner.right_top() + egui::vec2(-padding - 6.0, padding);
                    painter.text(
                        indicator_pos,
                        egui::Align2::LEFT_TOP,
                        ">",
                        egui::FontId::monospace(11.0),
                        Color32::from_rgba_premultiplied(255, 255, 255, 100),
                    );
                }
            }
        }

        if is_hovered {
            result.hovered = Some((
                view.entries[tr.child_index]
                    .path
                    .to_string_lossy()
                    .to_string(),
                tr.size,
            ));

            if clicked && tr.is_dir {
                result.clicked_dir = Some(tr.child_index);
            }
        }
    }

    result
}

fn lighten(c: Color32, amount: f32) -> Color32 {
    let r = c.r() as f32 + (255.0 - c.r() as f32) * amount;
    let g = c.g() as f32 + (255.0 - c.g() as f32) * amount;
    let b = c.b() as f32 + (255.0 - c.b() as f32) * amount;
    Color32::from_rgb(r as u8, g as u8, b as u8)
}

fn dim(c: Color32, factor: f32) -> Color32 {
    Color32::from_rgb(
        (c.r() as f32 * factor) as u8,
        (c.g() as f32 * factor) as u8,
        (c.b() as f32 * factor) as u8,
    )
}

/// Clip a diagonal line (top-right to bottom-left) to a rectangle.
fn clip_line_to_rect(
    x0: f32, y0: f32, x1: f32, y1: f32,
    min_x: f32, min_y: f32, max_x: f32, max_y: f32,
) -> (f32, f32, f32, f32) {
    // Line goes from (x0,y0) top towards (x1,y1) bottom-left with slope -1 (dy=dx)
    // Parametric: start = (x0,y0), end = (x1,y1)
    // For our diagonal stripes: x0 is on top edge, x1 is on left edge
    let sx = x0.clamp(min_x, max_x);
    let sy = y0 + (x0 - sx); // move down as we move left
    let ey = y1.clamp(min_y, max_y);
    let ex = x1 + (y1 - ey); // move right as we move up
    (sx, sy.clamp(min_y, max_y), ex.clamp(min_x, max_x), ey)
}

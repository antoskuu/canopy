use eframe::egui::Rect;
use eframe::emath::{pos2, vec2};

use crate::scanner::DirEntry;

pub struct TreemapRect {
    pub rect: Rect,
    pub child_index: usize,
    pub label: String,
    pub size: u64,
    pub is_dir: bool,
    pub extension: Option<String>,
    pub size_known: bool,
}

pub fn layout(entries: &[DirEntry], area: Rect) -> Vec<TreemapRect> {
    if entries.is_empty() {
        return Vec::new();
    }

    // Use placeholder size of 1 for directories with unknown size,
    // so they are visible even before background computation finishes.
    let effective_size = |e: &DirEntry| -> u64 {
        if e.is_dir && !e.size_known && e.size == 0 {
            1
        } else {
            e.size
        }
    };

    let total_size: u64 = entries.iter().map(|e| effective_size(e)).sum();
    if total_size == 0 {
        return Vec::new();
    }

    let total_f = total_size as f64;
    let mut results = Vec::with_capacity(entries.len());

    let items: Vec<(usize, f64)> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| effective_size(e) > 0)
        .map(|(i, e)| (i, effective_size(e) as f64 / total_f))
        .collect();

    squarify(&items, area, entries, &mut results);
    results
}

fn squarify(
    items: &[(usize, f64)],
    area: Rect,
    entries: &[DirEntry],
    results: &mut Vec<TreemapRect>,
) {
    if items.is_empty() || area.width() <= 0.0 || area.height() <= 0.0 {
        return;
    }

    if items.len() == 1 {
        let (idx, _) = items[0];
        let entry = &entries[idx];
        results.push(TreemapRect {
            rect: area,
            child_index: idx,
            label: entry.name.clone(),
            size: entry.size,
            is_dir: entry.is_dir,
            extension: entry.extension.clone(),
            size_known: entry.size_known,
        });
        return;
    }

    let total_area = area.width() as f64 * area.height() as f64;
    let total_fraction: f64 = items.iter().map(|(_, f)| f).sum();

    let short_side = area.width().min(area.height()) as f64;

    let mut best_row_len = 1;
    let mut best_worst_ratio = f64::MAX;

    for row_len in 1..=items.len() {
        let row_fraction: f64 = items[..row_len].iter().map(|(_, f)| f).sum();
        let row_area = total_area * row_fraction / total_fraction;
        let row_length = row_area / short_side;

        let mut worst_ratio = 0.0f64;
        for &(_, frac) in &items[..row_len] {
            let item_area = total_area * frac / total_fraction;
            let item_length = item_area / row_length;
            let ratio = (row_length / item_length).max(item_length / row_length);
            worst_ratio = worst_ratio.max(ratio);
        }

        if worst_ratio <= best_worst_ratio {
            best_worst_ratio = worst_ratio;
            best_row_len = row_len;
        } else {
            break;
        }
    }

    let row_fraction: f64 = items[..best_row_len].iter().map(|(_, f)| f).sum();
    let row_ratio = row_fraction / total_fraction;

    let horizontal = area.width() >= area.height();
    let mut offset = 0.0f32;

    for &(idx, frac) in &items[..best_row_len] {
        let item_ratio = (frac / row_fraction) as f32;
        let entry = &entries[idx];

        let rect = if horizontal {
            let strip_w = area.width() * row_ratio as f32;
            let item_h = area.height() * item_ratio;
            Rect::from_min_size(
                pos2(area.min.x, area.min.y + offset),
                vec2(strip_w, item_h),
            )
        } else {
            let strip_h = area.height() * row_ratio as f32;
            let item_w = area.width() * item_ratio;
            Rect::from_min_size(
                pos2(area.min.x + offset, area.min.y),
                vec2(item_w, strip_h),
            )
        };

        offset += if horizontal { rect.height() } else { rect.width() };

        results.push(TreemapRect {
            rect,
            child_index: idx,
            label: entry.name.clone(),
            size: entry.size,
            is_dir: entry.is_dir,
            extension: entry.extension.clone(),
            size_known: entry.size_known,
        });
    }

    if best_row_len < items.len() {
        let remaining = if horizontal {
            let strip_w = area.width() * row_ratio as f32;
            Rect::from_min_max(pos2(area.min.x + strip_w, area.min.y), area.max)
        } else {
            let strip_h = area.height() * row_ratio as f32;
            Rect::from_min_max(pos2(area.min.x, area.min.y + strip_h), area.max)
        };
        squarify(&items[best_row_len..], remaining, entries, results);
    }
}

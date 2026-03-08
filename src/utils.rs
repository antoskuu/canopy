use eframe::egui::Color32;

pub fn human_readable_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut size = bytes as f64;
    for unit in UNITS {
        if size < 1024.0 {
            return if size.fract() < 0.05 {
                format!("{:.0} {}", size, unit)
            } else {
                format!("{:.1} {}", size, unit)
            };
        }
        size /= 1024.0;
    }
    format!("{:.1} PB", size)
}

fn hash_str(s: &str) -> u32 {
    s.bytes()
        .fold(5381u32, |h, b| h.wrapping_mul(33).wrapping_add(b as u32))
}

// Nothing-inspired neon palette on dark background
const NEON_PALETTE: &[(u8, u8, u8)] = &[
    (255, 60, 56),   // nothing red
    (0, 220, 180),   // cyan/teal
    (255, 180, 0),   // amber
    (120, 200, 255), // ice blue
    (200, 80, 255),  // purple
    (255, 100, 160), // pink
    (80, 255, 120),  // neon green
    (255, 130, 60),  // orange
    (100, 140, 255), // periwinkle
    (220, 220, 220), // silver
    (255, 220, 100), // warm yellow
    (0, 180, 220),   // deep cyan
];

pub fn color_for_file(extension: &Option<String>) -> Color32 {
    let idx = match extension {
        Some(ext) => hash_str(ext) as usize % NEON_PALETTE.len(),
        None => 9, // silver for unknown
    };
    let (r, g, b) = NEON_PALETTE[idx];
    Color32::from_rgb(r, g, b)
}

pub fn color_for_dir(index: usize) -> Color32 {
    let idx = index % NEON_PALETTE.len();
    let (r, g, b) = NEON_PALETTE[idx];
    // Directories are slightly dimmed
    Color32::from_rgb(
        (r as f32 * 0.7) as u8,
        (g as f32 * 0.7) as u8,
        (b as f32 * 0.7) as u8,
    )
}

// Dark theme background colors
pub const BG_DARK: Color32 = Color32::from_rgb(18, 18, 18);
pub const BG_PANEL: Color32 = Color32::from_rgb(28, 28, 28);
pub const BG_ELEVATED: Color32 = Color32::from_rgb(38, 38, 38);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(230, 230, 230);
pub const TEXT_DIM: Color32 = Color32::from_rgb(140, 140, 140);
pub const ACCENT: Color32 = Color32::from_rgb(255, 60, 56); // nothing red
pub const BORDER: Color32 = Color32::from_rgb(55, 55, 55);

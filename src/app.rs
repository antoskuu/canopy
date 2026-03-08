use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use eframe::egui;
use eframe::egui::{Color32, FontId, Key, RichText, Stroke, Visuals};

use crate::render::draw_treemap;
use crate::scanner::{
    cache_age, disk_free_space, load_cache, save_cache, scan_shallow, start_size_computation,
    DirView, SizeUpdate,
};
use crate::treemap::TreemapRect;
use crate::utils::{
    human_readable_size, ACCENT, BG_DARK, BG_ELEVATED, BG_PANEL, BORDER, TEXT_DIM, TEXT_PRIMARY,
};

const CACHE_COLOR: Color32 = Color32::from_rgb(0, 220, 180);

pub struct StorageApp {
    current: Option<DirView>,
    history: Vec<DirView>,
    size_rx: Option<mpsc::Receiver<SizeUpdate>>,
    cancel_token: Option<Arc<AtomicBool>>,
    computing_sizes: bool,
    from_cache: bool,
    cached_layout: Option<(egui::Rect, Vec<TreemapRect>)>,
    hovered_info: Option<(String, u64)>,
    disk_info: Option<(u64, u64)>,
    error: Option<String>,
    initial_path: Option<PathBuf>,
    theme_applied: bool,
}

impl StorageApp {
    pub fn new(initial_path: Option<PathBuf>) -> Self {
        Self {
            current: None,
            history: Vec::new(),
            size_rx: None,
            cancel_token: None,
            computing_sizes: false,
            from_cache: false,
            cached_layout: None,
            hovered_info: None,
            disk_info: None,
            error: None,
            initial_path,
            theme_applied: false,
        }
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        if self.theme_applied {
            return;
        }
        self.theme_applied = true;

        let mut visuals = Visuals::dark();
        visuals.panel_fill = BG_PANEL;
        visuals.window_fill = BG_PANEL;
        visuals.extreme_bg_color = BG_DARK;
        visuals.faint_bg_color = BG_ELEVATED;
        visuals.override_text_color = Some(TEXT_PRIMARY);

        visuals.widgets.noninteractive.bg_fill = BG_PANEL;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM);
        visuals.widgets.inactive.bg_fill = BG_ELEVATED;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(50, 50, 50);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, ACCENT);
        visuals.widgets.active.bg_fill = Color32::from_rgb(60, 60, 60);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, ACCENT);

        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.item_spacing = egui::vec2(10.0, 6.0);
        ctx.set_style(style);
    }

    fn cancel_background(&mut self) {
        if let Some(token) = self.cancel_token.take() {
            token.store(true, Ordering::Relaxed);
        }
        self.size_rx = None;
        self.computing_sizes = false;
    }

    fn open_path(&mut self, path: PathBuf) {
        self.cancel_background();
        self.history.clear();
        self.cached_layout = None;
        self.error = None;
        self.from_cache = false;
        self.disk_info = disk_free_space(&path);

        // Try cache first
        if let Some(cached) = load_cache(&path) {
            let all_known = cached.entries.iter().all(|e| e.size_known);
            self.from_cache = true;
            self.current = Some(cached);
            if !all_known {
                self.start_background_sizes();
            }
            return;
        }

        match scan_shallow(&path) {
            Ok(view) => {
                self.current = Some(view);
                self.start_background_sizes();
            }
            Err(e) => {
                self.error = Some(e);
            }
        }
    }

    fn force_rescan(&mut self, path: PathBuf) {
        self.cancel_background();
        self.history.clear();
        self.cached_layout = None;
        self.error = None;
        self.from_cache = false;
        self.disk_info = disk_free_space(&path);

        match scan_shallow(&path) {
            Ok(view) => {
                self.current = Some(view);
                self.start_background_sizes();
            }
            Err(e) => {
                self.error = Some(e);
            }
        }
    }

    fn start_background_sizes(&mut self) {
        if let Some(view) = &self.current {
            let has_dirs = view.entries.iter().any(|e| e.is_dir && !e.size_known);
            if has_dirs {
                let (rx, token) = start_size_computation(view);
                self.size_rx = Some(rx);
                self.cancel_token = Some(token);
                self.computing_sizes = true;
            }
        }
    }

    fn navigate_into(&mut self, child_index: usize) {
        let Some(current) = self.current.take() else {
            return;
        };
        let path = current.entries[child_index].path.clone();
        self.cancel_background();
        self.history.push(current);
        self.cached_layout = None;

        // Try cache for this subdir
        if let Some(cached) = load_cache(&path) {
            let all_known = cached.entries.iter().all(|e| e.size_known);
            self.current = Some(cached);
            if !all_known {
                self.start_background_sizes();
            }
            return;
        }

        match scan_shallow(&path) {
            Ok(view) => {
                self.current = Some(view);
                self.start_background_sizes();
            }
            Err(e) => {
                self.error = Some(e);
                // Restore current from history
                self.current = self.history.pop();
            }
        }
    }

    fn navigate_back_to(&mut self, level: usize) {
        if level < self.history.len() {
            self.cancel_background();
            self.history.truncate(level + 1);
            self.current = self.history.pop();
            self.cached_layout = None;
            self.start_background_sizes();
        }
    }

    fn breadcrumb_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .history
            .iter()
            .map(|v| {
                v.path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| v.path.to_string_lossy().to_string())
            })
            .collect();
        if let Some(current) = &self.current {
            names.push(
                current
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| current.path.to_string_lossy().to_string()),
            );
        }
        names
    }

    fn drain_size_updates(&mut self) {
        let Some(rx) = &self.size_rx else { return };

        let mut updated = false;
        while let Ok(update) = rx.try_recv() {
            if let Some(view) = &mut self.current {
                if let Some(entry) = view.entries.iter_mut().find(|e| e.path == update.path) {
                    entry.size = update.size;
                    entry.size_known = true;
                    updated = true;
                }
            }
        }

        if updated {
            // Re-sort by size desc
            if let Some(view) = &mut self.current {
                view.entries.sort_by(|a, b| b.size.cmp(&a.size));
            }
            self.cached_layout = None;
        }

        // Check if all sizes are now known
        if let Some(view) = &self.current {
            let all_known = view.entries.iter().all(|e| e.size_known);
            if all_known && self.computing_sizes {
                self.computing_sizes = false;
                self.size_rx = None;
                save_cache(view);
            }
        }
    }
}

fn format_age(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

impl eframe::App for StorageApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_theme(ctx);

        if let Some(path) = self.initial_path.take() {
            self.open_path(path);
        }

        // Keyboard navigation: Escape/Backspace to go up one level
        if !self.history.is_empty() {
            let input = ctx.input(|i| {
                i.key_pressed(Key::Escape) || i.key_pressed(Key::Backspace)
            });
            if input {
                let level = self.history.len() - 1;
                self.navigate_back_to(level);
            }
        }

        // Drain background size updates
        self.drain_size_updates();

        if self.computing_sizes {
            ctx.request_repaint();
        }

        // Top bar
        egui::TopBottomPanel::top("toolbar")
            .frame(
                egui::Frame::new()
                    .fill(BG_PANEL)
                    .inner_margin(egui::Margin::same(10)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let btn = egui::Button::new(
                        RichText::new("OPEN")
                            .font(FontId::monospace(12.0))
                            .color(ACCENT),
                    )
                    .stroke(Stroke::new(1.0, ACCENT))
                    .corner_radius(3.0);

                    if ui.add(btn).clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.open_path(path);
                        }
                    }

                    ui.add_space(8.0);

                    if self.computing_sizes {
                        ui.spinner();
                        ui.label(
                            RichText::new("computing sizes...")
                                .font(FontId::monospace(11.0))
                                .color(TEXT_DIM),
                        );
                    }

                    if let Some(view) = &self.current {
                        let total_size: u64 = view.entries.iter().map(|e| e.size).sum();
                        ui.label(
                            RichText::new(format!(
                                "{} // {}",
                                view.path.display(),
                                human_readable_size(total_size)
                            ))
                            .font(FontId::monospace(11.0))
                            .color(TEXT_PRIMARY),
                        );

                        ui.add_space(8.0);

                        if self.from_cache && self.history.is_empty() {
                            let age_text = cache_age(&view.path)
                                .map(format_age)
                                .unwrap_or_default();
                            ui.label(
                                RichText::new(format!("CACHED {}", age_text))
                                    .font(FontId::monospace(9.0))
                                    .color(CACHE_COLOR),
                            );
                            ui.add_space(4.0);
                        }

                        let rescan = egui::Button::new(
                            RichText::new("RESCAN")
                                .font(FontId::monospace(10.0))
                                .color(TEXT_DIM),
                        )
                        .stroke(Stroke::new(1.0, BORDER))
                        .corner_radius(3.0);

                        if ui.add(rescan).clicked() {
                            // Rescan the root path (first in history, or current)
                            let root_path = self
                                .history
                                .first()
                                .map(|v| v.path.clone())
                                .unwrap_or_else(|| view.path.clone());
                            self.force_rescan(root_path);
                        }
                    }

                    if let Some(err) = &self.error {
                        ui.label(
                            RichText::new(err)
                                .font(FontId::monospace(11.0))
                                .color(ACCENT),
                        );
                    }
                });

                // Breadcrumb
                if !self.history.is_empty() {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let names = self.breadcrumb_names();
                        for (i, name) in names.iter().enumerate() {
                            if i > 0 {
                                ui.label(
                                    RichText::new("/")
                                        .font(FontId::monospace(11.0))
                                        .color(BORDER),
                                );
                            }
                            if i < names.len() - 1 {
                                // Clickable: navigate back to this level
                                let link = ui.add(
                                    egui::Label::new(
                                        RichText::new(name)
                                            .font(FontId::monospace(11.0))
                                            .color(TEXT_DIM),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                if link.clicked() {
                                    self.navigate_back_to(i);
                                }
                                if link.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                            } else {
                                ui.label(
                                    RichText::new(name)
                                        .font(FontId::monospace(11.0))
                                        .color(TEXT_PRIMARY),
                                );
                            }
                        }
                    });
                }
            });

        // Status bar
        egui::TopBottomPanel::bottom("status")
            .exact_height(28.0)
            .frame(
                egui::Frame::new()
                    .fill(BG_PANEL)
                    .inner_margin(egui::Margin::same(8)),
            )
            .show(ctx, |ui| {
                if let Some((path, size)) = &self.hovered_info {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(path)
                                .font(FontId::monospace(10.5))
                                .color(TEXT_PRIMARY),
                        );
                        ui.label(
                            RichText::new(format!("// {}", human_readable_size(*size)))
                                .font(FontId::monospace(10.5))
                                .color(ACCENT),
                        );
                    });
                } else {
                    ui.label(
                        RichText::new("hover to inspect")
                            .font(FontId::monospace(10.0))
                            .color(TEXT_DIM),
                    );
                }
            });

        // Main treemap area
        let mut clicked_dir = None;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(BG_DARK)
                    .inner_margin(egui::Margin::same(4)),
            )
            .show(ctx, |ui| {
                if let Some(ref view) = self.current {
                    if view.entries.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                RichText::new("[ empty ]")
                                    .font(FontId::monospace(14.0))
                                    .color(TEXT_DIM),
                            );
                        });
                    } else {
                        let result =
                            draw_treemap(ui, view, &mut self.cached_layout, self.disk_info);
                        self.hovered_info = result.hovered;
                        clicked_dir = result.clicked_dir;
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        let avail = ui.available_height();
                        ui.add_space(avail * 0.35);

                        ui.label(
                            RichText::new("CANOPY")
                                .font(FontId::monospace(28.0))
                                .color(ACCENT),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("disk space visualizer")
                                .font(FontId::monospace(12.0))
                                .color(TEXT_DIM),
                        );
                        ui.add_space(24.0);

                        let open_btn = egui::Button::new(
                            RichText::new("OPEN FOLDER")
                                .font(FontId::monospace(13.0))
                                .color(ACCENT),
                        )
                        .stroke(Stroke::new(1.0, ACCENT))
                        .corner_radius(4.0);

                        if ui.add(open_btn).clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                self.open_path(path);
                            }
                        }

                        ui.add_space(16.0);
                        ui.label(
                            RichText::new("or pass a path as argument")
                                .font(FontId::monospace(10.0))
                                .color(Color32::from_rgb(80, 80, 80)),
                        );
                    });
                }
            });

        if let Some(idx) = clicked_dir {
            self.navigate_into(idx);
        }
    }
}

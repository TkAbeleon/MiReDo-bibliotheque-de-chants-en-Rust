//! Theme configuration for the MiReDo interface.
//!
//! This small module isolates the global egui style from the screen logic so the app remains
//! easier to tune when adjusting light/dark palettes or widget density.

use eframe::egui::{self, Stroke};

use super::{MiReDoApp, ThemeMode};

/// Applies the theme values from the active palette to the current egui context.
pub fn apply_theme(app: &MiReDoApp, context: &egui::Context) {
    let palette = app.palette();
    let dark = matches!(app.theme_mode, ThemeMode::Dark)
        || (app.theme_mode == ThemeMode::System && app.system_dark);

    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.panel_fill = palette.get("background");
    visuals.window_fill = palette.get("surface_elevated");
    visuals.extreme_bg_color = palette.get("surface");
    visuals.faint_bg_color = palette.get("surface_hover");
    visuals.selection.bg_fill = palette.get("surface_selected");
    visuals.selection.stroke = Stroke::new(1.0_f32, palette.get("focus"));
    visuals.widgets.noninteractive.bg_fill = palette.get("surface");
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, palette.get("text_primary"));
    visuals.widgets.inactive.bg_fill = palette.get("surface");
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, palette.get("text_secondary"));
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, palette.get("border_subtle"));
    visuals.widgets.hovered.bg_fill = palette.get("surface_hover");
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, palette.get("text_primary"));
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, palette.get("border_strong"));
    visuals.widgets.active.bg_fill = palette.get("surface_selected");
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, palette.get("text_primary"));
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, palette.get("focus"));
    visuals.hyperlink_color = palette.get("accent");
    visuals.override_text_color = Some(palette.get("text_primary"));
    visuals.window_stroke = Stroke::new(1.0_f32, palette.get("border_subtle"));
    visuals.widgets.noninteractive.corner_radius = 8.into();
    visuals.widgets.inactive.corner_radius = 8.into();
    visuals.widgets.hovered.corner_radius = 8.into();
    visuals.widgets.active.corner_radius = 8.into();

    context.set_visuals(visuals);

    let mut style = context.style().as_ref().clone();
    style.spacing.item_spacing = egui::vec2(12.0, 10.0);
    style.spacing.button_padding = egui::vec2(11.0, 7.0);
    style.spacing.interact_size = egui::vec2(34.0, 34.0);
    style.spacing.window_margin = egui::Margin::same(20);
    style.spacing.menu_margin = egui::Margin::same(8);
    context.set_style(style);
}

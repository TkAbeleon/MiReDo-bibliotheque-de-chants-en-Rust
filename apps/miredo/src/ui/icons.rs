//! Reusable graphical primitives for the MiReDo interface.
//!
//! This file centralizes the Lucide-based icon rendering and the compact toolbar-button
//! styling so the rest of the UI code stays focused on screens and interactions.

use std::sync::Arc;

use eframe::egui::{self, Stroke, Vec2};

use super::AppIcon;

/// Renders a Lucide SVG at a precise rect using the current icon color.
pub fn paint_app_icon(ui: &mut egui::Ui, rect: egui::Rect, icon: AppIcon, color: egui::Color32) {
    let lucide = match icon {
        AppIcon::Home => egui_lucide::Lucide::House,
        AppIcon::Library => egui_lucide::Lucide::Library,
        AppIcon::Browse => egui_lucide::Lucide::BookOpenText,
        AppIcon::Favorite => egui_lucide::Lucide::Heart,
        AppIcon::Playlist => egui_lucide::Lucide::ListMusic,
        AppIcon::Add => egui_lucide::Lucide::Plus,
        AppIcon::Edit => egui_lucide::Lucide::Pencil,
        AppIcon::Delete => egui_lucide::Lucide::Trash2,
        AppIcon::Help => egui_lucide::Lucide::MessageCircleQuestionMark,
        AppIcon::About => egui_lucide::Lucide::Info,
        AppIcon::Settings => egui_lucide::Lucide::Settings2,
        AppIcon::Menu => egui_lucide::Lucide::Menu,
        AppIcon::Search => egui_lucide::Lucide::Search,
        AppIcon::Previous => egui_lucide::Lucide::ChevronLeft,
        AppIcon::Next => egui_lucide::Lucide::ChevronRight,
        AppIcon::Close => egui_lucide::Lucide::X,
        AppIcon::ZoomIn => egui_lucide::Lucide::ZoomIn,
        AppIcon::ZoomOut => egui_lucide::Lucide::ZoomOut,
        AppIcon::Fullscreen => egui_lucide::Lucide::Maximize,
    };

    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"#{:02x}{:02x}{:02x}\" stroke-width=\"1.75\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{}</svg>",
        color.r(),
        color.g(),
        color.b(),
        lucide.inner(),
    );
    let uri = format!(
        "bytes://miredo-lucide/{}-{:02x}{:02x}{:02x}.svg",
        lucide.name(),
        color.r(),
        color.g(),
        color.b(),
    );
    let source = egui::ImageSource::Bytes {
        uri: uri.into(),
        bytes: egui::load::Bytes::Shared(Arc::from(svg.into_bytes())),
    };

    ui.put(
        rect,
        egui::Image::new(source)
            .fit_to_exact_size(rect.size())
            .show_loading_spinner(false)
            .alt_text(lucide.name()),
    );
}

/// Small button wrapper used by the list actions and navigation controls.
pub fn icon_button(
    ui: &mut egui::Ui,
    icon: AppIcon,
    color: egui::Color32,
    tooltip: String,
) -> egui::Response {
    icon_button_enabled(ui, icon, color, tooltip, true)
}

/// Generic clickable icon button with support for disabled states.
pub fn icon_button_enabled(
    ui: &mut egui::Ui,
    icon: AppIcon,
    color: egui::Color32,
    tooltip: String,
    enabled: bool,
) -> egui::Response {
    let response = ui
        .add_enabled_ui(enabled, |ui| {
            ui.add_sized(
                Vec2::splat(36.0),
                egui::Button::new("")
                    .fill(egui::Color32::TRANSPARENT)
                    .stroke(Stroke::NONE),
            )
        })
        .inner;
    paint_app_icon(ui, response.rect, icon, color);
    response.on_hover_text(tooltip)
}

/// Compact toolbar variant used in the PDF reader and other dense panels.
pub fn pdf_toolbar_icon_button(
    ui: &mut egui::Ui,
    icon: AppIcon,
    color: egui::Color32,
    tooltip: String,
    enabled: bool,
) -> egui::Response {
    let response = ui
        .add_enabled_ui(enabled, |ui| {
            ui.add_sized(
                Vec2::splat(28.0),
                egui::Button::new("")
                    .fill(egui::Color32::TRANSPARENT)
                    .stroke(Stroke::NONE),
            )
        })
        .inner;
    let icon_rect = response.rect.shrink(6.0);
    paint_app_icon(ui, icon_rect, icon, color);
    response.on_hover_text(tooltip)
}

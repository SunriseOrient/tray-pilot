//! 托盘弹出菜单：常用动作列表 + 编辑 / 退出。
//!
//! 样式参考 Windows 原生右键菜单：单行条目、组间分隔线、悬停整行高亮。

use std::time::{Duration, Instant};

use eframe::egui;

use crate::app;
use crate::theme;
use crate::ui::{menu_item, menu_separator};

/// 绘制弹出菜单。运行在独立的「弹出菜单」视口里。
pub fn show(ui: &mut egui::Ui) {
    let pal = theme::palette();

    let full = ui.max_rect();
    let margin = app::WINDOW_MARGIN;
    let panel = egui::Rect::from_min_max(
        full.min + egui::vec2(margin, margin),
        full.max - egui::vec2(margin, margin),
    );

    // 面板：圆角 + 轻阴影，四周只留很窄的透明边。
    let frame = egui::Frame::new()
        .fill(pal.bg)
        .stroke(egui::Stroke::new(1.0, pal.border))
        .corner_radius(egui::CornerRadius::from(10u8))
        .shadow(theme::menu_shadow());
    ui.painter().add(frame.paint(panel));

    let content = panel.shrink(app::MENU_PADDING);
    ui.scope_builder(egui::UiBuilder::new().max_rect(content), |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let width = ui.available_width();

        let actions = {
            let sh = app::shared();
            sh.config.actions.clone()
        };

        // 动作组
        let mut handled = false;
        for action in &actions {
            let response = menu_item(ui, &action.name, &action.id, width);
            let clicked = response.clicked();
            if !action.description.is_empty() {
                response.on_hover_text(&action.description);
            }
            if clicked {
                app::run_action(&action.id);
                handled = true;
                break;
            }
        }

        if !handled {
            if !actions.is_empty() {
                menu_separator(ui, width);
            }

            // 编辑 / 退出
            if menu_item(ui, "编辑", "edit", width).clicked() {
                app::close_popup();
                app::open_editor(ui.ctx());
            } else if menu_item(ui, "退出", "quit", width).clicked() {
                app::quit(ui.ctx());
            }
        }
    });

    // 点击面板以外的区域 / 按 Esc → 关闭菜单。
    let pressed = ui.ctx().input(|i| i.pointer.any_pressed());
    let pointer = ui.ctx().input(|i| i.pointer.latest_pos());
    let outside = pressed && pointer.is_some_and(|p| !panel.contains(p));
    let escape = ui.ctx().input(|i| i.key_pressed(egui::Key::Escape));
    if outside || escape {
        app::close_popup();
        return;
    }

    // 失焦自动关闭（首次获得焦点后生效，避免刚弹出就被误关）。
    let focused = ui.ctx().input(|i| i.viewport().focused.unwrap_or(false));
    let now = Instant::now();
    let mut should_close = false;
    {
        let mut sh = app::shared();
        if let Some(popup) = sh.popup.as_mut() {
            if focused {
                popup.focused_once = true;
            } else if popup.focused_once
                && now.saturating_duration_since(popup.opened_at) > Duration::from_millis(300)
            {
                should_close = true;
            }
        }
    }
    if should_close {
        app::close_popup();
    }
}
